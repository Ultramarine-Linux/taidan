//! wi-fi and connectivity through the D-Bus API of `NetworkManager`
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use futures::{FutureExt as _, StreamExt as _};
use slint::{ComponentHandle, Model as _};
use zbus::blocking::Connection;
use zbus::proxy::{CacheProperties, Defaults};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

use crate::ui::{
    AppWindow, CaSource, EapMethod, EnterpriseLogin, HiddenSecurity, IpSettings, NetState, Network,
    Security, WifiNetwork,
};

/// how often the state and network list are refreshed while nothing else happens
const POLL: Duration = Duration::from_secs(3);
/// how often it looks while the network page isn't shown, where little shows the state (the
/// summary's updates switch), and the installation shouldn't compete with it
const POLL_ELSEWHERE: Duration = Duration::from_secs(10);
/// whether the network page is shown, see [`POLL_ELSEWHERE`]
static ON_PAGE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// how long a connection may take to come up
const CONNECT_TIMEOUT: Duration = Duration::from_secs(45);
/// how long scan results take to come in after asking for them
const SCAN_TIME: Duration = Duration::from_secs(3);
/// how often `NetworkManager` is asked to check connectivity while behind a captive portal, each
/// check is a request to the internet that can take a while
const PORTAL_CHECK: Duration = Duration::from_secs(10);

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
mod nm {
    use std::collections::HashMap;
    use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

    pub type ConnSettings<'a> = HashMap<&'static str, HashMap<&'static str, Value<'a>>>;

    #[zbus::proxy(
        interface = "org.freedesktop.NetworkManager",
        default_service = "org.freedesktop.NetworkManager",
        default_path = "/org/freedesktop/NetworkManager",
        gen_async = false,
        blocking_name = "NetworkManager"
    )]
    pub trait NetworkManager {
        fn get_devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
        fn activate_connection(
            &self,
            connection: &ObjectPath<'_>,
            device: &ObjectPath<'_>,
            specific_object: &ObjectPath<'_>,
        ) -> zbus::Result<OwnedObjectPath>;
        fn add_and_activate_connection(
            &self,
            connection: ConnSettings<'_>,
            device: &ObjectPath<'_>,
            specific_object: &ObjectPath<'_>,
        ) -> zbus::Result<(OwnedObjectPath, OwnedObjectPath)>;
        fn deactivate_connection(&self, active_connection: &ObjectPath<'_>) -> zbus::Result<()>;
        #[zbus(property)]
        fn state(&self) -> zbus::Result<u32>;
        #[zbus(property)]
        fn wireless_enabled(&self) -> zbus::Result<bool>;
        #[zbus(property)]
        fn set_wireless_enabled(&self, enabled: bool) -> zbus::Result<()>;
        #[zbus(property)]
        fn wireless_hardware_enabled(&self) -> zbus::Result<bool>;
        #[zbus(property)]
        fn connectivity(&self) -> zbus::Result<u32>;
        #[zbus(property)]
        fn connectivity_check_uri(&self) -> zbus::Result<String>;
        fn check_connectivity(&self) -> zbus::Result<u32>;
    }

    #[zbus::proxy(
        interface = "org.freedesktop.NetworkManager.Device",
        default_service = "org.freedesktop.NetworkManager",
        gen_async = false,
        blocking_name = "Device"
    )]
    pub trait Device {
        #[zbus(property)]
        fn device_type(&self) -> zbus::Result<u32>;
        #[zbus(property)]
        fn state(&self) -> zbus::Result<u32>;
        #[zbus(property)]
        fn state_reason(&self) -> zbus::Result<(u32, u32)>;
        #[zbus(property)]
        fn active_connection(&self) -> zbus::Result<OwnedObjectPath>;
        #[zbus(property)]
        fn available_connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
        #[zbus(property)]
        fn interface(&self) -> zbus::Result<String>;
    }

    #[zbus::proxy(
        interface = "org.freedesktop.NetworkManager.Device.Wireless",
        default_service = "org.freedesktop.NetworkManager",
        gen_async = false,
        blocking_name = "Wireless"
    )]
    pub trait Wireless {
        fn request_scan(&self, options: HashMap<&str, Value<'_>>) -> zbus::Result<()>;
        fn get_all_access_points(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
        #[zbus(property)]
        fn active_access_point(&self) -> zbus::Result<OwnedObjectPath>;
    }

    #[zbus::proxy(
        interface = "org.freedesktop.NetworkManager.Device.Wired",
        default_service = "org.freedesktop.NetworkManager",
        gen_async = false,
        blocking_name = "Wired"
    )]
    pub trait Wired {
        #[zbus(property)]
        fn perm_hw_address(&self) -> zbus::Result<String>;
    }

    #[zbus::proxy(
        interface = "org.freedesktop.NetworkManager.AccessPoint",
        default_service = "org.freedesktop.NetworkManager",
        gen_async = false,
        blocking_name = "AccessPoint"
    )]
    pub trait AccessPoint {
        #[zbus(property)]
        fn ssid(&self) -> zbus::Result<Vec<u8>>;
        #[zbus(property)]
        fn strength(&self) -> zbus::Result<u8>;
        #[zbus(property)]
        fn flags(&self) -> zbus::Result<u32>;
        #[zbus(property)]
        fn wpa_flags(&self) -> zbus::Result<u32>;
        #[zbus(property)]
        fn rsn_flags(&self) -> zbus::Result<u32>;
    }

    #[zbus::proxy(
        interface = "org.freedesktop.NetworkManager.Settings",
        default_service = "org.freedesktop.NetworkManager",
        default_path = "/org/freedesktop/NetworkManager/Settings",
        gen_async = false,
        blocking_name = "Settings"
    )]
    pub trait Settings {
        fn list_connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
        fn add_connection(&self, connection: ConnSettings<'_>) -> zbus::Result<OwnedObjectPath>;
    }

    #[zbus::proxy(
        interface = "org.freedesktop.NetworkManager.Settings.Connection",
        default_service = "org.freedesktop.NetworkManager",
        gen_async = false,
        blocking_name = "SettingsConnection"
    )]
    pub trait SettingsConnection {
        fn get_settings(&self) -> zbus::Result<HashMap<String, HashMap<String, OwnedValue>>>;
        fn get_secrets(
            &self,
            setting_name: &str,
        ) -> zbus::Result<HashMap<String, HashMap<String, OwnedValue>>>;
        fn update(
            &self,
            properties: HashMap<String, HashMap<String, OwnedValue>>,
        ) -> zbus::Result<()>;
        fn delete(&self) -> zbus::Result<()>;
    }

    #[zbus::proxy(
        interface = "org.freedesktop.NetworkManager.Connection.Active",
        default_service = "org.freedesktop.NetworkManager",
        gen_async = false,
        blocking_name = "ActiveConnection"
    )]
    pub trait ActiveConnection {
        #[zbus(property)]
        fn state(&self) -> zbus::Result<u32>;
        #[zbus(property)]
        fn connection(&self) -> zbus::Result<OwnedObjectPath>;
    }
}

// values from NetworkManager's D-Bus API docs
const STATE_CONNECTED_GLOBAL: u32 = 70;
const CONNECTIVITY_PORTAL: u32 = 2;
const CONNECTIVITY_FULL: u32 = 4;
const DEVICE_TYPE_ETHERNET: u32 = 1;
const DEVICE_TYPE_WIFI: u32 = 2;
/// has a carrier (a cable is plugged in), so a connection can be activated on it
const DEVICE_STATE_DISCONNECTED: u32 = 30;
/// connecting, from preparing up to being connected
const DEVICE_STATE_PREPARE: u32 = 40;
const DEVICE_STATE_ACTIVATED: u32 = 100;
const DEVICE_STATE_FAILED: u32 = 120;
const ACTIVE_STATE_ACTIVATED: u32 = 2;
const ACTIVE_STATE_DEACTIVATED: u32 = 4;
const AP_FLAGS_PRIVACY: u32 = 0x1;
const AP_SEC_KEY_MGMT_PSK: u32 = 0x100;
const AP_SEC_KEY_MGMT_802_1X: u32 = 0x200;
const AP_SEC_KEY_MGMT_SAE: u32 = 0x400;
const AP_SEC_KEY_MGMT_OWE: u32 = 0x800;
const AP_SEC_KEY_MGMT_EAP_SUITE_B_192: u32 = 0x2000;
/// device state reasons meaning the password (or other credentials) were wrong
const REASONS_BAD_SECRETS: [u32; 2] = [7, 8];

/// how a network wants clients to authenticate
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyMgmt {
    Open,
    Owe,
    Psk,
    Sae,
    Enterprise,
    /// WPA3-Enterprise 192-bit only, which needs its own key management and EAP-TLS
    EnterpriseSuiteB,
    /// old WEP security, which isn't supported
    Wep,
}

impl KeyMgmt {
    const fn from_flags(flags: u32, wpa: u32, rsn: u32) -> Self {
        let sec = wpa | rsn;
        if sec & AP_SEC_KEY_MGMT_802_1X != 0 {
            Self::Enterprise
        } else if sec & AP_SEC_KEY_MGMT_EAP_SUITE_B_192 != 0 {
            Self::EnterpriseSuiteB
        } else if sec & AP_SEC_KEY_MGMT_PSK != 0 {
            // also covers WPA2/WPA3 transition networks
            Self::Psk
        } else if sec & AP_SEC_KEY_MGMT_SAE != 0 {
            Self::Sae
        } else if sec & AP_SEC_KEY_MGMT_OWE != 0 {
            Self::Owe
        } else if flags & AP_FLAGS_PRIVACY != 0 {
            Self::Wep
        } else {
            Self::Open
        }
    }

    const fn security(self) -> Security {
        match self {
            Self::Open | Self::Owe => Security::Open,
            Self::Psk | Self::Sae => Security::Password,
            Self::Enterprise => Security::Enterprise,
            Self::EnterpriseSuiteB => Security::EnterpriseTls,
            Self::Wep => Security::Unsupported,
        }
    }
}

#[derive(Clone, Debug)]
struct Ap {
    /// as broadcast, which needn't be UTF-8
    ssid: Vec<u8>,
    /// the SSID as shown, and as the UI names the network
    name: String,
    strength: u8,
    key_mgmt: KeyMgmt,
    /// the adapter that sees it best, or is connected to it, which connecting uses
    device: OwnedObjectPath,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Snapshot {
    state: NetState,
    wifi: bool,
    radio: Radio,
    ethernet: bool,
    wired: bool,
    networks: Vec<WifiNetwork>,
}

/// whether the wi-fi radio is on
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Radio {
    #[default]
    On,
    Off,
    /// by a switch or key on the device, which only that can undo
    HardwareOff,
}

enum Cmd {
    Refresh,
    Connect {
        ssid: String,
        password: String,
        /// [`PAGE_VISIT`] when it was asked for
        visit: u64,
    },
    ConnectEnterprise {
        login: EnterpriseLogin,
        /// whether the dialog only offered TLS, as it does for WPA3-Enterprise 192-bit
        tls_only: bool,
        /// `Network.enterprise-visit` of the dialog it was sent from
        visit: i32,
    },
    /// a network that doesn't announce its name, so it isn't in the list
    ConnectHidden {
        ssid: String,
        key_mgmt: KeyMgmt,
        password: String,
    },
    SetProxy {
        url: String,
        bypass: String,
        /// whether to keep it after setup, see `Network.proxy-persist`
        persist: bool,
    },
    /// with the proxy and the hosts it's not used for, if one is set
    OpenPortal {
        proxy: Option<(String, String)>,
        /// [`PAGE_VISIT`] when it was asked for
        visit: u64,
    },
    ClosePortal,
    LoadIp(String),
    SaveIp(String, IpSettings),
    /// turns the wi-fi radio back on
    EnableWifi,
}

/// counts page changes, so a connection that fails after that doesn't
/// open its password or login dialog over a later page
static PAGE_VISIT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// why a connection failed, as a translation id
type Failure = &'static str;
const FAILED: Failure = "page-internet-error";
const FAILED_SECRETS: Failure = "page-internet-error-password";
const FAILED_PSK_LENGTH: Failure = "page-internet-error-password-length";
const FAILED_TIMEOUT: Failure = "page-internet-error-timeout";
const FAILED_IP: Failure = "page-internet-ip-error";
const PROXY_INVALID: Failure = "page-internet-proxy-invalid";
const PROXY_FAILED: Failure = "page-internet-proxy-error";
/// the browser for logging in to captive portals, a `WPE WebKit` shell
const PORTAL_BROWSER: &str = "/usr/bin/cog";
/// the user it runs as. Not taidan, which has the root helper while the OOBE runs, so a page
/// the network serves that gets past `WebKit` can't. `taidan-initial-setup.service` makes the user
/// and removes it once the OOBE is done
const PORTAL_USER: &str = "taidan-portal";
/// checked through the proxy if `NetworkManager` has no connectivity check URI of its own
const CHECK_URI: &str = "http://fedoraproject.org/static/hotspot.txt";

/// signal strength as the number of lit arcs in the wi-fi icon
const fn bars(strength: u8) -> i32 {
    match strength {
        75.. => 3,
        50..75 => 2,
        25..50 => 1,
        _ => 0,
    }
}

/// the browser opened for logging in to a captive portal, here rather than in [`Nm`] so
/// [`shutdown`] can close it
static PORTAL: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);

fn portal() -> std::sync::MutexGuard<'static, Option<std::process::Child>> {
    PORTAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// closes the captive portal browser, which would outlive taidan otherwise
pub fn shutdown() {
    SHUT_DOWN.store(true, std::sync::atomic::Ordering::SeqCst);
    close_portal();
}

/// set by [`shutdown`], so a request still queued doesn't open the portal browser as taidan exits
static SHUT_DOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

struct Nm {
    conn: Connection,
    manager: nm::NetworkManager<'static>,
    ui: slint::Weak<AppWindow>,
    /// access points from the last refresh, strongest per network
    aps: Vec<Ap>,
    /// how every network the list has shown is secured, by SSID, for one that's gone from it
    /// since
    seen: HashMap<Vec<u8>, KeyMgmt>,
    /// IP settings for wi-fi networks without a saved connection yet by SSID, used once connecting
    pending_ip: HashMap<Vec<u8>, IpConfig>,
    /// the last snapshot shown
    last: Option<Snapshot>,
    portal_open: bool,
    /// when `NetworkManager` last checked connectivity for the portal browser
    portal_checked: Option<Instant>,
    /// whether the internet wasn't fully working at some point since the portal browser opened.
    /// It's only closed on its own once that changes, since it's also there for portals
    /// `NetworkManager` misses, where it already says the internet works
    portal_offline: bool,
    /// when the scan the user asked for should be done
    scanning_until: Option<Instant>,
}

impl Nm {
    fn new(ui: slint::Weak<AppWindow>) -> zbus::Result<Self> {
        let conn = Connection::system()?;
        let nm = zbus::blocking::proxy::Builder::new(&conn)
            .cache_properties(CacheProperties::No)
            .build()?;
        Ok(Self {
            conn,
            manager: nm,
            ui,
            aps: vec![],
            seen: HashMap::new(),
            pending_ip: HashMap::new(),
            last: None,
            portal_open: false,
            portal_checked: None,
            portal_offline: false,
            scanning_until: None,
        })
    }

    /// the SSID of the network the UI calls `name`, which is lossy for SSIDs that aren't UTF-8
    fn ssid_of(&self, name: &str) -> Vec<u8> {
        self.aps
            .iter()
            .find(|ap| ap.name == name)
            .map_or_else(|| name.as_bytes().to_vec(), |ap| ap.ssid.clone())
    }

    /// shows what changed: the portal browser closing, the end of a scan, and the network state
    fn publish(&mut self) {
        if self.scanning_until.is_some_and(|until| Instant::now() >= until) {
            self.scanning_until = None;
            push(&self.ui, |net| net.set_scanning(false));
        }
        let open = self.check_portal();
        if open != self.portal_open {
            self.portal_open = open;
            push(&self.ui, move |net| net.set_portal_open(open));
        }
        let snap = self.snapshot();
        // replacing the model rebuilds every row, so only do it when something changed
        if self.last.as_ref() != Some(&snap) {
            self.last = Some(snap.clone());
            push_snapshot(&self.ui, snap);
        }
    }

    /// a proxy for the object at `path`, reading properties fresh each time
    fn at<'a, P>(&self, path: &'a ObjectPath<'a>) -> zbus::Result<P>
    where
        P: From<zbus::Proxy<'a>> + Defaults,
    {
        zbus::blocking::proxy::Builder::<P>::new(&self.conn)
            .path(path)?
            .cache_properties(CacheProperties::No)
            .build()
    }

    fn devices_of(&self, kind: u32) -> Vec<OwnedObjectPath> {
        let devices = self.manager.get_devices().unwrap_or_default();
        devices
            .into_iter()
            .filter(|path| {
                self.at::<nm::Device<'_>>(path)
                    .and_then(|d| d.device_type())
                    .is_ok_and(|t| t == kind)
            })
            .collect()
    }

    /// the wi-fi adapter to use, one that's connected or connecting, else one that can connect,
    /// so an unmanaged or blocked card doesn't hide a working one (e.g. a USB dongle)
    fn wifi_device(&self) -> Option<OwnedObjectPath> {
        self.best_device(DEVICE_TYPE_WIFI)
    }

    /// the device of `kind` in use, else one that could be (for ethernet, has a cable plugged in)
    fn best_device(&self, kind: u32) -> Option<OwnedObjectPath> {
        self.ranked_devices(kind).into_iter().next()
    }

    /// the devices of `kind`, the ones in use first, then the ones that could be
    fn ranked_devices(&self, kind: u32) -> Vec<OwnedObjectPath> {
        let rank = |path: &OwnedObjectPath| match self
            .at::<nm::Device<'_>>(path)
            .and_then(|d| d.state())
        {
            Ok(DEVICE_STATE_PREPARE..=DEVICE_STATE_ACTIVATED) => 0,
            Ok(s) if s >= DEVICE_STATE_DISCONNECTED => 1,
            _ => 2,
        };
        let mut devices = self.devices_of(kind);
        devices.sort_by_cached_key(rank);
        devices
    }

    /// the wi-fi adapter to connect to the network `ssid` with: the one the list got it from, else
    /// [`Self::wifi_device`]
    fn wifi_device_for(&self, ssid: &[u8]) -> Option<OwnedObjectPath> {
        (self.aps.iter().find(|ap| ap.ssid == ssid))
            .map(|ap| ap.device.clone())
            .or_else(|| self.wifi_device())
    }

    fn scan(&self) {
        // every adapter, the list shows what any of them sees
        for dev in self.devices_of(DEVICE_TYPE_WIFI) {
            // scans that come too soon after another are refused, which is fine
            if let Err(e) =
                self.at::<nm::Wireless<'_>>(&dev).and_then(|w| w.request_scan(HashMap::new()))
            {
                tracing::debug!(?e, ?dev, "cannot request wi-fi scan");
            }
        }
    }

    /// saved wi-fi connections by SSID, every one of them, as there can be more than one
    fn known(&self) -> HashMap<Vec<u8>, Vec<OwnedObjectPath>> {
        let settings: zbus::Result<nm::Settings<'_>> =
            zbus::blocking::proxy::Builder::new(&self.conn).build();
        let paths = settings.and_then(|s| s.list_connections()).unwrap_or_default();
        let mut known: HashMap<Vec<u8>, Vec<OwnedObjectPath>> = HashMap::new();
        for path in paths {
            let ssid = self
                .at::<nm::SettingsConnection<'_>>(&path)
                .and_then(|c| c.get_settings())
                .ok()
                .and_then(|mut s| s.get_mut("802-11-wireless")?.remove("ssid"))
                .and_then(|ssid| Vec::<u8>::try_from(ssid).ok());
            if let Some(ssid) = ssid {
                known.entry(ssid).or_default().push(path);
            }
        }
        known
    }

    /// the saved connection for `ssid` to use, the one that's up on some adapter, else the first
    fn known_one(&self, ssid: &[u8]) -> Option<OwnedObjectPath> {
        let saved = self.known().remove(ssid)?;
        let active = (self.devices_of(DEVICE_TYPE_WIFI).iter())
            .filter_map(|dev| self.active_on(dev))
            .find(|active| saved.contains(active));
        active.or_else(|| saved.into_iter().next())
    }

    /// the networks `devs` see, each once, and the ones they're connected to
    fn access_points(&self, devs: &[OwnedObjectPath]) -> (Vec<Ap>, Vec<Vec<u8>>) {
        let mut aps: Vec<Ap> = vec![];
        let mut active = vec![];
        for dev in devs {
            let Ok(wireless) = self.at::<nm::Wireless<'_>>(dev) else { continue };
            let read = |path: &ObjectPath<'_>| -> zbus::Result<Ap> {
                let ap = self.at::<nm::AccessPoint<'_>>(path)?;
                let ssid = ap.ssid()?;
                Ok(Ap {
                    name: String::from_utf8_lossy(&ssid).into_owned(),
                    ssid,
                    strength: ap.strength()?,
                    key_mgmt: KeyMgmt::from_flags(ap.flags()?, ap.wpa_flags()?, ap.rsn_flags()?),
                    device: dev.clone(),
                })
            };
            let on = wireless
                .active_access_point()
                .ok()
                .filter(|path| path.as_str() != "/")
                .and_then(|path| read(&path).ok())
                .map(|ap| ap.ssid);
            for path in wireless.get_all_access_points().unwrap_or_default() {
                // hidden networks have no name to show
                let Ok(ap) = read(&path) else { continue };
                if ap.ssid.is_empty() {
                    continue;
                }
                let connected = on.as_ref() == Some(&ap.ssid);
                match aps.iter_mut().find(|a| a.ssid == ap.ssid) {
                    // the adapter that's connected to it keeps it, so connecting again or
                    // changing its IP settings goes through the same one
                    Some(a)
                        if connected || (a.strength < ap.strength && !active.contains(&a.ssid)) =>
                    {
                        *a = ap;
                    }
                    Some(_) => {}
                    None => aps.push(ap),
                }
            }
            active.extend(on);
        }
        (aps, active)
    }

    fn snapshot(&mut self) -> Snapshot {
        let state = self.manager.state().unwrap_or_default();
        let portal = self.manager.connectivity().is_ok_and(|c| c == CONNECTIVITY_PORTAL);
        let state = match state {
            _ if portal => NetState::Portal,
            70 => NetState::Online,
            50 | 60 => NetState::Limited,
            40 => NetState::Connecting,
            _ => NetState::Offline,
        };
        let ethernet_devices = self.devices_of(DEVICE_TYPE_ETHERNET);
        let ethernet = !ethernet_devices.is_empty();
        let wired = ethernet_devices.iter().any(|path| {
            self.at::<nm::Device<'_>>(path)
                .and_then(|d| d.state())
                .is_ok_and(|s| s == DEVICE_STATE_ACTIVATED)
        });
        let radio = if !self.manager.wireless_hardware_enabled().unwrap_or(true) {
            Radio::HardwareOff
        } else if !self.manager.wireless_enabled().unwrap_or(true) {
            Radio::Off
        } else {
            Radio::On
        };
        let devs = self.ranked_devices(DEVICE_TYPE_WIFI);
        if devs.is_empty() {
            self.aps.clear();
            return Snapshot { state, wifi: false, radio, ethernet, wired, networks: vec![] };
        }
        let (aps, active) = self.access_points(&devs);
        let known = self.known();
        let mut networks = aps
            .iter()
            .map(|ap| WifiNetwork {
                ssid: ap.name.as_str().into(),
                strength: bars(ap.strength),
                security: ap.key_mgmt.security(),
                known: known.contains_key(&ap.ssid),
                connected: active.contains(&ap.ssid),
            })
            .collect::<Vec<_>>();
        // bucketed strength keeps the list from reshuffling on every small change
        networks.sort_by(|a, b| {
            (b.connected, b.strength).cmp(&(a.connected, a.strength)).then(a.ssid.cmp(&b.ssid))
        });
        self.seen.extend(aps.iter().map(|ap| (ap.ssid.clone(), ap.key_mgmt)));
        self.aps = aps;
        Snapshot { state, wifi: true, radio, ethernet, wired, networks }
    }

    fn delete(&self, path: &ObjectPath<'_>) {
        if let Err(e) = self.at::<nm::SettingsConnection<'_>>(path).and_then(|c| c.delete()) {
            tracing::warn!(?e, ?path, "cannot delete connection");
        }
    }

    /// state changes of `dev`, taken before activating so none are missed. A failed connection
    /// only stays in the failed state for a moment before the device moves on to disconnected,
    /// which forgets the reason, so polling the state hardly ever sees why it failed
    fn watch(&self, dev: &ObjectPath<'_>) -> Option<zbus::proxy::SignalStream<'static>> {
        let dev = self.at::<nm::Device<'_>>(dev).ok()?;
        zbus::block_on(dev.inner().inner().receive_signal("StateChanged"))
            .inspect_err(|e| tracing::warn!(?e, "cannot watch the device state"))
            .ok()
    }

    /// waits for `active` to come up on `dev`, showing the state meanwhile. `watch` is from
    /// [`Self::watch`] on `dev`
    fn wait(
        &mut self,
        active: &ObjectPath<'_>,
        dev: &ObjectPath<'_>,
        mut watch: Option<zbus::proxy::SignalStream<'static>>,
    ) -> Result<(), Failure> {
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        let active_path = active;
        let (active, dev) =
            (self.at::<nm::ActiveConnection<'_>>(active), self.at::<nm::Device<'_>>(dev));
        let mut reason = 0;
        // the reason of the last failure seen, without blocking
        let mut drain = |reason: &mut u32| {
            let Some(watch) = &mut watch else { return };
            while let Some(Some(msg)) = watch.next().now_or_never() {
                if let Ok((DEVICE_STATE_FAILED, _, r)) = msg.body().deserialize::<(u32, u32, u32)>()
                {
                    *reason = r;
                }
            }
        };
        let mut published = Instant::now();
        loop {
            drain(&mut reason);
            // in case the signal couldn't be subscribed to
            if let Ok(dev) = &dev
                && dev.state().is_ok_and(|s| s == DEVICE_STATE_FAILED)
                && let Ok((_, r)) = dev.state_reason()
            {
                reason = r;
            }
            // the active connection object vanishes once it's torn down
            match active.as_ref().map(nm::ActiveConnection::state) {
                Ok(Ok(ACTIVE_STATE_ACTIVATED)) => return Ok(()),
                Ok(Ok(ACTIVE_STATE_DEACTIVATED) | Err(_)) | Err(_) => break,
                Ok(Ok(_)) => {}
            }
            if Instant::now() > deadline {
                // so it doesn't come up later after all, while the UI says it failed
                if let Err(e) = self.manager.deactivate_connection(active_path) {
                    tracing::warn!(?e, "cannot stop the connection that took too long");
                }
                return Err(FAILED_TIMEOUT);
            }
            // the network list and the end of a scan, which would wait for this otherwise
            if published.elapsed() >= POLL {
                published = Instant::now();
                self.publish();
            }
            std::thread::sleep(Duration::from_millis(300));
        }
        // the device fails before the active connection goes down, so it should be in by now
        drain(&mut reason);
        if reason == 0
            && let Ok(dev) = &dev
        {
            reason = dev.state_reason().map_or(0, |(_, r)| r);
        }
        tracing::warn!(reason, "connection failed");
        Err(if REASONS_BAD_SECRETS.contains(&reason) { FAILED_SECRETS } else { FAILED })
    }

    /// the settings of the saved connection at `path`
    fn settings_of(
        &self,
        path: &ObjectPath<'_>,
    ) -> Option<HashMap<String, HashMap<String, OwnedValue>>> {
        self.at::<nm::SettingsConnection<'_>>(path).ok()?.get_settings().ok()
    }

    /// connects with new `settings`, replacing any saved connection for `ssid` but keeping its IP
    /// settings, and doesn't keep them if connecting fails, so a wrong password isn't saved. The
    /// old connection is only deleted once the new one works, so a failed attempt keeps it.
    fn add_and_connect(
        &mut self,
        ssid: &[u8],
        mut settings: nm::ConnSettings<'_>,
    ) -> Result<(), Failure> {
        let dev = self.wifi_device_for(ssid).ok_or(FAILED)?;
        let old = self.known().remove(ssid).unwrap_or_default();
        let was_active = self.active_on(&dev).filter(|active| old.contains(active));
        let ip = self.pending_ip.get(ssid).cloned().or_else(|| {
            // the IP settings of the one in use, else of any that has some
            let mut ordered = was_active.iter().chain(&old);
            ordered.find_map(|path| {
                let old = self.settings_of(path)?;
                Some(IpConfig::from_settings(&old)).filter(|ip| *ip != IpConfig::default())
            })
        });
        if let Some(ip) = ip {
            for (name, values) in ip.sections() {
                settings.entry(name).or_default().extend(values);
            }
        }
        let root = ObjectPath::from_static_str_unchecked("/");
        let watch = self.watch(&dev);
        let (conn, active) =
            self.manager.add_and_activate_connection(settings, &dev, &root).map_err(|e| {
                let ssid = String::from_utf8_lossy(ssid);
                tracing::error!(?e, %ssid, "cannot add connection");
                FAILED
            })?;
        let res = self.wait(&active, &dev, watch);
        if res.is_err() {
            self.delete(&conn);
            // go back to the network it was on before
            if let Some(old) = &was_active
                && let Err(e) = self.manager.activate_connection(old, &dev, &root)
            {
                tracing::warn!(?e, "cannot reactivate the previous connection");
            }
        } else {
            // in the saved connection now, where later changes go
            self.pending_ip.remove(ssid);
            // every earlier one, so the network doesn't end up saved twice
            for old in old {
                self.delete(&old);
            }
        }
        res
    }

    /// connects to a network from the list, `name` as the UI shows it
    fn connect(&mut self, name: &str, password: &str) -> Result<(), Failure> {
        let ssid = self.ssid_of(name);
        let dev = self.wifi_device_for(&ssid).ok_or(FAILED)?;
        let saved = self.known_one(&ssid);
        if password.is_empty()
            && let Some(saved) = &saved
        {
            let root = ObjectPath::from_static_str_unchecked("/");
            let watch = self.watch(&dev);
            let active = self.manager.activate_connection(saved, &dev, &root).map_err(|e| {
                tracing::error!(?e, name, "cannot activate connection");
                FAILED
            })?;
            return self.wait(&active, &dev, watch);
        }
        let saved = saved.and_then(|path| saved_security(&self.settings_of(&path)?));
        let (key_mgmt, hidden) = match self.aps.iter().find(|ap| ap.ssid == ssid) {
            // a hidden network that shows while connected stays hidden, or it'd stop connecting
            // once it's out of the scan again
            Some(ap) => (ap.key_mgmt, saved.is_some_and(|(_, hidden)| hidden)),
            // gone from the scan since it was picked, so go by how it was saved
            None => saved.ok_or_else(|| {
                tracing::warn!(name, "network is gone");
                FAILED
            })?,
        };
        check_psk(key_mgmt, password)?;
        let mut settings = wifi_settings(&ssid, name, key_mgmt, password);
        if hidden {
            mark_hidden(&mut settings);
        }
        self.add_and_connect(&ssid, settings)
    }

    /// connects to a network that doesn't announce its name, which `NetworkManager` then probes for
    fn connect_hidden(
        &mut self,
        ssid: &str,
        key_mgmt: KeyMgmt,
        password: &str,
    ) -> Result<(), Failure> {
        check_psk(key_mgmt, password)?;
        let mut settings = wifi_settings(ssid.as_bytes(), ssid, key_mgmt, password);
        mark_hidden(&mut settings);
        self.add_and_connect(ssid.as_bytes(), settings)
    }
}

/// `NetworkManager` rejects a WPA-PSK key that isn't 8 to 63 characters or 64 hex digits, so
/// catch it before adding the connection. SAE passwords have no such limit.
fn check_psk(key_mgmt: KeyMgmt, password: &str) -> Result<(), Failure> {
    let valid = match password.len() {
        8..=63 => true,
        64 => password.bytes().all(|b| b.is_ascii_hexdigit()),
        _ => false,
    };
    if key_mgmt == KeyMgmt::Psk && !valid {
        return Err(FAILED_PSK_LENGTH);
    }
    Ok(())
}

/// how a saved wi-fi connection with a password (or none) is secured, and whether it's for a
/// hidden network
fn saved_security(
    settings: &HashMap<String, HashMap<String, OwnedValue>>,
) -> Option<(KeyMgmt, bool)> {
    let get = |section: &str, key: &str| settings.get(section)?.get(key);
    let key_mgmt = match get("802-11-wireless-security", "key-mgmt") {
        None => KeyMgmt::Open,
        Some(v) => match v.downcast_ref::<&str>().ok()? {
            "wpa-psk" => KeyMgmt::Psk,
            "sae" => KeyMgmt::Sae,
            "owe" => KeyMgmt::Owe,
            // 802.1X logins go through their own dialog, and WEP isn't supported
            _ => return None,
        },
    };
    let hidden =
        get("802-11-wireless", "hidden").is_some_and(|h| h.downcast_ref::<bool>() == Ok(true));
    Some((key_mgmt, hidden))
}

/// settings for the wi-fi network `ssid`, named `name`
fn wifi_settings<'a>(
    ssid: &[u8],
    name: &'a str,
    key_mgmt: KeyMgmt,
    password: &'a str,
) -> nm::ConnSettings<'a> {
    let mut settings = base_settings(ssid, name);
    let security: &[(&str, Value<'_>)] = match key_mgmt {
        KeyMgmt::Psk => &[("key-mgmt", "wpa-psk".into()), ("psk", password.into())],
        KeyMgmt::Sae => &[("key-mgmt", "sae".into()), ("psk", password.into())],
        KeyMgmt::Owe => &[("key-mgmt", "owe".into())],
        KeyMgmt::Open | KeyMgmt::Enterprise | KeyMgmt::EnterpriseSuiteB | KeyMgmt::Wep => &[],
    };
    if !security.is_empty() {
        settings.insert("802-11-wireless-security", security.iter().cloned().collect());
    }
    settings
}

fn base_settings<'a>(ssid: &[u8], name: &'a str) -> nm::ConnSettings<'a> {
    HashMap::from([
        ("connection", HashMap::from([("id", name.into()), ("type", "802-11-wireless".into())])),
        (
            "802-11-wireless",
            HashMap::from([
                ("ssid", Value::from(ssid.to_vec())),
                ("mode", "infrastructure".into()),
            ]),
        ),
    ])
}

/// tells `NetworkManager` to look for the network even though it doesn't announce its name
fn mark_hidden(settings: &mut nm::ConnSettings<'_>) {
    if let Some(wifi) = settings.get_mut("802-11-wireless") {
        wifi.insert("hidden", true.into());
    }
}

/// name of the wired connection taidan makes for 802.1X, so a retry replaces it
const WIRED_8021X_ID: &str = "Wired 802.1X";

/// the part of `username` after the `@`, which for eduroam is the user's institution
fn realm(username: &str) -> &str {
    username.rsplit_once('@').map_or("", |(_, realm)| realm.trim())
}

/// settings for an 802.1X login, on the wi-fi network `ssid` unless it's wired, `hidden` for one
/// that doesn't announce its name, `key_mgmt` as the network announces it
fn enterprise_settings<'a>(
    login: &'a EnterpriseLogin,
    ssid: &[u8],
    hidden: bool,
    key_mgmt: KeyMgmt,
) -> nm::ConnSettings<'a> {
    let mut settings = if login.wired {
        HashMap::from([
            (
                "connection",
                HashMap::from([
                    ("id", WIRED_8021X_ID.into()),
                    ("type", "802-3-ethernet".into()),
                    // over the default wired connection, which has no login
                    ("autoconnect-priority", Value::from(10i32)),
                ]),
            ),
            ("802-3-ethernet", HashMap::new()),
        ])
    } else {
        let mut settings = base_settings(ssid, &login.ssid);
        if hidden {
            mark_hidden(&mut settings);
        }
        // `wpa-eap` only tries Suite-B-192 with PMF required, which this one turns on itself
        let key_mgmt =
            if key_mgmt == KeyMgmt::EnterpriseSuiteB { "wpa-eap-suite-b-192" } else { "wpa-eap" };
        settings.insert("802-11-wireless-security", HashMap::from([("key-mgmt", key_mgmt.into())]));
        settings
    };
    settings.insert("802-1x", eap_settings(login));
    settings
}

/// a file for `NetworkManager`'s certificate and key properties, which take the path as bytes
fn cert_path(path: &str) -> Value<'static> {
    Value::from(format!("file://{path}\0").into_bytes())
}

/// the `802-1x` setting, as `NetworkManager`'s connection editor would write it
fn eap_settings(login: &EnterpriseLogin) -> HashMap<&'static str, Value<'_>> {
    let method = match login.method {
        EapMethod::Peap => "peap",
        EapMethod::Ttls => "ttls",
        EapMethod::Tls => "tls",
        EapMethod::Fast => "fast",
        EapMethod::Pwd => "pwd",
        EapMethod::Leap => "leap",
        EapMethod::Md5 => "md5",
    };
    let mut x = HashMap::from([
        ("eap", Value::from(vec![method])),
        ("identity", login.username.as_str().into()),
    ]);
    if login.method != EapMethod::Tls {
        x.insert("password", login.password.as_str().into());
    }
    if matches!(login.method, EapMethod::Peap | EapMethod::Ttls | EapMethod::Fast) {
        if !login.anonymous_identity.is_empty() {
            // keeps the username private from anyone but the user's own institution
            x.insert("anonymous-identity", login.anonymous_identity.as_str().into());
        }
        // TTLS can run its inner method over EAP or not, the editor's "MSCHAPv2 (no EAP)"
        let (key, inner) = match (login.method, login.inner.as_str()) {
            (EapMethod::Ttls, "mschapv2-plain") => ("phase2-auth", "mschapv2"),
            (EapMethod::Ttls, inner @ ("mschapv2" | "md5" | "gtc")) => ("phase2-autheap", inner),
            (_, inner) => ("phase2-auth", inner),
        };
        x.insert(key, inner.into());
    }
    if login.method == EapMethod::Peap && matches!(login.peap_version, 1 | 2) {
        x.insert("phase1-peapver", (login.peap_version - 1).to_string().into());
    }
    if matches!(login.method, EapMethod::Peap | EapMethod::Ttls | EapMethod::Tls) {
        match login.ca {
            CaSource::System => {
                x.insert("system-ca-certs", true.into());
            }
            CaSource::File => {
                x.insert("ca-cert", cert_path(&login.ca_cert));
            }
            CaSource::None => {}
        }
        if login.ca != CaSource::None && !login.domain.is_empty() {
            // only log in to a server with a valid certificate for this domain
            x.insert("domain-suffix-match", login.domain.as_str().into());
        }
    }
    if login.method == EapMethod::Tls {
        x.insert("client-cert", cert_path(&login.client_cert));
        // PKCS#12 files carry the key, NetworkManager wants them as both
        let key =
            if login.private_key.is_empty() { &login.client_cert } else { &login.private_key };
        x.insert("private-key", cert_path(key));
        if login.private_key_password.is_empty() {
            // an unencrypted key has no password, NM_SETTING_SECRET_FLAG_NOT_REQUIRED says so,
            // or NetworkManager fails with no secrets, which reads as a wrong password
            x.insert("private-key-password-flags", Value::from(4u32));
        } else {
            x.insert("private-key-password", login.private_key_password.as_str().into());
        }
    }
    if login.method == EapMethod::Fast {
        x.insert("phase1-fast-provisioning", login.fast_provisioning.to_string().into());
        if !login.pac_file.is_empty() {
            x.insert("pac-file", login.pac_file.as_str().into());
        }
    }
    x
}

/// where certificates for 802.1X are kept, so they still work once the USB stick is gone
use libtaidan::helper::CERT_DIR;

fn is_pkcs12(path: &str) -> bool {
    let ext = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or_default();
    ext.eq_ignore_ascii_case("p12") || ext.eq_ignore_ascii_case("pfx")
}

/// what the certificate files of a network start with
/// its name for people to recognize plus the SSID in hex, so two SSIDs that read the same as a
/// hostname (e.g. `Café`, `Caf` or `東京`, `大阪`) don't overwrite each other's certs
fn cert_name(wired: bool, ssid: &str) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    if wired {
        return "wired".to_owned();
    }
    let hex: String = ssid
        .bytes()
        .flat_map(|b| [DIGITS[usize::from(b >> 4)], DIGITS[usize::from(b & 0xf)]])
        .map(char::from)
        .collect();
    match derive_hostname(ssid) {
        readable if readable.is_empty() => format!("wifi-{hex}"),
        readable => format!("{readable}-{hex}"),
    }
}

/// certificate files copied for one login attempt, see [`install_certs`]
struct Certs {
    /// the network's folder in [`CERT_DIR`], with one folder per attempt inside
    network: String,
    /// the folder of this attempt in it, if it copied anything
    attempt: Option<String>,
}

impl Certs {
    /// keeps this attempt's files and removes those of earlier ones once it `worked`, or else
    /// removes this attempt's, leaving the ones the network's saved connection still uses
    fn finish(&self, worked: bool) {
        let network = self.network.clone();
        let verb = if worked {
            if !std::path::Path::new(CERT_DIR).join(&self.network).is_dir() {
                return;
            }
            libtaidan::Verb::PruneCerts { network, keep: self.attempt.clone() }
        } else if let Some(attempt) = &self.attempt {
            libtaidan::Verb::RemoveCertAttempt { network, attempt: attempt.clone() }
        } else {
            return;
        };
        if let Err(e) = crate::RT.block_on(libtaidan::root(verb)) {
            tracing::warn!(?e, network = self.network, "cannot remove old certificates");
        }
    }
}

/// copies the files `login` uses to a new folder in [`CERT_DIR`] for this attempt, readable only
/// by root, and points it at them. A new folder each time, so a failed attempt can't overwrite the
/// files the network's working connection uses
fn install_certs(login: &mut EnterpriseLogin) -> Result<Certs, Failure> {
    let network = cert_name(login.wired, &login.ssid);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string();
    let mut certs = Certs { network: network.clone(), attempt: None };
    let files = [
        ("ca", &mut login.ca_cert),
        ("client", &mut login.client_cert),
        ("key", &mut login.private_key),
        ("pac", &mut login.pac_file),
    ];
    for (kind, path) in files {
        // copied even from `CERT_DIR`, as that file goes once this attempt works
        if path.is_empty() {
            continue;
        }
        let ext = std::path::Path::new(path.as_str())
            .extension()
            .and_then(|e| e.to_str())
            // the helper only takes a short ASCII one, the rest is what the file's in it anyway
            .filter(|e| !e.is_empty() && e.len() <= 8 && e.chars().all(|c| c.is_ascii_alphanumeric()))
            .map(|e| format!(".{}", e.to_ascii_lowercase()))
            .unwrap_or_default();
        let file = format!("{kind}{ext}");
        let dest = format!("{CERT_DIR}/{network}/{stamp}/{file}");
        certs.attempt = Some(stamp.clone());
        // read here, as taidan, so it can only be a file taidan may read anyway
        let res =
            std::fs::read(path.as_str()).map_err(libtaidan::prelude::Err::Io).and_then(|content| {
                let verb = libtaidan::Verb::InstallCert {
                    network: network.clone(),
                    attempt: stamp.clone(),
                    file: file.clone(),
                };
                crate::RT.block_on(libtaidan::root_with_stdin(verb, &content))
            });
        if let Err(e) = res {
            tracing::error!(?e, %path, dest, "cannot copy certificate");
            certs.finish(false);
            return Err("page-internet-8021x-error-file");
        }
        *path = dest.into();
    }
    Ok(certs)
}

/// certificate files, as offered by `NetworkManager`'s connection editor
const CERT_EXTS: [&str; 7] = ["der", "pem", "crt", "cer", "p12", "pfx", "key"];

/// what's in `dir` for the file browser: folders, then files with the extensions it wants
fn list_dir(dir: &std::path::Path, pac: bool, all: bool) -> Vec<crate::ui::FileEntry> {
    let Ok(read) = std::fs::read_dir(dir) else { return vec![] };
    let mut entries = read
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            // follows symlinks, so linked folders can be opened
            let is_dir = entry.path().is_dir();
            let ext =
                entry.path().extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
            let wanted = all
                || ext.is_some_and(
                    |e| if pac { e == "pac" } else { CERT_EXTS.contains(&e.as_str()) },
                );
            (is_dir || wanted).then(|| crate::ui::FileEntry {
                name: name.into(),
                path: entry.path().to_string_lossy().as_ref().into(),
                dir: is_dir,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by_cached_key(|e| (!e.dir, e.name.to_lowercase()));
    entries
}

/// places to start browsing from: removable drives, home, and the whole computer
fn browse_places() -> Vec<crate::ui::FileEntry> {
    let mut places = vec![];
    if let Some(user) = uzers::get_current_username() {
        let media = std::path::Path::new("/run/media").join(user);
        if media.is_dir() {
            places.push(("page-internet-files-drives", media));
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from)
        && home.is_dir()
    {
        places.push(("page-internet-files-home", home));
    }
    places.push(("page-internet-files-root", "/".into()));
    places
        .into_iter()
        .map(|(name, path)| crate::ui::FileEntry {
            name: name.into(),
            path: path.to_string_lossy().as_ref().into(),
            dir: true,
        })
        .collect()
}

/// checks the internet is reachable through the proxy, then uses it
fn set_proxy(nm: &Nm, ui: &slint::Weak<AppWindow>, url: &str, bypass: &str, persist: bool) {
    let res = if url.is_empty() {
        Ok(None)
    } else {
        libtaidan::proxy::Proxy::new(url, bypass).ok_or(PROXY_INVALID).and_then(|proxy| {
            let uri = nm
                .manager
                .connectivity_check_uri()
                .ok()
                .filter(|uri| !uri.is_empty())
                .unwrap_or_else(|| CHECK_URI.to_owned());
            let client = proxy.client().map_err(|_| PROXY_INVALID)?;
            let res = crate::RT.block_on(async {
                client.get(&uri).timeout(Duration::from_secs(15)).send().await?.error_for_status()
            });
            res.map(|_| Some(proxy)).map_err(|e| {
                tracing::warn!(?e, uri, "cannot reach the internet through the proxy");
                PROXY_FAILED
            })
        })
    };
    push(ui, move |net| {
        net.set_proxy_busy(false);
        // cancelled while it was checked, the user gave up on it
        if !net.get_proxy_open() {
            return;
        }
        match res {
            Ok(proxy) => {
                let public = proxy.as_ref().map(libtaidan::proxy::Proxy::public_url);
                net.set_proxy_public(public.unwrap_or_default().into());
                let (url, bypass) = proxy.map(|p| (p.url, p.no_proxy)).unwrap_or_default();
                net.set_proxy_url(url.into());
                net.set_proxy_persist(persist);
                net.set_proxy_bypass(bypass.replace(',', ", ").into());
                net.set_proxy_open(false);
            }
            Err(failure) => net.set_proxy_error_id(failure.into()),
        }
    });
}

/// IPv4 address and DNS servers of a connection, IPv6 stays automatic apart from DNS
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct IpConfig {
    /// address, prefix length, and gateway, or `None` for DHCP
    manual: Option<(Ipv4Addr, u32, Option<Ipv4Addr>)>,
    dns4: Vec<Ipv4Addr>,
    dns6: Vec<Ipv6Addr>,
}

/// keys replaced in the `ipv4` and `ipv6` settings, including the deprecated forms
const IP_KEYS: [&str; 7] =
    ["method", "addresses", "address-data", "gateway", "dns", "dns-data", "ignore-auto-dns"];

impl IpConfig {
    fn parse(ip: &IpSettings) -> Result<Self, Failure> {
        let mut cfg = Self::default();
        for server in ip.dns.split([',', ' ']).map(str::trim).filter(|s| !s.is_empty()) {
            match server.parse() {
                Ok(IpAddr::V4(addr)) => cfg.dns4.push(addr),
                Ok(IpAddr::V6(addr)) => cfg.dns6.push(addr),
                Err(_) => return Err("page-internet-ip-error-dns"),
            }
        }
        if !ip.manual {
            return Ok(cfg);
        }
        let (addr, prefix) =
            ip.address.trim().split_once('/').unwrap_or_else(|| (ip.address.trim(), "24"));
        let addr = addr.parse().map_err(|_| "page-internet-ip-error-address")?;
        let prefix = (prefix.parse().ok())
            .filter(|p| (1..=32).contains(p))
            .ok_or("page-internet-ip-error-address")?;
        let gateway = match ip.gateway.trim() {
            "" => None,
            gw => Some(gw.parse().map_err(|_| "page-internet-ip-error-gateway")?),
        };
        // without a DHCP server, nothing else would tell the system where to look up names
        if cfg.dns4.is_empty() && cfg.dns6.is_empty() {
            return Err("page-internet-ip-error-dns-needed");
        }
        cfg.manual = Some((addr, prefix, gateway));
        Ok(cfg)
    }

    /// as shown in the dialog
    fn display(&self) -> IpSettings {
        let dns = (self.dns4.iter().map(ToString::to_string))
            .chain(self.dns6.iter().map(ToString::to_string))
            .collect::<Vec<_>>()
            .join(", ");
        let Some((addr, prefix, gateway)) = self.manual else {
            return IpSettings { dns: dns.into(), ..Default::default() };
        };
        IpSettings {
            manual: true,
            address: format!("{addr}/{prefix}").into(),
            gateway: gateway.map(|gw| gw.to_string()).unwrap_or_default().into(),
            dns: dns.into(),
        }
    }

    /// values for the `ipv4` and `ipv6` settings, to put in place of [`IP_KEYS`]
    fn sections(&self) -> [(&'static str, HashMap<&'static str, Value<'static>>); 2] {
        let custom_dns = !self.dns4.is_empty() || !self.dns6.is_empty();
        let mut v4 = HashMap::from([("ignore-auto-dns", Value::from(custom_dns))]);
        if let Some((addr, prefix, gateway)) = self.manual {
            v4.insert("method", "manual".into());
            let address = HashMap::from([
                ("address", Value::from(addr.to_string())),
                ("prefix", Value::from(prefix)),
            ]);
            v4.insert("address-data", Value::from(vec![address]));
            if let Some(gw) = gateway {
                v4.insert("gateway", gw.to_string().into());
            }
        } else {
            v4.insert("method", "auto".into());
        }
        // NetworkManager wants these as integers in network byte order
        let dns4 = self.dns4.iter().map(|a| u32::from_ne_bytes(a.octets())).collect::<Vec<_>>();
        v4.insert("dns", dns4.into());
        let dns6 = self.dns6.iter().map(|a| a.octets().to_vec()).collect::<Vec<_>>();
        let v6 = HashMap::from([
            ("method", "auto".into()),
            ("ignore-auto-dns", Value::from(custom_dns)),
            ("dns", dns6.into()),
        ]);
        [("ipv4", v4), ("ipv6", v6)]
    }

    /// reads the settings of a saved connection
    fn from_settings(settings: &HashMap<String, HashMap<String, OwnedValue>>) -> Self {
        let mut cfg = Self::default();
        let v4 = settings.get("ipv4");
        let get = |key| v4.and_then(|v4| v4.get(key));
        let dns = |section: Option<&HashMap<String, OwnedValue>>| {
            let section = section?;
            // newer NetworkManager has the servers as strings, maybe with a `#name` for DoT
            if let Some(data) = section.get("dns-data") {
                let data = Vec::<String>::try_from(data.try_clone().ok()?).ok()?;
                return Some(
                    data.iter().filter_map(|s| s.split('#').next()?.parse().ok()).collect(),
                );
            }
            let raw = section.get("dns")?.try_clone().ok()?;
            if let Ok(v4) = Vec::<u32>::try_from(raw.try_clone().ok()?) {
                return Some(v4.into_iter().map(|n| IpAddr::from(n.to_ne_bytes())).collect());
            }
            let v6 = Vec::<Vec<u8>>::try_from(raw).ok()?;
            Some(
                v6.into_iter()
                    .filter_map(|b| <[u8; 16]>::try_from(b).ok().map(IpAddr::from))
                    .collect::<Vec<_>>(),
            )
        };
        let servers: Vec<IpAddr> = (dns(v4).into_iter().flatten())
            .chain(dns(settings.get("ipv6")).into_iter().flatten())
            .collect();
        for server in servers {
            match server {
                IpAddr::V4(a) => cfg.dns4.push(a),
                IpAddr::V6(a) => cfg.dns6.push(a),
            }
        }
        if get("method").and_then(|m| m.downcast_ref::<&str>().ok()) != Some("manual") {
            return cfg;
        }
        let first = get("address-data")
            .and_then(|v| v.try_clone().ok())
            .and_then(|v| Vec::<HashMap<String, OwnedValue>>::try_from(v).ok())
            .and_then(|addrs| addrs.into_iter().next());
        let Some(first) = first else { return cfg };
        let addr = first.get("address").and_then(|a| a.downcast_ref::<&str>().ok()?.parse().ok());
        let prefix = first.get("prefix").and_then(|p| p.downcast_ref::<u32>().ok());
        let gateway = get("gateway").and_then(|g| g.downcast_ref::<&str>().ok()?.parse().ok());
        if let (Some(addr), Some(prefix)) = (addr, prefix) {
            cfg.manual = Some((addr, prefix, gateway));
        }
        cfg
    }
}

impl Nm {
    /// opens a browser at the connectivity check page, which a captive portal redirects to its
    /// login, returns whether it's open
    fn open_portal(&mut self, proxy: Option<(String, String)>) -> bool {
        if SHUT_DOWN.load(std::sync::atomic::Ordering::SeqCst) {
            return false;
        }
        if portal().is_some() {
            return true;
        }
        let uri = (self.manager.connectivity_check_uri().ok())
            .filter(|uri| !uri.is_empty())
            .unwrap_or_else(|| CHECK_URI.to_owned());
        let Some((mut cmd, request)) = portal_command(proxy, &uri) else { return false };
        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write as _;
                    // a failed write shows up as the helper refusing, and the browser not opening
                    if let Err(e) = stdin.write_all(&request) {
                        tracing::error!(?e, "cannot pass the request to the helper");
                    }
                }
                tracing::info!(uri, "Opened the captive portal browser");
                {
                    let mut slot = portal();
                    *slot = Some(child);
                    // `shutdown` sets the flag before it takes the lock, so either it sees the
                    // browser here or this sees the flag and closes it the same way
                    if SHUT_DOWN.load(std::sync::atomic::Ordering::SeqCst) {
                        drop(slot);
                        close_portal();
                        return false;
                    }
                }
                self.portal_checked = None;
                self.portal_offline = self.manager.connectivity() != Ok(CONNECTIVITY_FULL);
                true
            }
            Err(e) => {
                tracing::error!(?e, "cannot open {PORTAL_BROWSER} for the captive portal");
                false
            }
        }
    }

    /// whether the portal browser is still open, it's closed once the internet works. Connectivity
    /// keeps being rechecked while behind the portal, even with the browser closed, since it's
    /// often closed right after logging in
    fn check_portal(&mut self) -> bool {
        let closed = {
            let mut portal = portal();
            let exited =
                (portal.as_mut()).is_some_and(|child| !matches!(child.try_wait(), Ok(None)));
            if exited {
                *portal = None;
            }
            exited
        };
        let open = portal().is_some();
        let behind_portal = self.manager.connectivity().is_ok_and(|c| c == CONNECTIVITY_PORTAL);
        if !open && !behind_portal {
            self.portal_checked = None;
            return false;
        }
        // right away once the browser closes, the login most likely just went through
        if !closed && self.portal_checked.is_some_and(|at| at.elapsed() < PORTAL_CHECK) {
            return open;
        }
        self.portal_checked = Some(Instant::now());
        // NetworkManager only rechecks every few minutes on its own, which is far too slow to notice
        // the login going through
        let online = match self.manager.check_connectivity() {
            Ok(connectivity) => connectivity == CONNECTIVITY_FULL,
            Err(e) => {
                tracing::debug!(?e, "cannot ask NetworkManager to check connectivity");
                self.manager.state().is_ok_and(|s| s == STATE_CONNECTED_GLOBAL)
            }
        };
        if !online {
            self.portal_offline = true;
        }
        // opened by hand while `NetworkManager` already said the internet works, so it isn't
        // closed until the internet stops and comes back
        if online && open && self.portal_offline {
            tracing::info!("Online, closing the captive portal browser");
            close_portal();
        }
        portal().is_some()
    }

    /// logs in to the wired network with 802.1X, replacing an earlier attempt once it works, and
    /// doesn't keep the connection if it fails, so a wrong password isn't saved and the earlier
    /// login still works
    fn connect_wired_8021x(&mut self, login: &EnterpriseLogin) -> Result<(), Failure> {
        let dev = self.ethernet_device().ok_or(FAILED)?;
        let settings: zbus::Result<nm::Settings<'_>> =
            zbus::blocking::proxy::Builder::new(&self.conn).build();
        let old = (settings.and_then(|s| s.list_connections()).unwrap_or_default().into_iter())
            .filter(|path| {
                let id = self
                    .at::<nm::SettingsConnection<'_>>(path)
                    .and_then(|c| c.get_settings())
                    .ok()
                    .and_then(|mut s| s.get_mut("connection")?.remove("id"))
                    .and_then(|id| String::try_from(id).ok());
                id.as_deref() == Some(WIRED_8021X_ID)
            })
            .collect::<Vec<_>>();
        let was_active = self.active_on(&dev).filter(|active| old.contains(active));
        let root = ObjectPath::from_static_str_unchecked("/");
        let watch = self.watch(&dev);
        let settings = enterprise_settings(login, &[], false, KeyMgmt::Enterprise);
        let (conn, active) =
            self.manager.add_and_activate_connection(settings, &dev, &root).map_err(|e| {
                tracing::error!(?e, "cannot add wired 802.1X connection");
                FAILED
            })?;
        let res = self.wait(&active, &dev, watch);
        if res.is_err() {
            self.delete(&conn);
            // go back to the login that worked before
            if let Some(old) = was_active
                && let Err(e) = self.manager.activate_connection(&old, &dev, &root)
            {
                tracing::warn!(?e, "cannot reactivate the previous wired 802.1X connection");
            }
        } else {
            for old in old {
                self.delete(&old);
            }
        }
        res
    }

    fn ethernet_device(&self) -> Option<OwnedObjectPath> {
        // e.g. a dock's rather than the laptop's own when the cable is in the dock
        self.best_device(DEVICE_TYPE_ETHERNET)
    }

    /// the saved connection active on `dev`, if any
    fn active_on(&self, dev: &ObjectPath<'_>) -> Option<OwnedObjectPath> {
        let active = self.at::<nm::Device<'_>>(dev).ok()?.active_connection().ok()?;
        if active.as_str() == "/" {
            return None;
        }
        self.at::<nm::ActiveConnection<'_>>(&active).ok()?.connection().ok()
    }

    /// the saved connection for the wired adapter, the active one or else one it could use
    fn ethernet_connection(&self, dev: &ObjectPath<'_>) -> Option<OwnedObjectPath> {
        let device = self.at::<nm::Device<'_>>(dev).ok()?;
        self.active_on(dev)
            .or_else(|| device.available_connections().ok()?.into_iter().next())
            .or_else(|| {
                // without a cable none are available, so look through the saved ones, else
                // saving here again would add another
                let iface = device.interface().ok()?;
                let mac = self
                    .at::<nm::Wired<'_>>(dev)
                    .and_then(|w| w.perm_hw_address())
                    .ok()
                    .and_then(|a| parse_mac(&a));
                let settings: nm::Settings<'_> =
                    zbus::blocking::proxy::Builder::new(&self.conn).build().ok()?;
                settings.list_connections().ok()?.into_iter().find(|path| {
                    let Some(settings) = self
                        .at::<nm::SettingsConnection<'_>>(path)
                        .ok()
                        .and_then(|c| c.get_settings().ok())
                    else {
                        return false;
                    };
                    let Some(connection) = settings.get("connection") else { return false };
                    let text = |key: &str| {
                        connection.get(key).and_then(|v| String::try_from(v.clone()).ok())
                    };
                    // a port of a bond or bridge has its IP settings on the bond or bridge
                    let port = ["controller", "master"]
                        .iter()
                        .any(|key| text(key).is_some_and(|c| !c.is_empty()));
                    // one bound to another card by its address isn't this one's
                    let other_card = (settings.get("802-3-ethernet"))
                        .and_then(|e| e.get("mac-address"))
                        .and_then(|v| Vec::<u8>::try_from(v.clone()).ok())
                        .is_some_and(|m| {
                            !m.is_empty() && mac.as_ref().is_some_and(|mac| *mac != m)
                        });
                    text("type").as_deref() == Some("802-3-ethernet")
                        && text("interface-name")
                            .is_none_or(|name| name.is_empty() || name == iface)
                        && !port
                        && !other_card
                })
            })
    }

    /// the saved connection the dialog is for, a wi-fi network if `ssid` isn't empty
    fn ip_connection(&self, ssid: &str) -> Option<OwnedObjectPath> {
        if ssid.is_empty() {
            let dev = self.ethernet_device()?;
            self.ethernet_connection(&dev)
        } else {
            self.known_one(&self.ssid_of(ssid))
        }
    }

    fn load_ip(&self, ssid: &str) -> IpConfig {
        let settings = self.ip_connection(ssid).and_then(|path| {
            self.at::<nm::SettingsConnection<'_>>(&path).ok()?.get_settings().ok()
        });
        settings.map_or_else(
            || self.pending_ip.get(&self.ssid_of(ssid)).cloned().unwrap_or_default(),
            |settings| IpConfig::from_settings(&settings),
        )
    }

    /// replaces the IP settings of a saved connection, keeping everything else
    fn update_ip(&self, path: &ObjectPath<'_>, ip: &IpConfig) -> zbus::Result<()> {
        let conn = self.at::<nm::SettingsConnection<'_>>(path)?;
        let mut settings = conn.get_settings()?;
        // secrets aren't part of the settings, and updating without them would drop them
        for name in ["802-11-wireless-security", "802-1x"] {
            if settings.contains_key(name)
                && let Ok(secrets) = conn.get_secrets(name)
            {
                for (name, values) in secrets {
                    settings.entry(name).or_default().extend(values);
                }
            }
        }
        for (name, values) in ip.sections() {
            let section = settings.entry(name.to_owned()).or_default();
            // the dialog only sets IPv6 DNS, a manual or disabled IPv6 stays as it was
            let v6 = name == "ipv6";
            // NetworkManager refuses DNS servers without IPv6 addressing, so leave it alone then
            let method = section.get("method").and_then(|m| m.downcast_ref::<&str>().ok());
            if v6 && matches!(method, Some("ignore" | "disabled" | "link-local")) {
                continue;
            }
            let replaced = |key: &str| {
                IP_KEYS.contains(&key)
                    && (!v6 || matches!(key, "dns" | "dns-data" | "ignore-auto-dns"))
            };
            section.retain(|key, _| !replaced(key));
            for (key, value) in values.into_iter().filter(|(key, _)| replaced(key)) {
                // only fails for file descriptors, which these never are
                if let Ok(value) = OwnedValue::try_from(value) {
                    section.insert(key.to_owned(), value);
                }
            }
        }
        conn.update(settings)
    }

    /// (re)connects `conn` on `dev`, so new settings take effect
    fn activate(&mut self, conn: &ObjectPath<'_>, dev: &ObjectPath<'_>) -> Result<(), Failure> {
        let root = ObjectPath::from_static_str_unchecked("/");
        let watch = self.watch(dev);
        let active = self.manager.activate_connection(conn, dev, &root).map_err(|e| {
            tracing::error!(?e, "cannot activate connection");
            FAILED_IP
        })?;
        self.wait(&active, dev, watch)
    }

    fn save_ip(&mut self, ssid: &str, ip: IpConfig) -> Result<(), Failure> {
        let failed = |e: zbus::Error| {
            tracing::error!(?e, ssid, "cannot save IP settings");
            FAILED_IP
        };
        if !ssid.is_empty() {
            let bytes = self.ssid_of(ssid);
            let Some(conn) = self.known_one(&bytes) else {
                // applied once connecting creates the connection
                self.pending_ip.insert(bytes, ip);
                return Ok(());
            };
            self.pending_ip.remove(&bytes);
            self.update_ip(&conn, &ip).map_err(failed)?;
            // on whichever adapter it's up on
            for dev in self.devices_of(DEVICE_TYPE_WIFI) {
                if self.active_on(&dev).as_ref() == Some(&conn) {
                    return self.activate(&conn, &dev);
                }
            }
            return Ok(());
        }
        let dev = self.ethernet_device().ok_or(FAILED_IP)?;
        let conn = if let Some(conn) = self.ethernet_connection(&dev) {
            self.update_ip(&conn, &ip).map_err(failed)?;
            conn
        } else {
            let mut settings: nm::ConnSettings<'_> = HashMap::from([
                (
                    "connection",
                    HashMap::from([
                        ("id", "Wired connection".into()),
                        ("type", "802-3-ethernet".into()),
                    ]),
                ),
                ("802-3-ethernet", HashMap::new()),
            ]);
            settings.extend(ip.sections());
            let settings_proxy: nm::Settings<'_> =
                zbus::blocking::proxy::Builder::new(&self.conn).build().map_err(failed)?;
            settings_proxy.add_connection(settings).map_err(failed)?
        };
        // without a cable there's nothing to connect, it's used once one is plugged in
        let plugged = self
            .at::<nm::Device<'_>>(&dev)
            .and_then(|d| d.state())
            .is_ok_and(|s| s >= DEVICE_STATE_DISCONNECTED);
        if plugged { self.activate(&conn, &dev) } else { Ok(()) }
    }
}

/// the bytes of a MAC address written like `AA:BB:CC:DD:EE:FF`
fn parse_mac(s: &str) -> Option<Vec<u8>> {
    s.split(':')
        .map(|b| u8::from_str_radix(b, 16).ok())
        .collect::<Option<Vec<_>>>()
        .filter(|b| b.len() == 6)
}

/// the helper starting the portal browser at `uri` as [`PORTAL_USER`] in `taidan-portal.service`, with
/// what to write to its stdin. It waits for the browser to quit, so [`Nm::check_portal`] can tell.
/// [`None`] if the browser can't be given the session's display
fn portal_command(
    proxy: Option<(String, String)>,
    uri: &str,
) -> Option<(std::process::Command, Vec<u8>)> {
    let runtime = std::env::var("XDG_RUNTIME_DIR").ok().filter(|d| d.starts_with('/'))?;
    let display = std::env::var("WAYLAND_DISPLAY").ok().filter(|d| !d.is_empty())?;
    let socket = if display.starts_with('/') { display } else { format!("{runtime}/{display}") };
    // bound into the browser's own runtime dir below, which takes the path unquoted
    if socket.contains(|c: char| c == ':' || c.is_whitespace()) {
        tracing::error!(socket, "cannot give the portal browser a display at this path");
        return None;
    }
    // the session's own files are only taidan's, so the browser gets the display socket bound into
    // its own runtime dir and nothing else. Connecting still needs write access to the socket
    let acl = format!("u:{PORTAL_USER}:rw");
    match std::process::Command::new("setfacl").args(["-m", &acl, "--", &socket]).status() {
        Ok(status) if status.success() => {}
        res => {
            tracing::error!(?res, socket, "cannot let the portal browser use the display");
            return None;
        }
    }
    // a crashed taidan's may still be there, it can't be started again while it is
    if let Err(e) = stop_portal_unit() {
        tracing::debug!(?e, "no earlier portal browser to stop");
    }
    let lang = std::env::var("LANG").ok();
    // the helper gets the proxy in the request, and passes it to the browser through a file rather
    // than an argument, since anyone can read a command's arguments and the URL may carry a
    // password
    let proxy = proxy.and_then(|(url, bypass)| libtaidan::proxy::Proxy::new(&url, &bypass));
    let verb = libtaidan::Verb::StartPortal { socket, lang, uri: uri.to_owned() };
    let mut cmd = libtaidan::helper::std_command(&verb);
    // the installation's proxy isn't set yet, so the request gets the page's
    let stdin = libtaidan::helper::stdin_with_proxy(&verb, &[], proxy.as_ref());
    cmd.stdin(std::process::Stdio::piped());
    Some((cmd, stdin))
}

/// `systemctl stop` for `taidan-portal.service`, on a thread of its own, as this may run inside
/// [`crate::RT`] already, e.g. from `install::finish`
fn stop_portal_unit() -> libtaidan::prelude::Res<()> {
    std::thread::spawn(|| crate::RT.block_on(libtaidan::root(libtaidan::Verb::StopPortal)))
        .join()
        .unwrap_or_else(|_| Err(libtaidan::prelude::Err::Msg("stopping the portal panicked")))
}

/// stops the portal browser, which would outlive taidan otherwise. Through systemd, it's another
/// user's and only root could signal it
fn close_portal() {
    let Some(mut child) = portal().take() else { return };
    if let Err(e) = stop_portal_unit() {
        tracing::warn!(?e, "cannot stop the captive portal browser");
    }
    // `systemd-run --wait` ends with the unit
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    tracing::warn!("systemd-run didn't end with the captive portal browser");
}

/// the hostname for the device name `name`, see [`derive_hostname`]. If nothing in `name` is
/// usable, e.g. it's all Japanese, it's `username-pc` so a name still changes the hostname. An
/// empty `name` stays empty, which keeps the system's existing name
fn device_hostname(name: &str, username: &str) -> String {
    if name.trim().is_empty() {
        return String::new();
    }
    let hostname = derive_hostname(name);
    if !hostname.is_empty() {
        return hostname;
    }
    let user = derive_hostname(username);
    if user.is_empty() { String::new() } else { derive_hostname(&format!("{user}-pc")) }
}

/// a hostname for `name`: lowercase letters, digits and dashes, or empty if there are none
fn derive_hostname(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().filter(|c| !matches!(c, '\'' | '’')) {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.truncate(63);
    out.trim_end_matches('-').to_owned()
}

fn push(ui: &slint::Weak<AppWindow>, f: impl FnOnce(&Network<'_>) + Send + 'static) {
    if let Err(e) = ui.upgrade_in_event_loop(move |ui| f(&ui.global::<Network<'_>>())) {
        tracing::error!(?e, "cannot update network state");
    }
}

fn push_snapshot(ui: &slint::Weak<AppWindow>, snap: Snapshot) {
    push(ui, move |net| {
        net.set_state(snap.state);
        net.set_wifi_available(snap.wifi);
        net.set_wifi_enabled(snap.radio == Radio::On);
        net.set_wifi_hardware_enabled(snap.radio != Radio::HardwareOff);
        net.set_ethernet(snap.ethernet);
        net.set_wired(snap.wired);
        net.set_connected_ssid(
            snap.networks.iter().find(|n| n.connected).map(|n| n.ssid.clone()).unwrap_or_default(),
        );
        // update the list in place, so rows that are still there keep keyboard and screen reader
        // focus rather than being rebuilt on every scan
        let model = net.get_networks();
        if let Some(model) = model.as_any().downcast_ref::<slint::VecModel<WifiNetwork>>() {
            sync_model(model, snap.networks, |n| n.ssid.clone());
        } else {
            net.set_networks(slint::ModelRc::new(slint::VecModel::from(snap.networks)));
        }
    });
}

/// makes `model` match `rows`, moving rows with the same key rather than replacing them and only
/// touching rows that changed
fn sync_model<T: Clone + PartialEq + 'static, K: PartialEq>(
    model: &slint::VecModel<T>,
    rows: Vec<T>,
    key: impl Fn(&T) -> K,
) {
    let keys = rows.iter().map(&key).collect::<Vec<_>>();
    for i in (0..model.row_count()).rev() {
        if model.row_data(i).is_none_or(|r| !keys.contains(&key(&r))) {
            model.remove(i);
        }
    }
    for (i, (row, k)) in rows.into_iter().zip(&keys).enumerate() {
        let found =
            (i..model.row_count()).find(|&j| model.row_data(j).is_some_and(|r| key(&r) == *k));
        match found {
            Some(j) => {
                if j != i {
                    let moved = model.remove(j);
                    model.insert(i, moved);
                }
                if model.row_data(i).as_ref() != Some(&row) {
                    model.set_row_data(i, row);
                }
            }
            None => model.insert(i, row),
        }
    }
}

fn handle(nm: &mut Nm, ui: &slint::Weak<AppWindow>, cmd: Cmd) {
    let (ssid, res, security, visit) = match cmd {
        Cmd::Refresh => {
            push(ui, |net| net.set_scanning(true));
            nm.scan();
            // results come in over the next few seconds, `Nm::publish` says when it's done
            nm.scanning_until = Some(Instant::now() + SCAN_TIME);
            return;
        }
        Cmd::Connect { ssid, password, visit } => {
            start_connecting(ui, &ssid);
            let res = nm.connect(&ssid, &password);
            let security =
                nm.aps.iter().find(|ap| ap.name == ssid).map(|ap| ap.key_mgmt.security());
            // an open network has no password to get wrong, a supplicant disconnect there is
            // something else, like a MAC filter or a full AP
            let res = match res {
                Err(FAILED_SECRETS) if security == Some(Security::Open) => Err(FAILED),
                res => res,
            };
            (ssid, res, security, visit)
        }
        Cmd::ConnectHidden { ssid, key_mgmt, password } => {
            return connect_hidden(nm, ui, ssid, key_mgmt, &password);
        }
        Cmd::ConnectEnterprise { login, tls_only, visit } => {
            return connect_enterprise(nm, ui, login, tls_only, visit);
        }
        Cmd::SetProxy { url, bypass, persist } => {
            return set_proxy(nm, ui, &url, &bypass, persist);
        }
        Cmd::OpenPortal { proxy, visit } => {
            // the page was left meanwhile, which closed the browser without waiting for this
            if PAGE_VISIT.load(std::sync::atomic::Ordering::SeqCst) != visit {
                return;
            }
            let open = nm.open_portal(proxy);
            push(ui, move |net| net.set_portal_open(open));
            return;
        }
        Cmd::ClosePortal => {
            close_portal();
            // check whether logging in worked on the next publish
            nm.portal_checked = None;
            push(ui, |net| net.set_portal_open(false));
            return;
        }
        Cmd::LoadIp(ssid) => {
            let ip = nm.load_ip(&ssid).display();
            push(ui, move |net| {
                // closed or opened for another network meanwhile, whose values these aren't
                if !net.get_ip_open() || net.get_ip_ssid().as_str() != ssid.as_str() {
                    return;
                }
                net.set_ip(ip);
                net.set_ip_loading(false);
            });
            return;
        }
        Cmd::SaveIp(ssid, ip) => {
            let res = IpConfig::parse(&ip).and_then(|ip| nm.save_ip(&ssid, ip));
            push(ui, move |net| {
                net.set_ip_busy(false);
                // cancelled meanwhile, it mustn't close or show errors in another network's
                if !net.get_ip_open() || net.get_ip_ssid().as_str() != ssid.as_str() {
                    return;
                }
                match res {
                    Ok(()) => net.set_ip_open(false),
                    Err(failure) => net.set_ip_error_id(failure.into()),
                }
            });
            return;
        }
        Cmd::EnableWifi => return enable_wifi(nm, ui),
    };
    push(ui, move |net| {
        net.set_connecting_ssid("".into());
        match res {
            Ok(()) => net.set_password_ssid("".into()),
            Err(failure) => {
                net.set_error_ssid(ssid.as_str().into());
                net.set_error_id(failure.into());
                // ask for the password if a saved one didn't work, or one was given and was wrong,
                // unless the page was left meanwhile, where a dialog would come out of nowhere
                let left = PAGE_VISIT.load(std::sync::atomic::Ordering::SeqCst) != visit;
                // nor over another dialog, whose typing would go into this one
                let other_dialog = net.get_proxy_open()
                    || net.get_ip_open()
                    || net.get_hidden_open()
                    || net.get_enterprise_open()
                    || net.get_offline_prompt()
                    || !["", ssid.as_str()].contains(&net.get_password_ssid().as_str());
                // nor again after its dialog was closed while this was still going
                let dismissed = net.get_dismissed_ssid().as_str() == ssid.as_str();
                if failure == FAILED_SECRETS && !left && !other_dialog && !dismissed {
                    if let Some(Security::Enterprise | Security::EnterpriseTls) = security {
                        net.set_enterprise_ssid(ssid.as_str().into());
                        net.set_enterprise_wired(false);
                        net.set_enterprise_tls_only(security == Some(Security::EnterpriseTls));
                        net.set_enterprise_error_id(failure.into());
                        net.set_enterprise_open(true);
                    } else {
                        net.set_password_ssid(ssid.as_str().into());
                    }
                }
            }
        }
    });
}

/// [`Cmd::ConnectEnterprise`], the dialog closes once it's connected and shows why if it can't
fn connect_enterprise(
    nm: &mut Nm,
    ui: &slint::Weak<AppWindow>,
    mut login: EnterpriseLogin,
    tls_only: bool,
    visit: i32,
) {
    // spaces around a typed hidden network's name are a slip, but a network in the list
    // can really have them
    if !nm.aps.iter().any(|ap| ap.name == login.ssid.as_str()) {
        login.ssid = login.ssid.trim().into();
    }
    if !login.wired {
        start_connecting(ui, &login.ssid);
    }
    let res = install_certs(&mut login).and_then(|certs| {
        let res = if login.wired {
            nm.connect_wired_8021x(&login)
        } else {
            let ssid = nm.ssid_of(&login.ssid);
            let (hidden, key_mgmt) = match nm.aps.iter().find(|ap| ap.ssid == ssid) {
                Some(ap) => (false, ap.key_mgmt),
                // dropped out of the scan since the dialog opened, so go by how it was seen
                None => match nm.seen.get(&ssid) {
                    Some(&key_mgmt) => (false, key_mgmt),
                    // never in the list, so typed in for a hidden network
                    None if tls_only => (true, KeyMgmt::EnterpriseSuiteB),
                    None => (true, KeyMgmt::Enterprise),
                },
            };
            nm.add_and_connect(&ssid, enterprise_settings(&login, &ssid, hidden, key_mgmt))
        };
        certs.finish(res.is_ok());
        res
    });
    push(ui, move |net| {
        net.set_connecting_ssid("".into());
        net.set_enterprise_busy(false);
        // the dialog it came from was closed meanwhile, maybe opened again for another network or
        // the wired connection, which mustn't close or show this login's error
        if !net.get_enterprise_open() || net.get_enterprise_visit() != visit {
            return;
        }
        match res {
            Ok(()) => net.set_enterprise_open(false),
            Err(failure) => net.set_enterprise_error_id(failure.into()),
        }
    });
}

/// [`Cmd::EnableWifi`]
fn enable_wifi(nm: &mut Nm, ui: &slint::Weak<AppWindow>) {
    if let Err(e) = nm.manager.set_wireless_enabled(true) {
        tracing::error!(?e, "cannot turn on wi-fi");
        return;
    }
    // `NetworkManager` scans once the adapter is up, which takes a moment
    push(ui, |net| net.set_scanning(true));
    nm.scanning_until = Some(Instant::now() + SCAN_TIME * 2);
}

/// [`Cmd::ConnectHidden`], the dialog closes once it's connected and shows why if it can't
fn connect_hidden(
    nm: &mut Nm,
    ui: &slint::Weak<AppWindow>,
    ssid: String,
    key_mgmt: KeyMgmt,
    password: &str,
) {
    start_connecting(ui, &ssid);
    let res = nm.connect_hidden(&ssid, key_mgmt, password);
    push(ui, move |net| {
        net.set_connecting_ssid("".into());
        match res {
            Ok(()) => net.set_hidden_open(false),
            Err(failure) => {
                net.set_error_ssid(ssid.as_str().into());
                net.set_error_id(failure.into());
            }
        }
    });
}

fn start_connecting(ui: &slint::Weak<AppWindow>, ssid: &str) {
    let ssid = slint::SharedString::from(ssid);
    push(ui, move |net| {
        net.set_error_id("".into());
        net.set_error_ssid("".into());
        net.set_connecting_ssid(ssid);
    });
}

/// waits for `NetworkManager`, which may start late or restart, trying again less and less often.
/// [`None`] once there's no point anymore, as taidan is exiting
fn connect_nm(ui: &slint::Weak<AppWindow>, rx: &mpsc::Receiver<Cmd>) -> Option<Nm> {
    const MAX_BACKOFF: Duration = Duration::from_secs(30);
    let mut backoff = Duration::from_secs(1);
    loop {
        if SHUT_DOWN.load(std::sync::atomic::Ordering::SeqCst) {
            return None;
        }
        let e = match Nm::new(ui.clone()) {
            Ok(nm) => return Some(nm),
            Err(e) => e,
        };
        if backoff == Duration::from_secs(1) {
            tracing::error!(?e, "cannot talk to NetworkManager");
            push(ui, |net| net.set_wifi_available(false));
        } else {
            tracing::debug!(?e, "still cannot talk to NetworkManager");
        }
        let deadline = Instant::now() + backoff;
        // what's asked meanwhile can't be done, so it fails rather than spinning
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match rx.recv_timeout(left) {
                Ok(_) => push(ui, stop_busy),
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => return None,
            }
        }
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

fn worker(ui: &slint::Weak<AppWindow>, rx: &mpsc::Receiver<Cmd>) {
    let Some(mut nm) = connect_nm(ui, rx) else { return };
    nm.scan();
    // right away, whether it's online decides how the installation runs, even if the network
    // page is skipped
    nm.publish();
    loop {
        if SHUT_DOWN.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        let poll =
            if ON_PAGE.load(std::sync::atomic::Ordering::Relaxed) { POLL } else { POLL_ELSEWHERE };
        // sooner if a scan is due to end
        let timeout = (nm.scanning_until)
            .map_or(poll, |until| until.saturating_duration_since(Instant::now()).min(poll));
        match rx.recv_timeout(timeout) {
            Ok(cmd) => handle(&mut nm, ui, cmd),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                close_portal();
                return;
            }
        }
        nm.publish();
    }
}

/// shows `dir` in the file browser, or the first place if it's empty. Listed on another thread,
/// as a slow drive or network mount would freeze the UI
fn browse(ui: &slint::Weak<AppWindow>, dir: &str, pac: bool, all: bool) {
    /// counts the folders asked for, so a slow listing doesn't replace a newer one
    static LATEST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let this = LATEST.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    let (ui, dir) = (ui.clone(), dir.to_owned());
    std::thread::spawn(move || {
        let places = browse_places();
        let dir = if dir.is_empty() {
            places.first().map_or_else(|| "/".to_owned(), |p| p.path.to_string())
        } else {
            dir
        };
        let path = std::path::Path::new(&dir);
        let parent = path.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let entries = list_dir(path, pac, all);
        push(&ui, move |net| {
            if LATEST.load(std::sync::atomic::Ordering::SeqCst) != this {
                return;
            }
            net.set_browse_entries(slint::ModelRc::new(slint::VecModel::from(entries)));
            net.set_browse_places(slint::ModelRc::new(slint::VecModel::from(places)));
            net.set_browse_parent(parent.into());
            net.set_browse_dir(dir.into());
        });
    });
}

/// connecting to a network from the list or the password dialog, and turning wi-fi on for it
fn init_connect(
    ui: &slint::Weak<AppWindow>,
    net: &Network<'_>,
    send: impl Fn(Cmd) + Clone + 'static,
) {
    net.on_enable_wifi({
        let send = send.clone();
        move || send(Cmd::EnableWifi)
    });
    net.on_connect({
        let (ui, send) = (ui.clone(), send.clone());
        move |ssid, password| {
            if start_busy(&ui, &ssid) {
                let visit = PAGE_VISIT.load(std::sync::atomic::Ordering::SeqCst);
                send(Cmd::Connect { ssid: ssid.into(), password: password.into(), visit });
            }
        }
    });
    net.on_connect_hidden({
        let ui = ui.clone();
        move |ssid, security, password| {
            let ssid = ssid.trim();
            let key_mgmt = match security {
                HiddenSecurity::Open => KeyMgmt::Open,
                HiddenSecurity::Wpa => KeyMgmt::Psk,
                HiddenSecurity::Wpa3 => KeyMgmt::Sae,
                // the 802.1X dialog takes it from here
                HiddenSecurity::Enterprise => return,
            };
            if let Some(app) = ui.upgrade() {
                app.global::<Network<'_>>().set_hidden_ssid(ssid.into());
            }
            if start_busy(&ui, ssid) {
                send(Cmd::ConnectHidden { ssid: ssid.into(), key_mgmt, password: password.into() });
            }
        }
    });
}

/// clears every busy flag once the network worker is gone, showing the work that was going on as
/// failed. without it a connect, proxy check or IP dialog would spin until the app is restarted
fn stop_busy(net: &Network<'_>) {
    let ssid = net.get_connecting_ssid();
    if !ssid.is_empty() {
        net.set_connecting_ssid("".into());
        net.set_error_ssid(ssid);
        net.set_error_id(FAILED.into());
    }
    if net.get_enterprise_busy() {
        net.set_enterprise_busy(false);
        net.set_enterprise_error_id(FAILED.into());
    }
    if net.get_proxy_busy() {
        net.set_proxy_busy(false);
        net.set_proxy_error_id(PROXY_FAILED.into());
    }
    if net.get_ip_busy() || net.get_ip_loading() {
        net.set_ip_busy(false);
        net.set_ip_loading(false);
        net.set_ip_error_id(FAILED_IP.into());
    }
    net.set_portal_open(false);
}

/// runs [`stop_busy`] when the network worker's thread ends, returning or panicking
struct WorkerGone(slint::Weak<AppWindow>);

impl Drop for WorkerGone {
    fn drop(&mut self) {
        if SHUT_DOWN.load(std::sync::atomic::Ordering::SeqCst) && !std::thread::panicking() {
            tracing::debug!("network worker stopped for the shutdown");
        } else {
            tracing::error!("network worker stopped");
        }
        push(&self.0, stop_busy);
    }
}

/// marks `ssid` as connecting right away, not once the worker gets to it after a scan, so a second
/// click or Enter doesn't queue another connect that drops the first one. False if one is going
/// already
fn start_busy(ui: &slint::Weak<AppWindow>, ssid: &str) -> bool {
    let Some(ui) = ui.upgrade() else { return true };
    let net = ui.global::<Network<'_>>();
    if !net.get_connecting_ssid().is_empty() {
        return false;
    }
    net.set_error_id("".into());
    net.set_error_ssid("".into());
    net.set_dismissed_ssid("".into());
    net.set_connecting_ssid(ssid.into());
    true
}

/// the captive portal button, which opens and closes the browser through the network worker
fn init_portal(
    ui: &slint::Weak<AppWindow>,
    net: &Network<'_>,
    send: impl Fn(Cmd) + Clone + 'static,
) {
    net.on_open_portal({
        let (send, ui) = (send.clone(), ui.clone());
        move || {
            let proxy = ui.upgrade().and_then(|ui| {
                let net = ui.global::<Network<'_>>();
                let url = net.get_proxy_url();
                (!url.is_empty()).then(|| (url.into(), net.get_proxy_bypass().into()))
            });
            let visit = PAGE_VISIT.load(std::sync::atomic::Ordering::SeqCst);
            send(Cmd::OpenPortal { proxy, visit });
        }
    });
    net.on_close_portal(move || send(Cmd::ClosePortal));
}

/// starts the network worker's thread, which takes commands from the returned sender
fn spawn_worker(ui: &slint::Weak<AppWindow>) -> mpsc::Sender<Cmd> {
    let (tx, rx) = mpsc::channel();
    let ui = ui.clone();
    std::thread::spawn(move || {
        // also when it panics, so the dialogs don't spin forever
        let _guard = WorkerGone(ui.clone());
        worker(&ui, &rx);
    });
    tx
}

/// the pure callbacks, which work on their arguments only
fn init_helpers(net: &Network<'_>) {
    net.on_derive_hostname(|name, username| device_hostname(&name, &username).into());
    net.on_realm(|username| realm(&username).into());
    net.on_is_pkcs12(|path| is_pkcs12(&path));
    net.on_trim(|text| text.trim().into());
    net.on_file_name(|path| {
        std::path::Path::new(path.as_str())
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().as_ref().into())
    });
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, net: &Network<'_>) {
    let tx = spawn_worker(ui);
    net.set_persist_offline_btn(crate::CFG.taidan0.persist_no_internet_btn);
    init_helpers(net);
    net.on_browse({
        let ui = ui.clone();
        move |dir, pac, all| browse(&ui, &dir, pac, all)
    });
    let send = {
        let ui = ui.clone();
        move |cmd| {
            if let Err(e) = tx.send(cmd) {
                tracing::error!(?e, "network worker is gone");
                // the callback set its busy flag already, and nothing is left to clear it
                push(&ui, stop_busy);
            }
        }
    };
    net.on_refresh({
        let send = send.clone();
        move || send(Cmd::Refresh)
    });
    init_connect(ui, net, send.clone());
    net.on_connect_enterprise({
        let (send, ui) = (send.clone(), ui.clone());
        move |login| {
            // right away, so a second click doesn't queue another login
            let (tls_only, visit) = if let Some(ui) = ui.upgrade() {
                let net = ui.global::<Network<'_>>();
                if net.get_enterprise_busy() {
                    return;
                }
                net.set_enterprise_busy(true);
                net.set_enterprise_error_id("".into());
                (net.get_enterprise_tls_only(), net.get_enterprise_visit())
            } else {
                (false, 0)
            };
            send(Cmd::ConnectEnterprise { login, tls_only, visit });
        }
    });
    net.on_set_proxy({
        let (send, ui) = (send.clone(), ui.clone());
        move |url, bypass, persist| {
            if let Some(ui) = ui.upgrade() {
                let net = ui.global::<Network<'_>>();
                net.set_proxy_busy(true);
                net.set_proxy_error_id("".into());
            }
            send(Cmd::SetProxy { url: url.trim().into(), bypass: bypass.into(), persist });
        }
    });
    net.on_open_ip({
        let (send, ui) = (send.clone(), ui.clone());
        move |ssid| {
            if let Some(ui) = ui.upgrade() {
                let net = ui.global::<Network<'_>>();
                net.set_ip_ssid(ssid.clone());
                net.set_ip(IpSettings::default());
                net.set_ip_error_id("".into());
                net.set_ip_busy(false);
                net.set_ip_loading(true);
                net.set_ip_open(true);
            }
            send(Cmd::LoadIp(ssid.into()));
        }
    });
    net.on_save_ip({
        let (send, ui) = (send.clone(), ui.clone());
        move |ssid, ip| {
            if let Some(ui) = ui.upgrade() {
                let net = ui.global::<Network<'_>>();
                net.set_ip_busy(true);
                net.set_ip_error_id("".into());
            }
            send(Cmd::SaveIp(ssid.into(), ip));
        }
    });
    init_portal(ui, net, send);
    net.on_page_changed(|on_page| {
        ON_PAGE.store(on_page, std::sync::atomic::Ordering::Relaxed);
        PAGE_VISIT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });
    net.on_save({
        let ui = ui.clone();
        move || {
            PAGE_VISIT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            // done with the network page, so done logging in to the portal too. Not through the
            // worker, which may be busy connecting for a while yet
            std::thread::spawn(close_portal);
            if let Some(ui) = ui.upgrade() {
                ui.global::<Network<'_>>().set_portal_open(false);
            }
            if let Some(ui) = ui.upgrade() {
                save(&ui.global::<Network<'_>>());
            }
        }
    });
}

/// keeps what the network page set for the installation, when leaving it. Whether it goes online
/// is up to `install::start`, from `Network.install-online`
fn save(net: &Network<'_>) {
    // a connection's dialog mustn't follow onto the next page
    net.set_password_ssid("".into());
    net.set_enterprise_open(false);
    let mut settings = crate::SETTINGS.write();
    let url = net.get_proxy_url();
    settings.proxy = (!url.is_empty())
        .then(|| libtaidan::proxy::Proxy::new(&url, &net.get_proxy_bypass()))
        .flatten();
    net.get_device_name().trim().clone_into(&mut settings.device_name);
    settings.hostname = net.get_hostname().into();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostnames() {
        assert_eq!(derive_hostname("Jane's Laptop"), "janes-laptop");
        assert_eq!(derive_hostname("  -- My  PC 2! "), "my-pc-2");
        assert_eq!(derive_hostname("ノートパソコン"), "");
        assert_eq!(device_hostname("Jane's Laptop", "jane"), "janes-laptop");
        assert_eq!(device_hostname("ノートパソコン", "taro"), "taro-pc");
        assert_eq!(device_hostname("ノートパソコン", "taro_y"), "taro-y-pc");
        // empty keeps the system's hostname, with or without a username yet
        assert_eq!(device_hostname("  ", "taro"), "");
        assert_eq!(device_hostname("ノートパソコン", ""), "");
    }

    #[test]
    fn cert_names_are_unique() {
        assert_eq!(cert_name(true, "ignored"), "wired");
        assert_eq!(cert_name(false, "Home"), "home-486f6d65");
        // the same as a hostname, but not as files
        assert_ne!(cert_name(false, "Café"), cert_name(false, "Caf"));
        assert_ne!(cert_name(false, "東京"), cert_name(false, "大阪"));
        assert_eq!(cert_name(false, "東京"), "wifi-e69db1e4baac");
    }

    #[test]
    fn hidden_wifi_settings() {
        let mut s = wifi_settings(b"Home", "Home", KeyMgmt::Sae, "hunter22");
        assert!(!s["802-11-wireless"].contains_key("hidden"));
        mark_hidden(&mut s);
        assert_eq!(s["802-11-wireless"]["hidden"], Value::from(true));
        assert_eq!(s["802-11-wireless-security"]["key-mgmt"], Value::from("sae"));
        let mut s = wifi_settings(b"Cafe", "Cafe", KeyMgmt::Open, "");
        mark_hidden(&mut s);
        assert!(!s.contains_key("802-11-wireless-security"));
    }

    #[test]
    fn non_utf8_ssid_kept_as_bytes() {
        let ssid = b"Caf\xe9";
        let name = String::from_utf8_lossy(ssid).into_owned();
        let s = wifi_settings(ssid, &name, KeyMgmt::Psk, "hunter22");
        assert_eq!(s["802-11-wireless"]["ssid"], Value::from(ssid.to_vec()));
        assert_eq!(s["connection"]["id"], Value::from("Caf\u{fffd}"));
    }

    fn owned(settings: nm::ConnSettings<'_>) -> HashMap<String, HashMap<String, OwnedValue>> {
        settings
            .into_iter()
            .map(|(name, values)| {
                let values = values
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), OwnedValue::try_from(v).unwrap()))
                    .collect();
                (name.to_owned(), values)
            })
            .collect()
    }

    #[test]
    fn security_of_saved_connections() {
        let mut s = wifi_settings(b"Home", "Home", KeyMgmt::Sae, "hunter22");
        mark_hidden(&mut s);
        assert_eq!(saved_security(&owned(s)), Some((KeyMgmt::Sae, true)));
        let s = wifi_settings(b"Cafe", "Cafe", KeyMgmt::Open, "");
        assert_eq!(saved_security(&owned(s)), Some((KeyMgmt::Open, false)));
    }

    #[test]
    fn realms() {
        assert_eq!(realm("jane@uni.example.edu"), "uni.example.edu");
        assert_eq!(realm("jane"), "");
    }

    #[test]
    fn key_mgmt_from_flags() {
        assert_eq!(KeyMgmt::from_flags(0, 0, 0), KeyMgmt::Open);
        assert_eq!(KeyMgmt::from_flags(1, 0, 0), KeyMgmt::Wep);
        assert_eq!(
            KeyMgmt::from_flags(1, 0, AP_SEC_KEY_MGMT_PSK | AP_SEC_KEY_MGMT_SAE),
            KeyMgmt::Psk
        );
        assert_eq!(KeyMgmt::from_flags(1, 0, AP_SEC_KEY_MGMT_SAE), KeyMgmt::Sae);
        assert_eq!(KeyMgmt::from_flags(1, 0, AP_SEC_KEY_MGMT_802_1X), KeyMgmt::Enterprise);
        // transition networks take plain WPA-EAP
        assert_eq!(
            KeyMgmt::from_flags(1, 0, AP_SEC_KEY_MGMT_802_1X | AP_SEC_KEY_MGMT_EAP_SUITE_B_192),
            KeyMgmt::Enterprise
        );
        assert_eq!(
            KeyMgmt::from_flags(1, 0, AP_SEC_KEY_MGMT_EAP_SUITE_B_192),
            KeyMgmt::EnterpriseSuiteB
        );
    }

    fn ip(manual: bool, address: &str, gateway: &str, dns: &str) -> IpSettings {
        IpSettings { manual, address: address.into(), gateway: gateway.into(), dns: dns.into() }
    }

    #[test]
    fn ip_parse_errors() {
        assert_eq!(IpConfig::parse(&ip(false, "", "", "")), Ok(IpConfig::default()));
        assert_eq!(
            IpConfig::parse(&ip(false, "", "", "1.1.1.1, nope")),
            Err("page-internet-ip-error-dns")
        );
        assert_eq!(
            IpConfig::parse(&ip(true, "10.0.0.5/40", "", "1.1.1.1")),
            Err("page-internet-ip-error-address")
        );
        assert_eq!(
            IpConfig::parse(&ip(true, "10.0.0.5", "x", "1.1.1.1")),
            Err("page-internet-ip-error-gateway")
        );
        assert_eq!(
            IpConfig::parse(&ip(true, "10.0.0.5", "", "")),
            Err("page-internet-ip-error-dns-needed")
        );
    }

    #[test]
    fn ip_round_trips_through_nm_settings() {
        let typed = ip(true, "192.168.1.20/24", "192.168.1.1", "1.1.1.1 2606:4700:4700::1111");
        let cfg = IpConfig::parse(&typed).unwrap();
        let settings = cfg
            .sections()
            .into_iter()
            .map(|(name, values)| {
                let values = values
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), OwnedValue::try_from(v).unwrap()))
                    .collect();
                (name.to_owned(), values)
            })
            .collect();
        assert_eq!(IpConfig::from_settings(&settings), cfg);
        let shown = cfg.display();
        assert_eq!(shown.address, "192.168.1.20/24");
        assert_eq!(shown.dns, "1.1.1.1, 2606:4700:4700::1111");
        // DNS-only override on DHCP
        let dhcp = IpConfig::parse(&ip(false, "", "", "9.9.9.9")).unwrap();
        assert_eq!(dhcp.sections()[0].1["ignore-auto-dns"], Value::from(true));
        assert_eq!(dhcp.sections()[0].1["method"], Value::from("auto"));
    }

    fn login(method: EapMethod, inner: &str) -> EnterpriseLogin {
        EnterpriseLogin {
            wired: false,
            ssid: "eduroam".into(),
            method,
            inner: inner.into(),
            username: "jane@uni.example.edu".into(),
            password: "pw".into(),
            anonymous_identity: "anonymous@uni.example.edu".into(),
            ca: CaSource::System,
            ca_cert: "".into(),
            domain: "uni.example.edu".into(),
            peap_version: 0,
            client_cert: "".into(),
            private_key: "".into(),
            private_key_password: "".into(),
            fast_provisioning: 0,
            pac_file: "".into(),
        }
    }

    #[test]
    fn enterprise_settings_verify_server() {
        let login = login(EapMethod::Peap, "mschapv2");
        let s = enterprise_settings(&login, b"eduroam", false, KeyMgmt::Enterprise);
        let x = &s["802-1x"];
        assert_eq!(x["eap"], Value::from(vec!["peap"]));
        assert_eq!(x["phase2-auth"], Value::from("mschapv2"));
        assert_eq!(x["anonymous-identity"], Value::from("anonymous@uni.example.edu"));
        assert_eq!(x["domain-suffix-match"], Value::from("uni.example.edu"));
        assert_eq!(x["system-ca-certs"], Value::from(true));
        assert!(!x.contains_key("phase1-peapver"));
        assert!(!s["802-11-wireless"].contains_key("hidden"));
        assert_eq!(s["802-11-wireless-security"]["key-mgmt"], Value::from("wpa-eap"));
        let suite_b = enterprise_settings(&login, b"eduroam", false, KeyMgmt::EnterpriseSuiteB);
        assert_eq!(
            suite_b["802-11-wireless-security"]["key-mgmt"],
            Value::from("wpa-eap-suite-b-192")
        );
        let hidden = enterprise_settings(&login, b"eduroam", true, KeyMgmt::Enterprise);
        assert_eq!(hidden["802-11-wireless"]["hidden"], Value::from(true));
        let wired = EnterpriseLogin { wired: true, ..login };
        let s = enterprise_settings(&wired, &[], false, KeyMgmt::Enterprise);
        assert_eq!(s["connection"]["type"], Value::from("802-3-ethernet"));
        assert!(!s.contains_key("802-11-wireless-security"));
    }

    #[test]
    fn ttls_inner_over_eap_or_not() {
        let l = login(EapMethod::Ttls, "mschapv2");
        let x = eap_settings(&l);
        assert_eq!(x["phase2-autheap"], Value::from("mschapv2"));
        assert!(!x.contains_key("phase2-auth"));
        let l = login(EapMethod::Ttls, "mschapv2-plain");
        let x = eap_settings(&l);
        assert_eq!(x["phase2-auth"], Value::from("mschapv2"));
        let l = login(EapMethod::Ttls, "pap");
        let x = eap_settings(&l);
        assert_eq!(x["phase2-auth"], Value::from("pap"));
    }

    #[test]
    fn peap_version_and_ca_file() {
        let mut l = login(EapMethod::Peap, "gtc");
        l.peap_version = 2;
        l.ca = CaSource::File;
        l.ca_cert = "/etc/pki/taidan/8021x/x-ca.pem".into();
        let x = eap_settings(&l);
        assert_eq!(x["phase1-peapver"], Value::from("1"));
        assert_eq!(x["ca-cert"], Value::from(b"file:///etc/pki/taidan/8021x/x-ca.pem\0".to_vec()));
        assert!(!x.contains_key("system-ca-certs"));
        l.ca = CaSource::None;
        let x = eap_settings(&l);
        assert!(!x.contains_key("domain-suffix-match"));
    }

    #[test]
    fn psk_length_is_checked() {
        assert_eq!(check_psk(KeyMgmt::Psk, "short"), Err(FAILED_PSK_LENGTH));
        assert_eq!(check_psk(KeyMgmt::Psk, "12345678"), Ok(()));
        assert_eq!(check_psk(KeyMgmt::Psk, &"a".repeat(64)), Ok(()));
        assert_eq!(check_psk(KeyMgmt::Psk, &"g".repeat(64)), Err(FAILED_PSK_LENGTH));
        assert_eq!(check_psk(KeyMgmt::Psk, &"a".repeat(65)), Err(FAILED_PSK_LENGTH));
        assert_eq!(check_psk(KeyMgmt::Sae, "short"), Ok(()));
        assert_eq!(check_psk(KeyMgmt::Open, ""), Ok(()));
    }

    #[test]
    fn sync_model_keeps_rows() {
        let model = slint::VecModel::from(vec![(1, 'a'), (2, 'b'), (3, 'c')]);
        sync_model(&model, vec![(3, 'C'), (1, 'a'), (4, 'd')], |r| r.0);
        assert_eq!(model.iter().collect::<Vec<_>>(), [(3, 'C'), (1, 'a'), (4, 'd')]);
        sync_model(&model, vec![], |r| r.0);
        assert_eq!(model.row_count(), 0);
        sync_model(&model, vec![(5, 'e'), (6, 'f')], |r| r.0);
        assert_eq!(model.iter().collect::<Vec<_>>(), [(5, 'e'), (6, 'f')]);
    }

    #[test]
    fn tls_uses_certs_not_password() {
        let mut l = login(EapMethod::Tls, "");
        l.client_cert = "/c/me.p12".into();
        l.private_key_password = "secret".into();
        let x = eap_settings(&l);
        assert!(!x.contains_key("password"));
        assert!(!x.contains_key("anonymous-identity"));
        assert_eq!(x["client-cert"], x["private-key"]);
        assert_eq!(x["private-key-password"], Value::from("secret"));
        assert!(!x.contains_key("private-key-password-flags"));
        assert!(is_pkcs12("/c/ME.PFX"));
        assert!(!is_pkcs12("/c/me.pem"));
    }

    #[test]
    fn tls_unencrypted_key_needs_no_password() {
        let mut l = login(EapMethod::Tls, "");
        l.client_cert = "/c/me.pem".into();
        l.private_key = "/c/me.key".into();
        let x = eap_settings(&l);
        assert!(!x.contains_key("private-key-password"));
        // NM_SETTING_SECRET_FLAG_NOT_REQUIRED
        assert_eq!(x["private-key-password-flags"], Value::from(4u32));
    }

    #[test]
    fn fast_and_simple_methods() {
        let mut l = login(EapMethod::Fast, "gtc");
        l.fast_provisioning = 3;
        let x = eap_settings(&l);
        assert_eq!(x["phase1-fast-provisioning"], Value::from("3"));
        assert_eq!(x["phase2-auth"], Value::from("gtc"));
        assert!(!x.contains_key("system-ca-certs"));
        for method in [EapMethod::Pwd, EapMethod::Leap, EapMethod::Md5] {
            let l = login(method, "");
            let x = eap_settings(&l);
            assert_eq!(x.len(), 3, "{method:?}: eap, identity, password");
        }
    }

    #[test]
    fn lists_folders_then_certificates() {
        let dir = std::env::temp_dir().join(format!("taidan-browse-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Sub")).unwrap();
        for f in ["b.pem", "A.p12", "notes.txt", ".hidden.pem", "x.pac"] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        let names = |pac, all| {
            list_dir(&dir, pac, all).iter().map(|e| e.name.to_string()).collect::<Vec<_>>()
        };
        assert_eq!(names(false, false), ["Sub", "A.p12", "b.pem"]);
        assert_eq!(names(true, false), ["Sub", "x.pac"]);
        assert_eq!(names(false, true), ["Sub", "A.p12", "b.pem", "notes.txt", "x.pac"]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
