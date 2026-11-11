use crate::{Verb, prelude::*, root};
use tokio::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NvidiaBranch {
    R580,
    Latest,
}

const NVIDIA_COMMON_PACKAGES: [&str; 2] = ["nvidia-gpu-firmware", "libva-nvidia-driver"];
const BROADCOM_PACKAGES: [&str; 2] = ["broadcom-wl", "akmod-wl"];

pub(crate) fn driver_packages() -> Vec<&'static str> {
    [
        &NVIDIA_COMMON_PACKAGES[..],
        &NvidiaBranch::R580.packages(),
        &NvidiaBranch::Latest.packages(),
        &BROADCOM_PACKAGES,
    ]
    .concat()
}

impl NvidiaBranch {
    fn packages(self) -> [&'static str; 3] {
        match self {
            Self::R580 => [
                "akmod-nvidia-580xx",
                "xorg-x11-drv-nvidia-580xx",
                "xorg-x11-drv-nvidia-580xx-cuda",
            ],
            Self::Latest => ["akmod-nvidia", "xorg-x11-drv-nvidia", "xorg-x11-drv-nvidia-cuda"],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NvidiaChip {
    Supported { oldest: NvidiaBranch, newest: NvidiaBranch },
    TooOld,
    Unknown,
}

// 390xx (Fermi) and 470xx (Kepler) have no GBM, which the Wayland-only desktops need, and 340xx
// is dead. `C5x` to `C7x` are nForce chipset graphics.
const PRE_MAXWELL: &[&str] = &["NV", "MCP", "C5", "C6", "C7", "G7", "G8", "G9", "GT", "GF", "GK"];
const MAXWELL_TO_VOLTA: &[&str] = &["GM", "GP", "GV"];
const TURING_AND_NEWER: &[&str] = &["TU", "GA", "GH", "AD", "GB"];

impl NvidiaChip {
    fn from_chipset(chipset: &str) -> Self {
        use NvidiaBranch::{Latest, R580};
        let prefix = |p: &[&str]| p.iter().any(|p| chipset.starts_with(p));
        if prefix(PRE_MAXWELL) {
            Self::TooOld
        } else if prefix(MAXWELL_TO_VOLTA) {
            Self::Supported { oldest: R580, newest: R580 }
        } else if prefix(TURING_AND_NEWER) {
            Self::Supported { oldest: R580, newest: Latest }
        } else {
            Self::Unknown
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NvidiaDriver {
    Branch(NvidiaBranch),
    TooOld,
    Unknown,
    Incompatible,
}

impl NvidiaDriver {
    fn for_chipsets<S: AsRef<str>>(chipsets: &[S]) -> Self {
        let chips: Vec<_> = chipsets.iter().map(|c| NvidiaChip::from_chipset(c.as_ref())).collect();
        if chips.is_empty() || chips.contains(&NvidiaChip::Unknown) {
            return Self::Unknown;
        }
        if chips.iter().all(|c| *c == NvidiaChip::TooOld) {
            return Self::TooOld;
        }
        let mut oldest = NvidiaBranch::R580;
        let mut newest = NvidiaBranch::Latest;
        for chip in chips {
            let NvidiaChip::Supported { oldest: o, newest: n } = chip else {
                return Self::Incompatible;
            };
            oldest = oldest.max(o);
            newest = newest.min(n);
        }
        if oldest > newest {
            return Self::Incompatible;
        }
        Self::Branch(newest)
    }
}

pub(crate) const NVIDIA_PRIMARY_GPU_CONF: &str = r#"Section "OutputClass"
	Identifier "nvidia-primary"
	MatchDriver "nvidia-drm"
	Driver "nvidia"
	Option "PrimaryGPU" "yes"
EndSection
"#;

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
        cfg: &crate::cfg::Config,
        sender: &C,
    ) -> Res<()> {
        if !settings.install_codecs_drivers || settings.nointernet {
            return Ok(());
        }
        let pci = pci_devices().await;
        let mut done = Setup::default();
        match Drivers::setup_nvidia(cfg, &pci).await {
            Ok(setup) => done |= setup,
            Err(e) => {
                tracing::warn!(?e, "cannot setup nvidia drivers");
                done.failed = true;
            }
        }
        done |= Drivers::setup_broadcom(cfg, &pci).await;
        if done.needs_reboot {
            sender.send(crate::InstallMsg::NeedsReboot);
        }
        if done.failed {
            sender.send(crate::InstallMsg::DriversFailed);
        }
        // FIXME: refactor this to somewhere else not in this file
        // also this is done inside `_05_dnfdownloadapps.rs`
        // Codecs::install_codecs().await?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Setup {
    needs_reboot: bool,
    failed: bool,
}

impl std::ops::BitOrAssign for Setup {
    fn bitor_assign(&mut self, rhs: Self) {
        self.needs_reboot |= rhs.needs_reboot;
        self.failed |= rhs.failed;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Hardware {
    pub nvidia: bool,
    pub broadcom_wifi: bool,
    pub secure_boot_blocks: bool,
}

pub async fn detect_hardware() -> Hardware {
    let pci = pci_devices().await;
    let nvidia = has_nvidia_gpu(&pci)
        && matches!(Drivers::get_nvidia_driver().await, Ok(NvidiaDriver::Branch(_)));
    let broadcom_wifi = has_wl_only_wifi(&pci);
    Hardware {
        nvidia,
        broadcom_wifi,
        secure_boot_blocks: (nvidia || broadcom_wifi) && !Drivers::kmods_would_load().await,
    }
}

const PCI_VENDOR_NVIDIA: u16 = 0x10de;
const PCI_VENDOR_BROADCOM: u16 = 0x14e4;
const PCI_CLASS_VGA: u32 = 0x0300;
const PCI_CLASS_3D: u32 = 0x0302;

// `wl`'s device list (https://wiki.debian.org/wl) minus the `brcmsmac` chips, which work with
// linux-firmware. the b43 chips stay since Fedora doesn't ship their proprietary firmware
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
    class_code: u32,
}

impl PciDevice {
    async fn read_from_sysfs(dir: &std::path::Path) -> Option<Self> {
        let read = async |name| {
            let s = tokio::fs::read_to_string(dir.join(name)).await.ok()?;
            u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
        };
        Some(Self {
            vendor: read("vendor").await?.try_into().ok()?,
            device: read("device").await?.try_into().ok()?,
            class_code: read("class").await?,
        })
    }

    const fn is_nvidia_gpu(self) -> bool {
        self.vendor == PCI_VENDOR_NVIDIA
            && matches!(self.class_code >> 8, PCI_CLASS_VGA | PCI_CLASS_3D)
    }
}

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
        match PciDevice::read_from_sysfs(&entry.path()).await {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Installed {
    Dnf,
    Ostree,
    Skipped,
}

async fn install_pkgs(cfg: &crate::cfg::Config, pkgs: &[&str]) -> Res<Installed> {
    let owned = pkgs.iter().map(|&p| p.to_owned()).collect();
    let (verb, installed) = if super::is_ostree().await {
        (Verb::RpmOstreeInstall { pkgs: owned }, Installed::Ostree)
    } else if cfg.taidan0.skip_dnf {
        tracing::info!(?pkgs, "skip_dnf is set, not installing driver packages");
        return Ok(Installed::Skipped);
    } else {
        (Verb::DnfDrivers { pkgs: owned }, Installed::Dnf)
    };
    root(verb).await.wrap_message(|_| format!("pkgs={pkgs:?}"))?;
    Ok(installed)
}

impl Drivers {
    async fn get_nvidia_driver() -> Res<NvidiaDriver> {
        let out = Command::new("lspci")
            .output()
            .await
            .map_err(|e| Err::FailToRunProgram("lspci", e))
            .wrap_msg("cannot detect nvidia chipset")?;
        if !out.status.success() {
            return Err(Err::program_fail("lspci", &out)).wrap_msg("cannot detect nvidia chipset");
        }
        let lspci = String::from_utf8_lossy(&out.stdout);
        let chipsets = nvidia_chipsets(&lspci);
        let driver = NvidiaDriver::for_chipsets(&chipsets);
        tracing::info!(?chipsets, ?driver, "detected Nvidia GPUs");
        Ok(driver)
    }

    async fn kernel_booting_next() -> Res<String> {
        let out = crate::root_output(Verb::DefaultKernel).await;
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

    async fn build_kmod(module: &str) -> Res<bool> {
        let kver = Self::kernel_booting_next().await?;
        tracing::info!(kver, module, "Waiting for akmods to build the kernel module");
        if let Err(e) = root(Verb::Akmods { kernel: kver.clone() }).await {
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

    async fn kmods_would_load() -> bool {
        const AKMODS_KEY: &str = "/etc/pki/akmods/certs/public_key.der";
        match secure_boot_enforced().await {
            Some(false) => return true,
            Some(true) => {}
            None => {
                tracing::warn!("cannot tell whether Secure Boot is on, taking it to be");
                return false;
            }
        }
        // rpm-ostree builds kmods without the akmods signing key
        if super::is_ostree().await {
            return false;
        }
        if !tokio::fs::try_exists(AKMODS_KEY).await.unwrap_or_default() {
            return false;
        }
        let out = Command::new("mokutil").args(["--test-key", AKMODS_KEY]).output().await;
        out.is_ok_and(|out| String::from_utf8_lossy(&out.stdout).contains("already enrolled"))
    }

    async fn setup_nvidia(cfg: &crate::cfg::Config, pci: &[PciDevice]) -> Res<Setup> {
        tracing::info!("Setting up Nvidia drivers");
        if !has_nvidia_gpu(pci) {
            tracing::info!("No Nvidia GPU detected");
            return Ok(Setup::default());
        }
        if !Self::kmods_would_load().await {
            tracing::warn!(
                "Secure Boot is on and the module wouldn't be signed with an enrolled key, so \
                 the NVIDIA module wouldn't load. Not installing it, the open driver keeps working"
            );
            return Ok(Setup::default());
        }
        let branch = match Self::get_nvidia_driver().await? {
            NvidiaDriver::Branch(branch) => branch,
            NvidiaDriver::TooOld => {
                tracing::info!("Nvidia GPU is too old for any packaged driver");
                return Ok(Setup::default());
            }
            NvidiaDriver::Unknown => {
                tracing::warn!("cannot tell which Nvidia GPU this is, not installing a driver");
                return Ok(Setup::default());
            }
            NvidiaDriver::Incompatible => {
                tracing::warn!("no packaged Nvidia driver supports every GPU, not installing one");
                return Ok(Setup::default());
            }
        };
        let pkgs = [&NVIDIA_COMMON_PACKAGES[..], &branch.packages()].concat();
        let installed = install_pkgs(cfg, &pkgs).await?;
        if installed == Installed::Skipped {
            return Ok(Setup::default());
        }
        let primary_gpu = std::env::var("STELLAR_OPTION").is_ok_and(|x| x == "1");
        let mut primary_failed = false;
        if primary_gpu && let Err(e) = root(Verb::NvidiaPrimaryGpu).await {
            tracing::warn!(?e, "cannot set nvidia as primary gpu");
            primary_failed = true;
        }
        if installed == Installed::Ostree {
            root(Verb::NvidiaKargs).await?;
            return Ok(Setup { needs_reboot: true, failed: primary_failed });
        }

        let built = Self::build_kmod("nvidia").await?;
        if !built {
            tracing::warn!("Nvidia kernel module isn't built, not asking for a reboot");
        }
        Ok(Setup { needs_reboot: built, failed: !built || primary_failed })
    }

    async fn setup_broadcom(cfg: &crate::cfg::Config, pci: &[PciDevice]) -> Setup {
        if !has_wl_only_wifi(pci) {
            return Setup::default();
        }
        if !Self::kmods_would_load().await {
            tracing::warn!(
                "Secure Boot is on and the module wouldn't be signed with an enrolled key, so \
                 wl wouldn't load. Not installing it"
            );
            return Setup::default();
        }
        tracing::info!("Setting up broadcom wifi drivers");
        match install_pkgs(cfg, &BROADCOM_PACKAGES).await {
            Ok(Installed::Dnf) => {
                let built = Self::build_kmod("wl").await.unwrap_or_else(|e| {
                    tracing::warn!(?e, "cannot check the wl kernel module");
                    false
                });
                if !built {
                    tracing::warn!("wl kernel module isn't built, not asking for a reboot");
                }
                Setup { needs_reboot: built, failed: !built }
            }
            Ok(Installed::Ostree) => Setup { needs_reboot: true, failed: false },
            Ok(Installed::Skipped) => Setup::default(),
            Err(e) => {
                tracing::warn!(?e, "fail to install broadcom wifi drivers");
                Setup { needs_reboot: false, failed: true }
            }
        }
    }
}

pub(crate) async fn secure_boot_enforced() -> Option<bool> {
    const EFIVARS: &str = "/sys/firmware/efi/efivars";
    const EFI_VAR_ATTRIBUTES_LEN: usize = 4;
    const MOK_VALIDATION_DISABLED: u8 = 1;
    let efi_var_value = |name: &'static str| async move {
        match tokio::fs::read(format!("{EFIVARS}/{name}")).await {
            Ok(bytes) => Ok(bytes.get(EFI_VAR_ATTRIBUTES_LEN).copied()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    };
    if !tokio::fs::try_exists("/sys/firmware/efi").await.unwrap_or_default() {
        return Some(false);
    }
    if !tokio::fs::try_exists(EFIVARS).await.unwrap_or_default() {
        return None;
    }
    match efi_var_value("SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c").await {
        Ok(Some(1)) => {}
        Ok(Some(0) | None) => return Some(false),
        Ok(Some(_)) | Err(_) => return None,
    }
    match efi_var_value("MokSBStateRT-605dab50-e046-4300-abb6-3dd810dd8b23").await {
        Ok(Some(MOK_VALIDATION_DISABLED)) => Some(false),
        Ok(_) => Some(true),
        Err(_) => None,
    }
}

fn nvidia_chipsets(lspci: &str) -> Vec<&str> {
    lspci
        .lines()
        .filter(|l| l.contains("VGA compatible controller") || l.contains("3D controller"))
        .filter_map(|l| l.split_once(": ")?.1.strip_prefix("NVIDIA Corporation "))
        .filter_map(|desc| desc.split_whitespace().next())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_chipsets_from_lspci() {
        let lspci = "\
00:02.0 VGA compatible controller: Intel Corporation Alder Lake-P GT2 [Iris Xe Graphics] (rev 0c)
01:00.1 Audio device: NVIDIA Corporation GA106 High Definition Audio Controller (rev a1)
01:00.0 3D controller: NVIDIA Corporation GA106M [GeForce RTX 3060 Mobile / Max-Q] (rev a1)
";
        assert_eq!(nvidia_chipsets(lspci), ["GA106M"]);
        let lspci = "\
01:00.0 VGA compatible controller: NVIDIA Corporation GK208B [GeForce GT 710] (rev a1)
02:00.0 VGA compatible controller: NVIDIA Corporation TU116 [GeForce GTX 1660 SUPER] (rev a1)
";
        assert_eq!(nvidia_chipsets(lspci), ["GK208B", "TU116"]);
        let lspci_newer_than_pci_ids =
            "01:00.0 VGA compatible controller: NVIDIA Corporation Device 2c05 (rev a1)";
        assert_eq!(nvidia_chipsets(lspci_newer_than_pci_ids), ["Device"]);
        assert!(
            nvidia_chipsets("00:02.0 VGA compatible controller: Intel Corporation UHD").is_empty()
        );
    }

    #[test]
    fn nvidia_chip_support() {
        use NvidiaBranch::{Latest, R580};
        for chip in ["NV34", "MCP79", "C51", "G86", "G92", "GT218", "GF108", "GK104", "GK208B"] {
            assert_eq!(NvidiaChip::from_chipset(chip), NvidiaChip::TooOld, "{chip}");
        }
        for chip in ["GM107", "GM204", "GP108", "GP102", "GV100"] {
            let only_580 = NvidiaChip::Supported { oldest: R580, newest: R580 };
            assert_eq!(NvidiaChip::from_chipset(chip), only_580, "{chip}");
        }
        for chip in ["TU116", "TU117M", "GA106M", "GH100", "AD104", "GB202", "GB206M"] {
            let both = NvidiaChip::Supported { oldest: R580, newest: Latest };
            assert_eq!(NvidiaChip::from_chipset(chip), both, "{chip}");
        }
        for chip in ["", "Device", "XX100"] {
            assert_eq!(NvidiaChip::from_chipset(chip), NvidiaChip::Unknown, "{chip:?}");
        }
    }

    #[test]
    fn nvidia_driver_for_chipsets() {
        use NvidiaBranch::{Latest, R580};
        let driver = |chips: &[&str]| NvidiaDriver::for_chipsets(chips);
        assert_eq!(driver(&[]), NvidiaDriver::Unknown);
        assert_eq!(driver(&[""]), NvidiaDriver::Unknown);
        assert_eq!(driver(&["Device"]), NvidiaDriver::Unknown);
        assert_eq!(driver(&["TU116", "Device"]), NvidiaDriver::Unknown);
        assert_eq!(driver(&["GK104"]), NvidiaDriver::TooOld);
        assert_eq!(driver(&["GK104", "GF108"]), NvidiaDriver::TooOld);
        assert_eq!(driver(&["GK208B", "TU116"]), NvidiaDriver::Incompatible);
        assert_eq!(driver(&["GP108"]), NvidiaDriver::Branch(R580));
        assert_eq!(driver(&["GM107", "GP104"]), NvidiaDriver::Branch(R580));
        assert_eq!(driver(&["TU116"]), NvidiaDriver::Branch(Latest));
        assert_eq!(driver(&["GA106M", "AD104"]), NvidiaDriver::Branch(Latest));
        assert_eq!(driver(&["GP108", "TU116"]), NvidiaDriver::Branch(R580));
        assert_eq!(driver(&["AD104", "GP108"]), NvidiaDriver::Branch(R580));
    }

    #[test]
    fn nvidia_branch_packages() {
        assert_eq!(NvidiaBranch::Latest.packages()[0], "akmod-nvidia");
        assert!(NvidiaBranch::R580.packages().iter().all(|p| p.contains("-580xx")));
    }

    const fn dev(vendor: u16, device: u16, class_code: u32) -> PciDevice {
        PciDevice { vendor, device, class_code }
    }

    #[test]
    fn nvidia_gpu_by_class() {
        let nforce = [dev(0x10de, 0x0aa2, 0x0c0500), dev(0x10de, 0x0ab4, 0x060100)];
        assert!(!has_nvidia_gpu(&nforce));
        let hdmi_audio = dev(0x10de, 0x228e, 0x040300);
        assert!(!has_nvidia_gpu(&[hdmi_audio]));
        let vga = dev(0x10de, 0x2503, 0x030000);
        assert!(has_nvidia_gpu(&[vga]));
        let laptop_3d = dev(0x10de, 0x2520, 0x030200);
        assert!(has_nvidia_gpu(&[laptop_3d]));
        let intel_igpu = dev(0x8086, 0x46a6, 0x030000);
        assert!(!has_nvidia_gpu(&[intel_igpu]));
    }

    #[test]
    fn wl_only_broadcom_wifi() {
        let bcm4360 = dev(0x14e4, 0x43a0, 0x028000);
        let bcm4352 = dev(0x14e4, 0x43b1, 0x028000);
        assert!(has_wl_only_wifi(&[bcm4360]));
        assert!(has_wl_only_wifi(&[bcm4352]));
        let bcm4313_brcmsmac = dev(0x14e4, 0x4727, 0x028000);
        let bcm4350_brcmfmac = dev(0x14e4, 0x43a3, 0x028000);
        let bcm43602_brcmfmac = dev(0x14e4, 0x43ba, 0x028000);
        assert!(!has_wl_only_wifi(&[bcm4313_brcmsmac]));
        assert!(!has_wl_only_wifi(&[bcm4350_brcmfmac]));
        assert!(!has_wl_only_wifi(&[bcm43602_brcmfmac]));
        let non_broadcom_with_wl_device_id = dev(0x8086, 0x4360, 0x028000);
        assert!(!has_wl_only_wifi(&[non_broadcom_with_wl_device_id]));
    }

    #[tokio::test]
    async fn reads_sysfs_pci_device() {
        let dir = tempfile::tempdir().unwrap();
        for (name, value) in
            [("vendor", "0x10de\n"), ("device", "0x2503\n"), ("class", "0x030000\n")]
        {
            std::fs::write(dir.path().join(name), value).unwrap();
        }
        let read = PciDevice::read_from_sysfs(dir.path()).await;
        assert_eq!(read, Some(dev(0x10de, 0x2503, 0x030000)));
    }
}
