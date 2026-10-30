use crate::{prelude::*, root};
use tokio::process::Command;
static NVIDIA_PREFIXES: std::sync::LazyLock<std::collections::HashMap<&str, &str>> =
    std::sync::LazyLock::new(|| {
        [
            // List of chipset prefixes with its corresponding last supported driver version
            // if it's not in the list, it's probably supported by the latest driver
            // but... if it's really really old, then you're out of luck
            // We're gonna be supporting GPUs from the 8000 series and up
            ("NV", "unsupported"),
            ("MCP", "unsupported"),
            ("G7", "unsupported"),
            ("G8", "340xx"),
            // wtf this goes from like 8000 to 100 series
            ("G9", "340xx"),
            // finally, a sane naming scheme
            // Tesla GPUs
            ("GT", "340xx"),
            // Fermi GPUs, in case you like burning your house down
            ("GF", "390xx"),
            // Kepler GPUs
            // now we're finally up to the modern era
            ("GK", "470xx"),
            // The rest should be supported by the latest driver, at least as of
            // late 2023
        ]
        .into_iter()
        .collect()
    });

#[derive(Clone, Copy, Debug, Default)]
pub struct DriversCodecs;
#[derive(Clone, Copy, Debug, Default)]
pub struct Drivers;
#[derive(Clone, Copy, Debug, Default)]
pub struct Codecs;
impl super::Step for DriversCodecs {
    #[tracing::instrument(skip(sender))]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        _cfg: &crate::cfg::Config,
        sender: &C,
    ) -> Res<()> {
        if !settings.install_codecs_drivers || settings.nointernet {
            return Ok(());
        }
        match Drivers::setup_nvidia().await {
            Ok(true) => sender.send(crate::InstallMsg::NeedsReboot),
            Ok(false) => {}
            Err(e) => tracing::warn!(?e, "cannot setup nvidia drivers"),
        }
        if let Err(e) = Drivers::setup_broadcom().await {
            tracing::warn!(?e, "cannot setup broadcom drivers");
        }
        // FIXME: refactor this to somewhere else not in this file
        // also this is done inside `_05_dnfdownloadapps.rs`
        // Codecs::install_codecs().await?;
        Ok(())
    }
}

/// hardware that needs extra drivers, as installed by [`DriversCodecs`]
#[derive(Clone, Copy, Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Hardware {
    pub nvidia: bool,
    pub broadcom_wifi: bool,
    pub broadcom_bluetooth: bool,
}

/// detect hardware that [`DriversCodecs`] would install drivers for
pub async fn detect_hardware() -> Hardware {
    let (nvidia, broadcom_wifi, broadcom_bluetooth) = tokio::join!(
        Drivers::check_nvidia_gpu(),
        Drivers::check_boardcom_wifi(),
        Drivers::check_boardcom_bluetooth(),
    );
    Hardware { nvidia, broadcom_wifi, broadcom_bluetooth }
}

// NOTE: added during `_05_dnfdownloadapps.rs`
impl Codecs {
    pub const fn codecs() -> &'static [&'static str] {
        &["@multimedia"]
    }
}

impl Drivers {
    /// Returns the latest supported driver for the given chipset
    fn get_nvidia_driver(chipset: &str) -> &str {
        for (prefix, driver) in &*NVIDIA_PREFIXES {
            if chipset.starts_with(prefix) {
                return driver;
            }
        }
        "latest"
    }

    async fn check_nvidia_gpu() -> bool {
        Command::new("sh")
            .args(["-c", "lspci | grep -q -i NVIDIA"])
            .status()
            .await
            .is_ok_and(|s| s.success())
    }

    async fn get_nvidia_chipset() -> Res<String> {
        Ok(String::from_utf8_lossy(Command::new("sh").arg("-c").arg("lspci | grep -i NVIDIA | head -n 1 | cut -d ':' -f 3 | cut -d '[' -f 1 | sed -e 's/^[[:space:]]*//'").stdout(std::process::Stdio::piped()).output().await.map_err(|e| Err::FailToRunProgram("sh", e)).wrap_msg("cannot detect nvidia chipset")?.stdout.rsplit(|&c| c == b' ').next().expect("malformatted output from shell")).to_string())
    }

    ///    Returns a list of Nvidia packages to install
    async fn list_nvidia_packages() -> Res<Vec<String>> {
        let mut pkgs = vec!["nvidia-gpu-firmware".into(), "libva-nvidia-driver".into()];
        let chipset = Self::get_nvidia_chipset().await?;
        match Self::get_nvidia_driver(&chipset) {
            "unsupported" => (),
            // TODO: nvidia-driver
            "latest" => pkgs.extend_from_slice(&[
                "akmod-nvidia".into(),
                "xorg-x11-drv-nvidia".into(),
                "xorg-x11-drv-nvidia-cuda".into(),
            ]),
            v => pkgs.extend_from_slice(&[
                format!("akmod-nvidia-{v}"),
                format!("xorg-x11-drv-nvidia-{v}"),
                format!("xorg-x11-drv-nvidia-{v}-cuda"),
            ]),
        }
        Ok(pkgs)
    }

    async fn is_ostree() -> bool {
        tokio::fs::try_exists("/ostree").await.unwrap_or_default()
    }

    async fn setup_nvidia_ostree() -> Res<()> {
        let pkgs = Self::list_nvidia_packages().await?;
        let mut args = vec!["install", "-y"];
        args.extend(pkgs.iter().map(String::as_str));
        root("rpm-ostree", &args).await.wrap_message(|_| format!("pkgs={pkgs:?}"))?;
        root(
            "rpm-ostree",
            &[
                "kargs",
                "--append=rd.driver.blacklist=nouveau",
                "--append=modprobe.blacklist=nouveau",
                "--append=nvidia-drm.modeset=1",
                "initcall_blacklist=simpledrm_platform_driver_init",
            ],
        )
        .await?;
        Ok(())
    }

    /// the kernel that boots next, which may be newer than the running one after the updates
    async fn next_kernel() -> Res<String> {
        let out = Command::new("grubby").arg("--default-kernel").output().await;
        if let Ok(out) = &out
            && out.status.success()
            && let Some(kver) =
                String::from_utf8_lossy(&out.stdout).trim().strip_prefix("/boot/vmlinuz-")
        {
            return Ok(kver.to_owned());
        }
        tracing::warn!(?out, "cannot get the default kernel from grubby, using the running one");
        let kver = tokio::fs::read_to_string("/proc/sys/kernel/osrelease")
            .await
            .map_err(|e| Err::Message(format!("cannot read kernel version: {e}")))?;
        Ok(kver.trim().to_owned())
    }

    /// Waits for akmods to build the NVIDIA kernel module, then checks that it's there.
    ///
    /// The akmod's `%posttrans` starts a build in the background, a second `akmods` waits on its
    /// lock and skips kmods that are already up to date, so this also builds it if that didn't.
    async fn nvidia_kmod_built() -> Res<bool> {
        let kver = Self::next_kernel().await?;
        tracing::info!(kver, "Waiting for akmods to build the Nvidia kernel module");
        if let Err(e) = root("akmods", ["--kernels", &kver]).await {
            tracing::warn!(?e, "akmods failed");
        }
        Ok(Command::new("modinfo")
            .args(["-k", &kver, "-n", "nvidia"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .is_ok_and(|s| s.success()))
    }

    /// Returns whether the kernel module is ready, so a reboot would load it.
    async fn setup_nvidia() -> Res<bool> {
        tracing::info!("Setting up Nvidia drivers");
        let primary_gpu = std::env::var("STELLAR_OPTION").is_ok_and(|x| x == "1");
        if !Self::check_nvidia_gpu().await {
            tracing::info!("No Nvidia GPU detected");
            return Ok(false);
        }
        if Self::is_ostree().await {
            tracing::debug!("ostree detected");
            // the kmod is built inside the rpm-ostree transaction, which needs a reboot anyway
            Self::setup_nvidia_ostree().await?;
            return Ok(true);
        }

        let pkgs = Self::list_nvidia_packages().await?;
        let mut args = vec!["in", "-y", "--allowerasing", "--best"];
        args.extend(pkgs.iter().map(String::as_str));
        root("dnf", &args).await.wrap_message(|_| format!("pkgs={pkgs:?}"))?;

        if primary_gpu {
            root(
                "sh",
                &[r#"
                    cp -p /usr/share/X11/xorg.conf.d/nvidia.conf /etc/X11/xorg.conf.d/nvidia.conf
                    sed -i '10i\\\tOption "PrimaryGPU" "yes"' /etc/X11/xorg.conf.d/nvidia.conf
                "#],
            )
            .await
            .wrap_msg("cannot set nvidia as primary gpu")?;
        }

        let built = Self::nvidia_kmod_built().await?;
        if !built {
            tracing::warn!("Nvidia kernel module isn't built, not asking for a reboot");
        }
        Ok(built)
    }

    async fn check_boardcom_wifi() -> bool {
        Command::new("sh")
            .arg("-c")
            .arg("lspci | grep -i Network | grep -q -i Broadcom")
            .status()
            .await
            .is_ok_and(|s| s.success())
    }
    async fn check_boardcom_bluetooth() -> bool {
        Command::new("sh")
            .arg("-c")
            .arg("lspci | grep -i Bluetooth | grep -q -i Broadcom")
            .status()
            .await
            .is_ok_and(|s| s.success())
    }

    async fn setup_broadcom() -> Res<()> {
        if Self::check_boardcom_wifi().await {
            tracing::info!("Setting up broadcom wifi drivers");
            root("dnf", &["in", "-y", "broadcom-wl", "akmod-wl"])
                .await
                .wrap_msg("fail to install broadcom wifi drivers")?;
        }
        if Self::check_boardcom_bluetooth().await {
            tracing::info!("Setting up broadcom bluetooth drivers");
            root("dnf", &["in", "-y", "broadcom-bt-firmware"])
                .await
                .wrap_msg("fail to install broadcom bluetooth drivers")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_nvidia_driver() {
        assert_eq!(Drivers::get_nvidia_driver("NV34"), "unsupported");
        assert_eq!(Drivers::get_nvidia_driver("GK104"), "470xx");
        assert_eq!(Drivers::get_nvidia_driver("GP108"), "latest");
        assert_eq!(Drivers::get_nvidia_driver("GK208"), "470xx");
        assert_eq!(Drivers::get_nvidia_driver("GT218"), "340xx");
    }
}
