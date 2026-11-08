use crate::{prelude::*, root, root_with_stdin};
use tokio::process::Command;
/// an NVIDIA driver branch RPM Fusion packages, oldest first, so the oldest of several sorts
/// lowest
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NvidiaBranch {
    /// the last branch for Maxwell, Pascal and Volta, `akmod-nvidia-580xx`
    R580,
    /// `akmod-nvidia`, Turing and newer
    Latest,
}

impl NvidiaBranch {
    /// the akmod, the driver and the CUDA libraries, which also have Vulkan and OpenGL
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

/// which packaged branches drive an NVIDIA chip, from its name in `lspci`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NvidiaChip {
    /// every branch from `oldest` to `newest`
    Supported { oldest: NvidiaBranch, newest: NvidiaBranch },
    /// older than any packaged branch supports
    TooOld,
    /// not a chip this knows, e.g. `Device 2c05` when `pci.ids` doesn't have it yet
    Unknown,
}

impl NvidiaChip {
    fn from_chipset(chipset: &str) -> Self {
        use NvidiaBranch::{Latest, R580};
        const fn branches(oldest: NvidiaBranch, newest: NvidiaBranch) -> NvidiaChip {
            NvidiaChip::Supported { oldest, newest }
        }
        let prefix = |p: &[&str]| p.iter().any(|p| chipset.starts_with(p));
        // RPM Fusion stopped packaging 340xx, and 390xx (Fermi) and 470xx (Kepler) have no GBM,
        // which the Wayland-only desktops need, so those stay on nouveau. `C5x` to `C7x` are
        // nForce chipset graphics, e.g. `C51 [GeForce 6150 LE]`
        if prefix(&["NV", "MCP", "C5", "C6", "C7", "G7", "G8", "G9", "GT", "GF", "GK"]) {
            Self::TooOld
        // Maxwell, Pascal and Volta, dropped in the 590 driver
        } else if prefix(&["GM", "GP", "GV"]) {
            branches(R580, R580)
        // Turing, Ampere, Hopper, Ada and Blackwell. 580 drives all of them too
        } else if prefix(&["TU", "GA", "GH", "AD", "GB"]) {
            branches(R580, Latest)
        } else {
            Self::Unknown
        }
    }
}

/// what to install for every NVIDIA GPU in the system
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NvidiaDriver {
    Branch(NvidiaBranch),
    /// every GPU is too old for a packaged driver, nouveau drives them
    TooOld,
    /// no GPU was found, or one isn't known, so nothing is installed rather than a driver that
    /// might take the display away
    Unknown,
    /// no single branch drives every GPU, e.g. a Kepler card next to a Turing one
    Incompatible,
}

impl NvidiaDriver {
    /// the oldest branch any of the GPUs needs, if it drives the rest too
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

/// makes the NVIDIA GPU drive the displays instead of the integrated one, for `STELLAR_OPTION`
const NVIDIA_PRIMARY_GPU_CONF: &str = r#"Section "OutputClass"
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
        // a failed driver doesn't fail the installation, the system works without it, but the
        // done page says so
        let mut done = Setup::default();
        match Drivers::setup_nvidia(cfg, &pci).await {
            Ok(setup) => done |= setup,
            Err(e) => {
                tracing::warn!(?e, "cannot setup nvidia drivers");
                done.failed = true;
            }
        }
        match Drivers::setup_broadcom(cfg, &pci).await {
            Ok(setup) => done |= setup,
            Err(e) => {
                tracing::warn!(?e, "cannot setup broadcom drivers");
                done.failed = true;
            }
        }
        if done.reboot {
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

/// what setting up a driver did
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Setup {
    /// a reboot would load the driver
    reboot: bool,
    /// some of it didn't work, e.g. the packages are in but the kernel module didn't build
    failed: bool,
}

impl std::ops::BitOrAssign for Setup {
    fn bitor_assign(&mut self, rhs: Self) {
        self.reboot |= rhs.reboot;
        self.failed |= rhs.failed;
    }
}

/// hardware that needs extra drivers, as installed by [`DriversCodecs`]
#[derive(Clone, Copy, Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Hardware {
    /// an NVIDIA GPU a packaged driver supports
    pub nvidia: bool,
    pub broadcom_wifi: bool,
    pub broadcom_bluetooth: bool,
    /// Secure Boot keeps the NVIDIA or Broadcom Wi-Fi driver from loading, so they aren't
    /// installed, see `Drivers::kmods_would_load`
    pub secure_boot_blocks: bool,
}

/// detect hardware that [`DriversCodecs`] would install drivers for
pub async fn detect_hardware() -> Hardware {
    let pci = pci_devices().await;
    let nvidia = has_nvidia_gpu(&pci)
        && matches!(Drivers::get_nvidia_driver().await, Ok(NvidiaDriver::Branch(_)));
    let broadcom_wifi = has_wl_only_wifi(&pci);
    Hardware {
        nvidia,
        broadcom_wifi,
        broadcom_bluetooth: has_broadcom_bluetooth(&pci, &usb_interfaces().await),
        secure_boot_blocks: (nvidia || broadcom_wifi) && !Drivers::kmods_would_load().await,
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

/// USB vendors whose vendor-specific Bluetooth interfaces are Broadcom chips that need patchram
/// firmware (`broadcom-bt-firmware`), Broadcom's own and the rebrands. From the
/// `BTUSB_BCM_PATCHRAM` entries in the kernel's `drivers/bluetooth/btusb_main.c`. Apple's are
/// left out, `btusb` doesn't load firmware for them
const BCM_PATCHRAM_USB_VENDORS: &[u16] = &[
    0x0a5c, // Broadcom
    0x0489, // Foxconn / Hon Hai
    0x04ca, // Lite-On
    0x050d, // Belkin
    0x0930, // Toshiba
    0x0b05, // ASUS
    0x0bb4, // HTC
    0x105b, // Foxconn / Lenovo
    0x13d3, // IMC Networks
    0x413c, // Dell
];

/// a USB interface, e.g. `1-4:1.0` in `/sys/bus/usb/devices`, with its device's ids
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UsbInterface {
    vendor: u16,
    product: u16,
    /// class, subclass and protocol
    class: [u8; 3],
}

impl UsbInterface {
    /// read the interface `name` from `devices`, `None` if it isn't one
    async fn read(devices: &std::path::Path, name: &str) -> Option<Self> {
        let (device, _) = name.split_once(':')?;
        let intf = devices.join(name);
        let device = devices.join(device);
        let read = async |path: std::path::PathBuf| {
            let s = tokio::fs::read_to_string(path).await.ok()?;
            u16::from_str_radix(s.trim(), 16).ok()
        };
        let class = async |name| u8::try_from(read(intf.join(name)).await?).ok();
        Some(Self {
            vendor: read(device.join("idVendor")).await?,
            product: read(device.join("idProduct")).await?,
            class: [
                class("bInterfaceClass").await?,
                class("bInterfaceSubClass").await?,
                class("bInterfaceProtocol").await?,
            ],
        })
    }

    /// matches `btusb`'s Broadcom patchram entries
    fn is_broadcom_bluetooth(self) -> bool {
        // Broadcom BCM20702B0 (Dynex/Insignia), the one listed by its product id
        (self.vendor == 0x19ff && self.product == 0x0239)
            || (BCM_PATCHRAM_USB_VENDORS.contains(&self.vendor) && self.class == [0xff, 0x01, 0x01])
    }
}

/// every USB interface in sysfs, or none if it can't be read
async fn usb_interfaces() -> Vec<UsbInterface> {
    let devices = std::path::Path::new("/sys/bus/usb/devices");
    let mut dir = match tokio::fs::read_dir(devices).await {
        Ok(dir) => dir,
        Err(e) => {
            tracing::warn!(?e, "cannot list USB devices");
            return vec![];
        }
    };
    let mut interfaces = vec![];
    while let Ok(Some(entry)) = dir.next_entry().await {
        // devices and hubs have no `:` in their names, they're read through their interfaces
        if let Some(intf) = UsbInterface::read(devices, &entry.file_name().to_string_lossy()).await
        {
            interfaces.push(intf);
        }
    }
    interfaces
}

/// a Broadcom Bluetooth controller that needs `broadcom-bt-firmware`, usually over USB, but a
/// few are PCI
fn has_broadcom_bluetooth(pci: &[PciDevice], usb: &[UsbInterface]) -> bool {
    // 0x0d11 is a Bluetooth controller
    pci.iter().any(|d| d.vendor == PCI_VENDOR_BROADCOM && d.class >> 8 == 0x0d11)
        || usb.iter().any(|i| i.is_broadcom_bluetooth())
}

// NOTE: added during `_05_dnfdownloadapps.rs`
impl Codecs {
    pub const fn codecs() -> &'static [&'static str] {
        &["@multimedia"]
    }
}

/// how [`install_pkgs`] installed packages
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Installed {
    /// with dnf, so akmods builds kmods on the running system
    Dnf,
    /// layered with rpm-ostree, which builds kmods in the transaction and needs a reboot
    Ostree,
    /// not at all, since the config skips dnf on a non-atomic system
    Skipped,
}

/// install packages with rpm-ostree on atomic systems, or dnf unless the config skips it
async fn install_pkgs(cfg: &crate::cfg::Config, pkgs: &[&str]) -> Res<Installed> {
    if super::is_ostree().await {
        // idempotent, so a retried install doesn't fail on packages it already layered
        let mut args = vec!["install", "-y", "--idempotent"];
        args.extend_from_slice(pkgs);
        root("rpm-ostree", &args).await.wrap_message(|_| format!("pkgs={pkgs:?}"))?;
        return Ok(Installed::Ostree);
    }
    if cfg.taidan0.skip_dnf {
        tracing::info!(?pkgs, "skip_dnf is set, not installing driver packages");
        return Ok(Installed::Skipped);
    }
    let mut args = vec!["in", "-y", "--allowerasing", "--best"];
    args.extend_from_slice(pkgs);
    root("dnf", &args).await.wrap_message(|_| format!("pkgs={pkgs:?}"))?;
    Ok(Installed::Dnf)
}

impl Drivers {
    /// the driver for every NVIDIA GPU `lspci` lists
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

    async fn setup_nvidia_kargs() -> Res<()> {
        // append-if-missing so a retry doesn't duplicate kargs. rpm-ostree exits 0 on no changes
        root(
            "rpm-ostree",
            &[
                "kargs",
                "--append-if-missing=rd.driver.blacklist=nouveau",
                "--append-if-missing=modprobe.blacklist=nouveau",
                "--append-if-missing=nvidia-drm.modeset=1",
                "--append-if-missing=initcall_blacklist=simpledrm_platform_driver_init",
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

    /// whether a module akmods builds would load: Secure Boot is off, or the key akmods signs with
    /// is enrolled. Otherwise the kernel refuses it, and as the driver packages turn off nouveau
    /// (and simpledrm on rpm-ostree), the next boot would have no display at all
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
        if !tokio::fs::try_exists(AKMODS_KEY).await.unwrap_or_default() {
            return false;
        }
        let out = Command::new("mokutil").args(["--test-key", AKMODS_KEY]).output().await;
        out.is_ok_and(|out| String::from_utf8_lossy(&out.stdout).contains("already enrolled"))
    }

    /// Returns whether the kernel module is ready, so a reboot would load it.
    async fn setup_nvidia(cfg: &crate::cfg::Config, pci: &[PciDevice]) -> Res<Setup> {
        tracing::info!("Setting up Nvidia drivers");
        let primary_gpu = std::env::var("STELLAR_OPTION").is_ok_and(|x| x == "1");
        if !has_nvidia_gpu(pci) {
            tracing::info!("No Nvidia GPU detected");
            return Ok(Setup::default());
        }
        if !Self::kmods_would_load().await {
            tracing::warn!(
                "Secure Boot is on and the akmods signing key isn't enrolled, so the NVIDIA \
                 module wouldn't load. Not installing it, the open driver keeps working"
            );
            return Ok(Setup::default());
        }
        // nouveau keeps driving the GPU unless a driver is sure to support it
        let branch = match Self::get_nvidia_driver().await? {
            NvidiaDriver::Branch(branch) => branch,
            NvidiaDriver::TooOld => {
                tracing::info!("Nvidia GPU is too old for any packaged driver");
                return Ok(Setup::default());
            }
            NvidiaDriver::Unknown => {
                tracing::warn!("cannot tell which Nvidia GPU this is, not installing a driver");
                return Ok(Setup { reboot: false, failed: true });
            }
            NvidiaDriver::Incompatible => {
                tracing::warn!("no packaged Nvidia driver supports every GPU, not installing one");
                return Ok(Setup { reboot: false, failed: true });
            }
        };
        let mut pkgs = vec!["nvidia-gpu-firmware", "libva-nvidia-driver"];
        pkgs.extend(branch.packages());
        let installed = install_pkgs(cfg, &pkgs).await?;
        if installed == Installed::Skipped {
            return Ok(Setup::default());
        }
        // the driver works without it, so the done page mentions it but it's not worth more
        let mut primary_failed = false;
        if primary_gpu {
            // a snippet of its own, the packaged `nvidia.conf` stays as it is. Xorg applies every
            // `OutputClass` that matches, so this only adds the option
            if let Err(e) = root_with_stdin(
                "install",
                ["-Dm644", "/dev/stdin", "/etc/X11/xorg.conf.d/10-nvidia-primary.conf"],
                NVIDIA_PRIMARY_GPU_CONF.as_bytes(),
            )
            .await
            {
                tracing::warn!(?e, "cannot set nvidia as primary gpu");
                primary_failed = true;
            }
        }
        if installed == Installed::Ostree {
            // the kmod is built inside the rpm-ostree transaction, which needs a reboot anyway
            Self::setup_nvidia_kargs().await?;
            return Ok(Setup { reboot: true, failed: primary_failed });
        }

        let built = Self::kmod_built("nvidia").await?;
        if !built {
            tracing::warn!("Nvidia kernel module isn't built, not asking for a reboot");
        }
        Ok(Setup { reboot: built, failed: !built || primary_failed })
    }

    /// Returns whether a reboot would load the drivers, i.e. the `wl` kernel module is ready or
    /// rpm-ostree layered the packages.
    async fn setup_broadcom(cfg: &crate::cfg::Config, pci: &[PciDevice]) -> Res<Setup> {
        let mut done = Setup::default();
        if has_wl_only_wifi(pci) && !Self::kmods_would_load().await {
            tracing::warn!(
                "Secure Boot is on and the akmods signing key isn't enrolled, so wl wouldn't load. \
                 Not installing it"
            );
        } else if has_wl_only_wifi(pci) {
            tracing::info!("Setting up broadcom wifi drivers");
            // broadcom-wl blacklists the in-kernel drivers, so wl only takes over after a reboot
            // the Bluetooth firmware below doesn't depend on wl, so a failure here doesn't stop it
            match install_pkgs(cfg, &["broadcom-wl", "akmod-wl"]).await {
                Ok(Installed::Dnf) => {
                    let built = Self::kmod_built("wl").await.unwrap_or_else(|e| {
                        tracing::warn!(?e, "cannot check the wl kernel module");
                        false
                    });
                    if !built {
                        tracing::warn!("wl kernel module isn't built, not asking for a reboot");
                    }
                    done |= Setup { reboot: built, failed: !built };
                }
                // built inside the rpm-ostree transaction
                Ok(Installed::Ostree) => done.reboot = true,
                Ok(Installed::Skipped) => {}
                Err(e) => {
                    tracing::warn!(?e, "fail to install broadcom wifi drivers");
                    done.failed = true;
                }
            }
        }
        if has_broadcom_bluetooth(pci, &usb_interfaces().await) {
            tracing::info!("Setting up broadcom bluetooth drivers");
            // not worth losing the wifi driver's reboot over
            match install_pkgs(cfg, &["broadcom-bt-firmware"]).await {
                // layered packages only show up after a reboot
                Ok(installed) => done.reboot |= installed == Installed::Ostree,
                Err(e) => {
                    tracing::warn!(?e, "cannot install broadcom bluetooth firmware");
                    done.failed = true;
                }
            }
        }
        Ok(done)
    }
}

/// whether the kernel only loads signed modules: Secure Boot is on in the firmware and shim
/// hasn't had validation turned off in MokManager, as the kernel decides it. [`None`] if it can't
/// be told
async fn secure_boot_enforced() -> Option<bool> {
    const EFIVARS: &str = "/sys/firmware/efi/efivars";
    // the last byte of an EFI variable, after 4 bytes of attributes
    let var = |name: &'static str| async move {
        match tokio::fs::read(format!("{EFIVARS}/{name}")).await {
            Ok(bytes) => Ok(bytes.get(4).copied()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    };
    if !tokio::fs::try_exists("/sys/firmware/efi").await.unwrap_or_default() {
        // BIOS, no Secure Boot
        return Some(false);
    }
    if !tokio::fs::try_exists(EFIVARS).await.unwrap_or_default() {
        return None;
    }
    match var("SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c").await {
        Ok(Some(1)) => {}
        // not there means firmware without Secure Boot
        Ok(Some(0) | None) => return Some(false),
        Ok(Some(_)) | Err(_) => return None,
    }
    // 1 when validation was turned off in MokManager
    match var("MokSBStateRT-605dab50-e046-4300-abb6-3dd810dd8b23").await {
        Ok(Some(1)) => Some(false),
        Ok(_) => Some(true),
        Err(_) => None,
    }
}

/// the chips of every NVIDIA GPU in `lspci`'s output, e.g. `GA106` from
/// `01:00.0 VGA compatible controller: NVIDIA Corporation GA106 [GeForce RTX 3060] (rev a1)`
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
        // a GPU newer than `pci.ids`
        let lspci = "01:00.0 VGA compatible controller: NVIDIA Corporation Device 2c05 (rev a1)";
        assert_eq!(nvidia_chipsets(lspci), ["Device"]);
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
        // nothing detected never means the latest driver
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
        // the oldest branch that drives both
        assert_eq!(driver(&["GP108", "TU116"]), NvidiaDriver::Branch(R580));
        assert_eq!(driver(&["AD104", "GP108"]), NvidiaDriver::Branch(R580));
    }

    #[test]
    fn nvidia_branch_packages() {
        assert_eq!(NvidiaBranch::Latest.packages()[0], "akmod-nvidia");
        assert!(NvidiaBranch::R580.packages().iter().all(|p| p.contains("-580xx")));
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

    const fn usb(vendor: u16, product: u16, class: [u8; 3]) -> UsbInterface {
        UsbInterface { vendor, product, class }
    }

    #[test]
    fn broadcom_bluetooth() {
        // BCM20702A0 and a Dell rebrand
        assert!(has_broadcom_bluetooth(&[], &[usb(0x0a5c, 0x21e8, [0xff, 0x01, 0x01])]));
        assert!(has_broadcom_bluetooth(&[], &[usb(0x413c, 0x8197, [0xff, 0x01, 0x01])]));
        assert!(has_broadcom_bluetooth(&[], &[usb(0x19ff, 0x0239, [0xe0, 0x01, 0x01])]));
        // a Foxconn MediaTek one is a standard Bluetooth interface, and Apple's need no firmware
        assert!(!has_broadcom_bluetooth(&[], &[usb(0x0489, 0xe0cd, [0xe0, 0x01, 0x01])]));
        assert!(!has_broadcom_bluetooth(&[], &[usb(0x05ac, 0x828f, [0xff, 0x01, 0x01])]));
        // the Broadcom Wi-Fi function isn't it
        assert!(!has_broadcom_bluetooth(&[dev(0x14e4, 0x43a0, 0x028000)], &[]));
        assert!(has_broadcom_bluetooth(&[dev(0x14e4, 0x4359, 0x0d1100)], &[]));
    }

    #[tokio::test]
    async fn reads_sysfs_usb_interface() {
        let dir = std::env::temp_dir().join(format!("taidan-usb-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("1-4:1.0")).unwrap();
        std::fs::create_dir_all(dir.join("1-4")).unwrap();
        std::fs::write(dir.join("1-4/idVendor"), "0a5c\n").unwrap();
        std::fs::write(dir.join("1-4/idProduct"), "21e8\n").unwrap();
        for (name, val) in
            [("bInterfaceClass", "ff"), ("bInterfaceSubClass", "01"), ("bInterfaceProtocol", "01")]
        {
            std::fs::write(dir.join("1-4:1.0").join(name), format!("{val}\n")).unwrap();
        }
        let read = UsbInterface::read(&dir, "1-4:1.0").await;
        let device = UsbInterface::read(&dir, "1-4").await;
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(read, Some(usb(0x0a5c, 0x21e8, [0xff, 0x01, 0x01])));
        assert_eq!(device, None);
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
