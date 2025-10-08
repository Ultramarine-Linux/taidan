use crate::{backend::root, prelude::*};
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

use async_process::Command;

#[derive(Clone, Copy, Debug, Default)]
pub struct DriversCodecs;
#[derive(Clone, Copy, Debug, Default)]
pub struct Drivers;
#[derive(Clone, Copy, Debug, Default)]
pub struct Codecs;
impl super::Step for DriversCodecs {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        if !settings.install_codecs_drivers || settings.nointernet {
            return Ok(());
        }
        if let Err(e) = Drivers::setup_nvidia().await {
            tracing::warn!(?e, "cannot setup nvidia drivers");
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

// NOTE: added during `_05_dnfdownloadapps.rs`
impl Codecs {
    pub const fn codecs() -> &'static [&'static str] {
        &[
            "ffmpeg",
            "gstreamer1",
            "gstreamer1-plugins-bad-free-libs",
            "gstreamer1-plugins-bad-free",
            "gstreamer1-plugins-bad-free-extras",
            // "gstreamer1-plugins-bad-free-freeworld",
            "gstreamer1-plugins-base",
            "gstreamer1-plugins-good",
            "gstreamer1-plugins-good-gtk",
            "gstreamer1-plugins-good-qt",
            "gstreamer1-plugins-good-qt6",
            "gstreamer1-plugins-ugly",
            "gstreamer1-plugins-ugly-free",
            "gstreamer1-plugin-libav",
            "gstreamer1-plugin-openh264",
            "lame",
            "libaacs",
            // "libavcodec",
            // "libdvdcss",
            "libvpx",
            "nv-codec-headers",
            "opus",
            "openh264",
            "pipewire-codec-aptx",
            "pipewire-gstreamer",
            "x264",
            "x265",
        ]
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

    async fn get_nvidia_chipset() -> color_eyre::Result<String> {
        String::from_utf8(Command::new("sh").arg("-c").arg("lspci | grep -i NVIDIA | head -n 1 | cut -d ':' -f 3 | cut -d '[' -f 1 | sed -e 's/^[[:space:]]*//'").stdout(std::process::Stdio::piped()).output().await.wrap_err("cannot detect nvidia chipset")?.stdout.rsplit(|&c| c == b' ').next().expect("malformatted output from shell").to_vec()).wrap_err("cannot convert shell output to utf8")
    }

    ///    Returns a list of Nvidia packages to install
    async fn list_nvidia_packages() -> color_eyre::Result<Vec<String>> {
        let mut pkgs = vec!["nvidia-gpu-firmware".into(), "libva-nvidia-driver".into()];
        let chipset = Self::get_nvidia_chipset().await?;
        match Self::get_nvidia_driver(&chipset) {
            "unsupported" => (),
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
        crate::a::exist("/ostree").await
    }

    async fn setup_nvidia_ostree() -> color_eyre::Result<()> {
        let pkgs = Self::list_nvidia_packages().await?;
        let mut args = vec!["install", "-y"];
        args.extend(pkgs.iter().map(String::as_str));
        root("rpm-ostree", &args)
            .await
            .with_note(|| format!("pkgs={pkgs:?}"))?;
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

    async fn setup_nvidia() -> color_eyre::Result<()> {
        tracing::info!("Setting up Nvidia drivers");
        let primary_gpu = std::env::var("STELLAR_OPTION").is_ok_and(|x| x == "1");
        if !Self::check_nvidia_gpu().await {
            tracing::info!("No Nvidia GPU detected");
            return Ok(());
        }
        if Self::is_ostree().await {
            tracing::debug!("ostree detected");
            return Self::setup_nvidia_ostree().await;
        }

        let pkgs = Self::list_nvidia_packages().await?;
        let mut args = vec!["in", "-y", "--allowerasing", "--best"];
        args.extend(pkgs.iter().map(String::as_str));
        root("dnf", &args)
            .await
            .with_note(|| format!("pkgs={pkgs:?}"))?;

        if primary_gpu {
            root(
                "sh",
                &[r#"
                    cp -p /usr/share/X11/xorg.conf.d/nvidia.conf /etc/X11/xorg.conf.d/nvidia.conf
                    sed -i '10i\\\tOption "PrimaryGPU" "yes"' /etc/X11/xorg.conf.d/nvidia.conf
                "#],
            )
            .await
            .wrap_err("cannot set nvidia as primary gpu")?;
        }

        Ok(())
    }

    async fn check_boardcom_wifi() -> bool {
        Command::new("sh")
            .arg("-c")
            .arg("lspci | grep -q -i Network | grep -q -i Broadcom")
            .status()
            .await
            .is_ok_and(|s| s.success())
    }
    async fn check_boardcom_bluetooth() -> bool {
        Command::new("sh")
            .arg("-c")
            .arg("lspci | grep -q -i Bluetooth| grep -q -i Broadcom")
            .status()
            .await
            .is_ok_and(|s| s.success())
    }

    async fn setup_broadcom() -> color_eyre::Result<()> {
        if Self::check_boardcom_wifi().await {
            tracing::info!("Setting up broadcom wifi drivers");
            root("dnf", &["in", "-y", "broadcom-wl", "akmod-wl"])
                .await
                .wrap_err("fail to install broadcom wifi drivers")?;
        }
        if Self::check_boardcom_bluetooth().await {
            tracing::info!("Setting up broadcom bluetooth drivers");
            root("dnf", &["in", "-y", "broadcom-bt-firmware"])
                .await
                .wrap_err("fail to install broadcom bluetooth drivers")?;
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
