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
            // 340xx isn't packaged by RPM Fusion anymore
            ("G8", "unsupported"),
            // wtf this goes from like 8000 to 100 series
            ("G9", "unsupported"),
            // finally, a sane naming scheme
            // Tesla GPUs
            ("GT", "unsupported"),
            // Fermi GPUs, in case you like burning your house down
            ("GF", "390xx"),
            // Kepler GPUs
            // now we're finally up to the modern era
            ("GK", "470xx"),
            // Maxwell, Pascal and Volta, dropped in the 590 driver
            ("GM", "580xx"),
            ("GP", "580xx"),
            ("GV", "580xx"),
            // Turing and newer should be supported by the latest driver, at least as of
            // late 2026
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
    #[tracing::instrument(skip(settings, sender))]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        _cfg: &crate::cfg::Config,
        sender: &C,
    ) -> Res<()> {
        if !settings.install_codecs_drivers || settings.nointernet {
            return Ok(());
        }
        let pci = pci_devices().await;
        let mut needs_reboot = false;
        match Drivers::setup_nvidia(&pci).await {
            Ok(built) => needs_reboot |= built,
            Err(e) => tracing::warn!(?e, "cannot setup nvidia drivers"),
        }
        match Drivers::setup_broadcom(&pci).await {
            Ok(built) => needs_reboot |= built,
            Err(e) => tracing::warn!(?e, "cannot setup broadcom drivers"),
        }
        if needs_reboot {
            sender.send(crate::InstallMsg::NeedsReboot);
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
    let pci = pci_devices().await;
    Hardware {
        nvidia: has_nvidia_gpu(&pci),
        broadcom_wifi: has_wl_only_wifi(&pci),
        broadcom_bluetooth: Drivers::check_boardcom_bluetooth().await,
    }
}

const PCI_VENDOR_NVIDIA: u16 = 0x10de;
const PCI_VENDOR_BROADCOM: u16 = 0x14e4;

/// Broadcom Wi-Fi chips that only `wl` (`broadcom-wl`) can drive on Fedora.
///
/// This is `wl`'s device list minus the chips `brcmsmac` handles (4727, 0576, 4353, 4357),
/// which work out of the box with linux-firmware. The b43 chips stay in, since Fedora doesn't
/// ship the proprietary b43 firmware they need. Newer chips are `brcmfmac`'s and aren't
/// supported by `wl` at all. See <https://wiki.debian.org/wl>.
const WL_ONLY_DEVICES: &[u16] = &[
    0x4311, 0x4312, // BCM4311
    0x4315, // BCM4312
    0x4328, 0x4329, 0x432a, // BCM4321
    0x432b, 0x432c, 0x432d, // BCM4322
    0x4358, // BCM43227
    0x4359, // BCM43228
    0x4365, // BCM43142
    0x4331, // BCM4331
    0x43b1, // BCM4352
    0x43a0, 0x4360, // BCM4360
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PciDevice {
    vendor: u16,
    device: u16,
    /// class, subclass and prog-if, e.g. `0x030000` for a VGA controller
    class: u32,
}

impl PciDevice {
    /// read a device from its directory in `/sys/bus/pci/devices`
    async fn read(dir: &std::path::Path) -> Option<Self> {
        let read = async |name| {
            let s = tokio::fs::read_to_string(dir.join(name)).await.ok()?;
            u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
        };
        Some(Self {
            vendor: read("vendor").await?.try_into().ok()?,
            device: read("device").await?.try_into().ok()?,
            class: read("class").await?,
        })
    }

    /// a display controller that the NVIDIA driver drives, i.e. VGA or 3D, but not e.g. the
    /// HDMI audio function or an nForce chipset
    const fn is_nvidia_gpu(self) -> bool {
        self.vendor == PCI_VENDOR_NVIDIA && matches!(self.class >> 8, 0x0300 | 0x0302)
    }
}

/// every PCI device in sysfs, or none if it can't be read
async fn pci_devices() -> Vec<PciDevice> {
    let mut dir = match tokio::fs::read_dir("/sys/bus/pci/devices").await {
        Ok(dir) => dir,
        Err(e) => {
            tracing::warn!(?e, "cannot list PCI devices");
            return vec![];
        }
    };
    let mut devices = vec![];
    while let Ok(Some(entry)) = dir.next_entry().await {
        match PciDevice::read(&entry.path()).await {
            Some(dev) => devices.push(dev),
            None => tracing::debug!(path = ?entry.path(), "cannot read PCI device"),
        }
    }
    devices
}

fn has_nvidia_gpu(pci: &[PciDevice]) -> bool {
    pci.iter().any(|d| d.is_nvidia_gpu())
}

fn has_wl_only_wifi(pci: &[PciDevice]) -> bool {
    pci.iter().any(|d| d.vendor == PCI_VENDOR_BROADCOM && WL_ONLY_DEVICES.contains(&d.device))
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

    async fn get_nvidia_chipset() -> Res<String> {
        let out = Command::new("lspci")
            .output()
            .await
            .map_err(|e| Err::FailToRunProgram("lspci", e))
            .wrap_msg("cannot detect nvidia chipset")?;
        Ok(nvidia_chipset(&String::from_utf8_lossy(&out.stdout)).unwrap_or_default().to_owned())
    }

    ///    Returns a list of Nvidia packages to install
    ///
    /// Returns `None` if no packaged driver supports the GPU.
    async fn list_nvidia_packages() -> Res<Option<Vec<String>>> {
        let mut pkgs = vec!["nvidia-gpu-firmware".into(), "libva-nvidia-driver".into()];
        let chipset = Self::get_nvidia_chipset().await?;
        match Self::get_nvidia_driver(&chipset) {
            "unsupported" => {
                tracing::info!(chipset, "Nvidia GPU is too old for any packaged driver");
                return Ok(None);
            }
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
        Ok(Some(pkgs))
    }

    async fn setup_nvidia_ostree(pkgs: &[String]) -> Res<()> {
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
                "--append=initcall_blacklist=simpledrm_platform_driver_init",
            ],
        )
        .await?;
        Ok(())
    }

    /// the kernel that boots next, which may be newer than the running one if an earlier attempt
    /// or the image installed one. Updates from [`super::DownloadUpdates`] only install on the
    /// restart, then akmods builds the module for their kernel while booting
    async fn next_kernel() -> Res<String> {
        // grubby reads the boot loader entries, which only root may
        let out = crate::root_output("grubby", ["--default-kernel"]).await;
        if let Ok(out) = &out
            && let Some(kver) = out.trim().strip_prefix("/boot/vmlinuz-")
        {
            return Ok(kver.to_owned());
        }
        tracing::warn!(?out, "cannot get the default kernel from grubby, using the running one");
        let kver = tokio::fs::read_to_string("/proc/sys/kernel/osrelease")
            .await
            .map_err(|e| Err::Message(format!("cannot read kernel version: {e}")))?;
        Ok(kver.trim().to_owned())
    }

    /// Waits for akmods to build the given kernel module, then checks that it's there.
    ///
    /// The akmod's `%posttrans` starts a build in the background, a second `akmods` waits on its
    /// lock and skips kmods that are already up to date, so this also builds it if that didn't.
    async fn kmod_built(module: &str) -> Res<bool> {
        let kver = Self::next_kernel().await?;
        tracing::info!(kver, module, "Waiting for akmods to build the kernel module");
        if let Err(e) = root("akmods", ["--kernels", &kver]).await {
            tracing::warn!(?e, "akmods failed");
        }
        Ok(Command::new("modinfo")
            .args(["-k", &kver, "-n", module])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .is_ok_and(|s| s.success()))
    }

    /// Returns whether the kernel module is ready, so a reboot would load it.
    async fn setup_nvidia(pci: &[PciDevice]) -> Res<bool> {
        tracing::info!("Setting up Nvidia drivers");
        let primary_gpu = std::env::var("STELLAR_OPTION").is_ok_and(|x| x == "1");
        if !has_nvidia_gpu(pci) {
            tracing::info!("No Nvidia GPU detected");
            return Ok(false);
        }
        let Some(pkgs) = Self::list_nvidia_packages().await? else {
            return Ok(false);
        };
        if super::is_ostree().await {
            tracing::debug!("ostree detected");
            // the kmod is built inside the rpm-ostree transaction, which needs a reboot anyway
            Self::setup_nvidia_ostree(&pkgs).await?;
            return Ok(true);
        }

        let mut args = vec!["in", "-y", "--allowerasing", "--best"];
        args.extend(pkgs.iter().map(String::as_str));
        root("dnf", &args).await.wrap_message(|_| format!("pkgs={pkgs:?}"))?;

        if primary_gpu {
            root(
                "sh",
                &[
                    "-c",
                    r#"
                    cp -p /usr/share/X11/xorg.conf.d/nvidia.conf /etc/X11/xorg.conf.d/nvidia.conf
                    sed -i '10i\\\tOption "PrimaryGPU" "yes"' /etc/X11/xorg.conf.d/nvidia.conf
                "#,
                ],
            )
            .await
            .wrap_msg("cannot set nvidia as primary gpu")?;
        }

        let built = Self::kmod_built("nvidia").await?;
        if !built {
            tracing::warn!("Nvidia kernel module isn't built, not asking for a reboot");
        }
        Ok(built)
    }

    async fn check_boardcom_bluetooth() -> bool {
        Command::new("sh")
            .arg("-c")
            .arg("lspci | grep -i Bluetooth | grep -q -i Broadcom")
            .status()
            .await
            .is_ok_and(|s| s.success())
    }

    /// Returns whether the `wl` kernel module is ready, so a reboot would load it.
    async fn setup_broadcom(pci: &[PciDevice]) -> Res<bool> {
        let mut built = false;
        if has_wl_only_wifi(pci) {
            tracing::info!("Setting up broadcom wifi drivers");
            root("dnf", &["in", "-y", "broadcom-wl", "akmod-wl"])
                .await
                .wrap_msg("fail to install broadcom wifi drivers")?;
            // broadcom-wl blacklists the in-kernel drivers, so wl only takes over after a reboot
            built = Self::kmod_built("wl").await?;
            if !built {
                tracing::warn!("wl kernel module isn't built, not asking for a reboot");
            }
        }
        if Self::check_boardcom_bluetooth().await {
            tracing::info!("Setting up broadcom bluetooth drivers");
            root("dnf", &["in", "-y", "broadcom-bt-firmware"])
                .await
                .wrap_msg("fail to install broadcom bluetooth drivers")?;
        }
        Ok(built)
    }
}

/// the chip of the first NVIDIA GPU in `lspci`'s output, e.g. `GA106` from
/// `01:00.0 VGA compatible controller: NVIDIA Corporation GA106 [GeForce RTX 3060] (rev a1)`
fn nvidia_chipset(lspci: &str) -> Option<&str> {
    lspci
        .lines()
        .filter(|l| l.contains("VGA compatible controller") || l.contains("3D controller"))
        .filter_map(|l| l.split_once(": ")?.1.strip_prefix("NVIDIA Corporation "))
        .find_map(|desc| desc.split_whitespace().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_chipset_from_lspci() {
        let lspci = "\
00:02.0 VGA compatible controller: Intel Corporation Alder Lake-P GT2 [Iris Xe Graphics] (rev 0c)
01:00.1 Audio device: NVIDIA Corporation GA106 High Definition Audio Controller (rev a1)
01:00.0 3D controller: NVIDIA Corporation GA106M [GeForce RTX 3060 Mobile / Max-Q] (rev a1)
";
        assert_eq!(nvidia_chipset(lspci), Some("GA106M"));
        let lspci = "01:00.0 VGA compatible controller: NVIDIA Corporation GK208B [GeForce GT 710] (rev a1)";
        assert_eq!(nvidia_chipset(lspci), Some("GK208B"));
        assert_eq!(Drivers::get_nvidia_driver(nvidia_chipset(lspci).unwrap()), "470xx");
        assert_eq!(
            nvidia_chipset("00:02.0 VGA compatible controller: Intel Corporation UHD"),
            None
        );
    }

    #[test]
    fn test_get_nvidia_driver() {
        assert_eq!(Drivers::get_nvidia_driver("NV34"), "unsupported");
        assert_eq!(Drivers::get_nvidia_driver("GK104"), "470xx");
        assert_eq!(Drivers::get_nvidia_driver("GK208"), "470xx");
        assert_eq!(Drivers::get_nvidia_driver("GT218"), "unsupported");
        assert_eq!(Drivers::get_nvidia_driver("GM107"), "580xx");
        assert_eq!(Drivers::get_nvidia_driver("GP108"), "580xx");
        assert_eq!(Drivers::get_nvidia_driver("GV100"), "580xx");
        assert_eq!(Drivers::get_nvidia_driver("TU116"), "latest");
        assert_eq!(Drivers::get_nvidia_driver("GA106M"), "latest");
        assert_eq!(Drivers::get_nvidia_driver("AD104"), "latest");
    }

    const fn dev(vendor: u16, device: u16, class: u32) -> PciDevice {
        PciDevice { vendor, device, class }
    }

    #[test]
    fn nvidia_gpu_by_class() {
        // nForce chipset bits and the HDMI audio function aren't GPUs
        let nforce = [dev(0x10de, 0x0aa2, 0x0c0500), dev(0x10de, 0x0ab4, 0x060100)];
        assert!(!has_nvidia_gpu(&nforce));
        assert!(!has_nvidia_gpu(&[dev(0x10de, 0x228e, 0x040300)]));
        // VGA, and 3D for laptop dGPUs
        assert!(has_nvidia_gpu(&[dev(0x10de, 0x2503, 0x030000)]));
        assert!(has_nvidia_gpu(&[dev(0x10de, 0x2520, 0x030200)]));
        // an Intel iGPU isn't one either
        assert!(!has_nvidia_gpu(&[dev(0x8086, 0x46a6, 0x030000)]));
    }

    #[test]
    fn wl_only_broadcom_wifi() {
        // BCM4360 and BCM4352 need wl
        assert!(has_wl_only_wifi(&[dev(0x14e4, 0x43a0, 0x028000)]));
        assert!(has_wl_only_wifi(&[dev(0x14e4, 0x43b1, 0x028000)]));
        // BCM4313 is brcmsmac's, BCM4350 and BCM43602 are brcmfmac's
        assert!(!has_wl_only_wifi(&[dev(0x14e4, 0x4727, 0x028000)]));
        assert!(!has_wl_only_wifi(&[dev(0x14e4, 0x43a3, 0x028000)]));
        assert!(!has_wl_only_wifi(&[dev(0x14e4, 0x43ba, 0x028000)]));
        // a non-Broadcom device with a matching id
        assert!(!has_wl_only_wifi(&[dev(0x8086, 0x4360, 0x028000)]));
    }

    #[tokio::test]
    async fn reads_sysfs_pci_device() {
        let dir = std::env::temp_dir().join(format!("taidan-pci-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("vendor"), "0x10de\n").unwrap();
        std::fs::write(dir.join("device"), "0x2503\n").unwrap();
        std::fs::write(dir.join("class"), "0x030000\n").unwrap();
        let read = PciDevice::read(&dir).await;
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(read, Some(dev(0x10de, 0x2503, 0x030000)));
    }
}
