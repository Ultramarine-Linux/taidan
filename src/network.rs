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

const POLL_ON_PAGE: Duration = Duration::from_secs(3);
const POLL_OFF_PAGE: Duration = Duration::from_secs(10);
static ON_PAGE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(45);
const SCAN_RESULTS_DELAY: Duration = Duration::from_secs(3);
const PORTAL_RECHECK_INTERVAL: Duration = Duration::from_secs(10);

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
        #[zbus(property)]
        fn primary_connection(&self) -> zbus::Result<OwnedObjectPath>;
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

const STATE_CONNECTED_GLOBAL: u32 = 70;
const CONNECTIVITY_PORTAL: u32 = 2;
const CONNECTIVITY_FULL: u32 = 4;
const DEVICE_TYPE_ETHERNET: u32 = 1;
const DEVICE_TYPE_WIFI: u32 = 2;
const DEVICE_STATE_DISCONNECTED: u32 = 30;
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
const DEVICE_STATE_REASON_NO_SECRETS: u32 = 7;
const DEVICE_STATE_REASON_SUPPLICANT_DISCONNECT: u32 = 8;
const REASONS_BAD_SECRETS: [u32; 2] =
    [DEVICE_STATE_REASON_NO_SECRETS, DEVICE_STATE_REASON_SUPPLICANT_DISCONNECT];
const SECRET_FLAG_NOT_REQUIRED: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyMgmt {
    Open,
    Owe,
    Psk,
    Sae,
    Enterprise,
    EnterpriseSuiteB,
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

    const fn merge(self, other: Self) -> Self {
        const fn rank(k: KeyMgmt) -> u8 {
            match k {
                KeyMgmt::Enterprise => 0,
                KeyMgmt::EnterpriseSuiteB => 1,
                KeyMgmt::Psk => 2,
                KeyMgmt::Sae => 3,
                KeyMgmt::Owe => 4,
                KeyMgmt::Wep => 5,
                KeyMgmt::Open => 6,
            }
        }
        if rank(other) < rank(self) { other } else { self }
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
    ssid: Vec<u8>,
    name: String,
    strength: u8,
    key_mgmt: KeyMgmt,
    device: OwnedObjectPath,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Snapshot {
    state: NetState,
    primary: String,
    wifi: bool,
    radio: Radio,
    ethernet: bool,
    wired: bool,
    networks: Vec<WifiNetwork>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Radio {
    #[default]
    On,
    Off,
    HardwareOff,
}

enum Cmd {
    Refresh,
    Connect { ssid: String, password: String, visit: u64 },
    ConnectEnterprise { login: EnterpriseLogin, tls_only: bool, visit: i32 },
    ConnectHidden { ssid: String, key_mgmt: KeyMgmt, password: String },
    SetProxy { url: String, bypass: String, persist: bool, visit: i32 },
    OpenPortal { proxy: Option<(String, String)>, visit: u64 },
    ClosePortal,
    LoadIp(String),
    SaveIp(String, IpSettings),
    EnableWifi,
}

static PAGE_VISIT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

type Failure = &'static str;
const FAILED: Failure = "page-internet-error";
const FAILED_SECRETS: Failure = "page-internet-error-password";
const FAILED_PSK_LENGTH: Failure = "page-internet-error-password-length";
const FAILED_TIMEOUT: Failure = "page-internet-error-timeout";
const FAILED_IP: Failure = "page-internet-ip-error";
const FAILED_IP_RECONNECT: Failure = "page-internet-ip-error-reconnect";
const PROXY_INVALID: Failure = "page-internet-proxy-invalid";
const PROXY_FAILED: Failure = "page-internet-proxy-error";
const PORTAL_BROWSER: &str = "/usr/bin/cog";
/// not taidan's user, which can use the root helper, so a page that escapes `WebKit` can't
const PORTAL_USER: &str = "taidan-portal";
const CHECK_URI: &str = "http://fedoraproject.org/static/hotspot.txt";

const fn bars(strength: u8) -> i32 {
    match strength {
        75.. => 3,
        50..75 => 2,
        25..50 => 1,
        _ => 0,
    }
}

static PORTAL: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);

fn portal() -> std::sync::MutexGuard<'static, Option<std::process::Child>> {
    PORTAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn shutdown() {
    SHUT_DOWN.store(true, std::sync::atomic::Ordering::SeqCst);
    close_portal();
}

static SHUT_DOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

struct Nm {
    conn: Connection,
    manager: nm::NetworkManager<'static>,
    ui: slint::Weak<AppWindow>,
    aps: Vec<Ap>,
    seen_key_mgmt: HashMap<Vec<u8>, KeyMgmt>,
    pending_ip: HashMap<Vec<u8>, IpConfig>,
    last_published: Option<Snapshot>,
    portal_open: bool,
    portal_checked: Option<Instant>,
    portal_saw_offline: bool,
    scanning_until: Option<Instant>,
    rx: Option<mpsc::Receiver<Cmd>>,
    deferred: std::collections::VecDeque<Cmd>,
    proxy_rechecked_for: String,
}

impl Nm {
    fn new(ui: slint::Weak<AppWindow>) -> zbus::Result<Self> {
        let conn = Connection::system()?;
        let manager = zbus::blocking::proxy::Builder::new(&conn)
            .cache_properties(CacheProperties::No)
            .build()?;
        Ok(Self {
            conn,
            manager,
            ui,
            aps: vec![],
            seen_key_mgmt: HashMap::new(),
            pending_ip: HashMap::new(),
            last_published: None,
            portal_open: false,
            portal_checked: None,
            portal_saw_offline: false,
            scanning_until: None,
            rx: None,
            deferred: std::collections::VecDeque::new(),
            proxy_rechecked_for: String::new(),
        })
    }

    fn primary(&self) -> String {
        self.manager.primary_connection().map_or_else(|_| "/".to_owned(), |p| p.as_str().to_owned())
    }

    fn check_uri(&self) -> String {
        (self.manager.connectivity_check_uri().ok())
            .filter(|uri| !uri.is_empty())
            .unwrap_or_else(|| CHECK_URI.to_owned())
    }

    fn recheck_proxy(&self, primary: String) {
        let (ui, uri) = (self.ui.clone(), self.check_uri());
        push(&self.ui, move |net| {
            if net.get_proxy_checked_on().as_str() == primary {
                return;
            }
            let Some(proxy) =
                libtaidan::proxy::Proxy::new(&net.get_proxy_url(), &net.get_proxy_bypass())
            else {
                return;
            };
            std::thread::spawn(move || check_proxy_on(&ui, proxy, &uri, primary));
        });
    }

    fn next_cmd(&mut self, timeout: Duration) -> Result<Cmd, mpsc::RecvTimeoutError> {
        if let Some(cmd) = self.deferred.pop_front() {
            return Ok(cmd);
        }
        let rx = self.rx.as_ref().ok_or(mpsc::RecvTimeoutError::Disconnected)?;
        rx.recv_timeout(timeout)
    }

    fn run_quick_cmds(&mut self) {
        let Some(rx) = self.rx.take() else { return };
        while let Ok(cmd) = rx.try_recv() {
            let quick = matches!(cmd, Cmd::Refresh | Cmd::LoadIp(_) | Cmd::SetProxy { .. });
            if quick && self.deferred.is_empty() {
                let ui = self.ui.clone();
                handle(self, &ui, cmd);
            } else {
                self.deferred.push_back(cmd);
            }
        }
        self.rx = Some(rx);
    }

    fn ssid_of(&self, name: &str) -> Vec<u8> {
        self.aps
            .iter()
            .find(|ap| ap.name == name)
            .map_or_else(|| name.as_bytes().to_vec(), |ap| ap.ssid.clone())
    }

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
        if matches!(snap.state, NetState::Limited | NetState::Portal)
            && snap.primary != "/"
            && self.proxy_rechecked_for != snap.primary
        {
            self.proxy_rechecked_for.clone_from(&snap.primary);
            self.recheck_proxy(snap.primary.clone());
        }
        if self.last_published.as_ref() != Some(&snap) {
            self.last_published = Some(snap.clone());
            push_snapshot(&self.ui, snap);
        }
    }

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

    fn best_device(&self, kind: u32) -> Option<OwnedObjectPath> {
        self.ranked_devices(kind).into_iter().next()
    }

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

    fn wifi_device_for(&self, ssid: &[u8]) -> Option<OwnedObjectPath> {
        (self.aps.iter().find(|ap| ap.ssid == ssid))
            .map(|ap| ap.device.clone())
            .or_else(|| self.best_device(DEVICE_TYPE_WIFI))
    }

    fn scan(&self) {
        for dev in self.devices_of(DEVICE_TYPE_WIFI) {
            if let Err(e) =
                self.at::<nm::Wireless<'_>>(&dev).and_then(|w| w.request_scan(HashMap::new()))
            {
                tracing::debug!(?e, ?dev, "cannot request wi-fi scan");
            }
        }
    }

    fn settings(&self) -> zbus::Result<nm::Settings<'static>> {
        zbus::blocking::proxy::Builder::new(&self.conn).build()
    }

    fn saved(&self) -> Vec<OwnedObjectPath> {
        self.settings().and_then(|s| s.list_connections()).unwrap_or_default()
    }

    fn known(&self) -> HashMap<Vec<u8>, Vec<OwnedObjectPath>> {
        let mut known: HashMap<Vec<u8>, Vec<OwnedObjectPath>> = HashMap::new();
        for path in self.saved() {
            let ssid = self
                .settings_of(&path)
                .and_then(|mut s| s.get_mut("802-11-wireless")?.remove("ssid"))
                .and_then(|ssid| Vec::<u8>::try_from(ssid).ok());
            if let Some(ssid) = ssid {
                known.entry(ssid).or_default().push(path);
            }
        }
        known
    }

    fn known_one(&self, ssid: &[u8]) -> Option<OwnedObjectPath> {
        let saved = self.known().remove(ssid)?;
        let active = (self.devices_of(DEVICE_TYPE_WIFI).iter())
            .filter_map(|dev| self.active_on(dev))
            .find(|active| saved.contains(active));
        active.or_else(|| saved.into_iter().next())
    }

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
                let Ok(ap) = read(&path) else { continue };
                if ap.ssid.is_empty() {
                    continue;
                }
                let connected = on.as_ref() == Some(&ap.ssid);
                match aps.iter_mut().find(|a| a.ssid == ap.ssid) {
                    Some(a)
                        if connected || (a.strength < ap.strength && !active.contains(&a.ssid)) =>
                    {
                        let key_mgmt = a.key_mgmt.merge(ap.key_mgmt);
                        *a = Ap { key_mgmt, ..ap };
                    }
                    Some(a) => a.key_mgmt = a.key_mgmt.merge(ap.key_mgmt),
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
        let primary = self.primary();
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
            return Snapshot {
                state,
                primary,
                wifi: false,
                radio,
                ethernet,
                wired,
                networks: vec![],
            };
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
        networks.sort_by(|a, b| {
            (b.connected, b.strength).cmp(&(a.connected, a.strength)).then(a.ssid.cmp(&b.ssid))
        });
        self.seen_key_mgmt.extend(aps.iter().map(|ap| (ap.ssid.clone(), ap.key_mgmt)));
        self.aps = aps;
        Snapshot { state, primary, wifi: true, radio, ethernet, wired, networks }
    }

    fn delete(&self, path: &ObjectPath<'_>) {
        if let Err(e) = self.at::<nm::SettingsConnection<'_>>(path).and_then(|c| c.delete()) {
            tracing::warn!(?e, ?path, "cannot delete connection");
        }
    }

    /// a failed device moves on to disconnected right away and forgets the reason, so polling
    /// rarely sees it
    fn watch(&self, dev: &ObjectPath<'_>) -> Option<zbus::proxy::SignalStream<'static>> {
        let dev = self.at::<nm::Device<'_>>(dev).ok()?;
        zbus::block_on(dev.inner().inner().receive_signal("StateChanged"))
            .inspect_err(|e| tracing::warn!(?e, "cannot watch the device state"))
            .ok()
    }

    fn wait(
        &mut self,
        path: &ObjectPath<'_>,
        dev: &ObjectPath<'_>,
        mut watch: Option<zbus::proxy::SignalStream<'static>>,
    ) -> Result<(), Failure> {
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        let (active, dev) =
            (self.at::<nm::ActiveConnection<'_>>(path), self.at::<nm::Device<'_>>(dev));
        let mut reason = 0;
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
            if let Ok(dev) = &dev
                && dev.state().is_ok_and(|s| s == DEVICE_STATE_FAILED)
                && let Ok((_, r)) = dev.state_reason()
            {
                reason = r;
            }
            match active.as_ref().map(nm::ActiveConnection::state) {
                Ok(Ok(ACTIVE_STATE_ACTIVATED)) => return Ok(()),
                Ok(Ok(ACTIVE_STATE_DEACTIVATED) | Err(_)) | Err(_) => break,
                Ok(Ok(_)) => {}
            }
            if Instant::now() > deadline {
                if let Err(e) = self.manager.deactivate_connection(path) {
                    tracing::warn!(?e, "cannot stop the connection that took too long");
                }
                return Err(FAILED_TIMEOUT);
            }
            if published.elapsed() >= POLL_ON_PAGE {
                published = Instant::now();
                self.publish();
            }
            self.run_quick_cmds();
            std::thread::sleep(Duration::from_millis(300));
        }
        drain(&mut reason);
        if reason == 0
            && let Ok(dev) = &dev
        {
            reason = dev.state_reason().map_or(0, |(_, r)| r);
        }
        tracing::warn!(reason, "connection failed");
        Err(if REASONS_BAD_SECRETS.contains(&reason) { FAILED_SECRETS } else { FAILED })
    }

    fn settings_of(
        &self,
        path: &ObjectPath<'_>,
    ) -> Option<HashMap<String, HashMap<String, OwnedValue>>> {
        self.at::<nm::SettingsConnection<'_>>(path).ok()?.get_settings().ok()
    }

    fn add_and_connect(
        &mut self,
        ssid: &[u8],
        mut settings: nm::ConnSettings<'_>,
    ) -> Result<(), Failure> {
        let dev = self.wifi_device_for(ssid).ok_or(FAILED)?;
        let old = self.known().remove(ssid).unwrap_or_default();
        let was_active = self.active_on(&dev).filter(|active| old.contains(active));
        let ip = self.pending_ip.get(ssid).cloned().or_else(|| {
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
        let res = self.replace(&dev, settings, old, was_active);
        if res.is_ok() {
            self.pending_ip.remove(ssid);
        }
        res
    }

    fn replace(
        &mut self,
        dev: &ObjectPath<'_>,
        settings: nm::ConnSettings<'_>,
        old: Vec<OwnedObjectPath>,
        was_active: Option<OwnedObjectPath>,
    ) -> Result<(), Failure> {
        let root = ObjectPath::from_static_str_unchecked("/");
        let watch = self.watch(dev);
        let (conn, active) =
            self.manager.add_and_activate_connection(settings, dev, &root).map_err(|e| {
                tracing::error!(?e, "cannot add connection");
                FAILED
            })?;
        let res = self.wait(&active, dev, watch);
        if res.is_err() {
            self.delete(&conn);
            if let Some(old) = was_active
                && let Err(e) = self.manager.activate_connection(&old, dev, &root)
            {
                tracing::warn!(?e, "cannot reactivate the previous connection");
            }
        } else {
            for old in old {
                self.delete(&old);
            }
        }
        res
    }

    fn activate(&mut self, conn: &ObjectPath<'_>, dev: &ObjectPath<'_>) -> Result<(), Failure> {
        let root = ObjectPath::from_static_str_unchecked("/");
        let watch = self.watch(dev);
        let active = self.manager.activate_connection(conn, dev, &root).map_err(|e| {
            tracing::error!(?e, "cannot activate connection");
            FAILED
        })?;
        self.wait(&active, dev, watch)
    }

    fn connect(&mut self, name: &str, password: &str) -> Result<(), Failure> {
        let ssid = self.ssid_of(name);
        let dev = self.wifi_device_for(&ssid).ok_or(FAILED)?;
        let saved = self.known_one(&ssid);
        if password.is_empty()
            && let Some(saved) = &saved
        {
            return self.activate(saved, &dev);
        }
        let saved = saved.and_then(|path| saved_security(&self.settings_of(&path)?));
        let (key_mgmt, hidden) = match self.aps.iter().find(|ap| ap.ssid == ssid) {
            Some(ap) => (ap.key_mgmt, saved.is_some_and(|(_, hidden)| hidden)),
            None => saved.ok_or_else(|| {
                tracing::warn!(name, "network is gone");
                FAILED
            })?,
        };
        check_psk_length(key_mgmt, password)?;
        let mut settings = wifi_settings(&ssid, name, key_mgmt, password);
        if hidden {
            mark_hidden(&mut settings);
        }
        self.add_and_connect(&ssid, settings)
    }

    fn connect_hidden(
        &mut self,
        ssid: &str,
        key_mgmt: KeyMgmt,
        password: &str,
    ) -> Result<(), Failure> {
        check_psk_length(key_mgmt, password)?;
        let mut settings = wifi_settings(ssid.as_bytes(), ssid, key_mgmt, password);
        mark_hidden(&mut settings);
        self.add_and_connect(ssid.as_bytes(), settings)
    }
}

fn check_psk_length(key_mgmt: KeyMgmt, password: &str) -> Result<(), Failure> {
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
            _ => return None,
        },
    };
    let hidden =
        get("802-11-wireless", "hidden").is_some_and(|h| h.downcast_ref::<bool>() == Ok(true));
    Some((key_mgmt, hidden))
}

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

fn mark_hidden(settings: &mut nm::ConnSettings<'_>) {
    if let Some(wifi) = settings.get_mut("802-11-wireless") {
        wifi.insert("hidden", true.into());
    }
}

const WIRED_8021X_ID: &str = "Wired 802.1X";
const ABOVE_DEFAULT_WIRED_PRIORITY: i32 = 10;

fn realm(username: &str) -> &str {
    username.rsplit_once('@').map_or("", |(_, realm)| realm.trim())
}

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
                    ("autoconnect-priority", Value::from(ABOVE_DEFAULT_WIRED_PRIORITY)),
                ]),
            ),
            ("802-3-ethernet", HashMap::new()),
        ])
    } else {
        let mut settings = base_settings(ssid, &login.ssid);
        if hidden {
            mark_hidden(&mut settings);
        }
        // `wpa-eap` only tries Suite-B-192 with PMF required, which `wpa-eap-suite-b-192` turns on
        let key_mgmt =
            if key_mgmt == KeyMgmt::EnterpriseSuiteB { "wpa-eap-suite-b-192" } else { "wpa-eap" };
        settings.insert("802-11-wireless-security", HashMap::from([("key-mgmt", key_mgmt.into())]));
        settings
    };
    settings.insert("802-1x", eap_settings(login));
    settings
}

fn cert_path(path: &str) -> Value<'static> {
    Value::from(format!("file://{path}\0").into_bytes())
}

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
            x.insert("anonymous-identity", login.anonymous_identity.as_str().into());
        }
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
            x.insert("domain-suffix-match", login.domain.as_str().into());
        }
    }
    if login.method == EapMethod::Tls {
        x.insert("client-cert", cert_path(&login.client_cert));
        let key_or_pkcs12_bundle =
            if login.private_key.is_empty() { &login.client_cert } else { &login.private_key };
        x.insert("private-key", cert_path(key_or_pkcs12_bundle));
        if login.private_key_password.is_empty() {
            x.insert("private-key-password-flags", Value::from(SECRET_FLAG_NOT_REQUIRED));
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

use libtaidan::helper::CERT_DIR;

fn is_pkcs12(path: &str) -> bool {
    let ext = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or_default();
    ext.eq_ignore_ascii_case("p12") || ext.eq_ignore_ascii_case("pfx")
}

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

struct Certs {
    network: String,
    attempt: Option<String>,
}

impl Certs {
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
        if path.is_empty() {
            continue;
        }
        let file = format!("{kind}{}", helper_accepted_extension(path));
        let dest = format!("{CERT_DIR}/{network}/{stamp}/{file}");
        certs.attempt = Some(stamp.clone());
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

fn helper_accepted_extension(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty() && e.len() <= 8 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(|e| format!(".{}", e.to_ascii_lowercase()))
        .unwrap_or_default()
}

const CERT_EXTS: [&str; 7] = ["der", "pem", "crt", "cer", "p12", "pfx", "key"];

fn list_dir(dir: &std::path::Path, pac: bool, all: bool) -> Vec<crate::ui::FileEntry> {
    let Ok(read) = std::fs::read_dir(dir) else { return vec![] };
    let mut entries = read
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
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

fn check_proxy(proxy: &libtaidan::proxy::Proxy, uri: &str) -> Result<(), Failure> {
    let client = proxy.client().map_err(|_| PROXY_INVALID)?;
    let res = crate::RT.block_on(async {
        client.get(uri).timeout(Duration::from_secs(15)).send().await?.error_for_status()
    });
    res.map(|_| ()).map_err(|e| {
        tracing::warn!(?e, uri, "cannot reach the internet through the proxy");
        PROXY_FAILED
    })
}

fn check_proxy_on(
    ui: &slint::Weak<AppWindow>,
    proxy: libtaidan::proxy::Proxy,
    uri: &str,
    primary: String,
) {
    if check_proxy(&proxy, uri).is_ok() {
        push(ui, move |net| {
            if net.get_proxy_url().as_str() == proxy.url {
                net.set_proxy_checked_on(primary.into());
            }
        });
    }
}

const DEFAULT_BYPASS: &str = "localhost, 127.0.0.1, ::1";

fn set_proxy(
    nm: &Nm,
    ui: &slint::Weak<AppWindow>,
    url: String,
    bypass: String,
    persist: bool,
    visit: i32,
) {
    let (uri, primary, ui) = (nm.check_uri(), nm.primary(), ui.clone());
    std::thread::spawn(move || {
        let res = if url.is_empty() {
            Ok(None)
        } else {
            libtaidan::proxy::Proxy::new(&url, &bypass)
                .ok_or(PROXY_INVALID)
                .and_then(|proxy| check_proxy(&proxy, &uri).map(|()| Some(proxy)))
        };
        push(&ui, move |net| {
            if !net.get_proxy_open() || net.get_proxy_visit() != visit {
                return;
            }
            net.set_proxy_busy(false);
            match res {
                Ok(proxy) => {
                    let public = proxy.as_ref().map(libtaidan::proxy::Proxy::public_url);
                    net.set_proxy_public(public.unwrap_or_default().into());
                    let (url, bypass) = proxy.map_or_else(
                        || (String::new(), DEFAULT_BYPASS.to_owned()),
                        |p| (p.url, p.no_proxy.replace(',', ", ")),
                    );
                    net.set_proxy_checked_on(
                        if url.is_empty() { "" } else { primary.as_str() }.into(),
                    );
                    net.set_proxy_url(url.into());
                    net.set_proxy_persist(persist);
                    net.set_proxy_bypass(bypass.into());
                    net.set_proxy_open(false);
                }
                Err(failure) => net.set_proxy_error_id(failure.into()),
            }
        });
    });
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct IpConfig {
    manual: Option<(Ipv4Addr, u32, Option<Ipv4Addr>)>,
    dns4: Vec<Ipv4Addr>,
    dns6: Vec<Ipv6Addr>,
}

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
        if cfg.dns4.is_empty() && cfg.dns6.is_empty() {
            return Err("page-internet-ip-error-dns-needed");
        }
        cfg.manual = Some((addr, prefix, gateway));
        Ok(cfg)
    }

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

    fn from_settings(settings: &HashMap<String, HashMap<String, OwnedValue>>) -> Self {
        let mut cfg = Self::default();
        let v4 = settings.get("ipv4");
        let get = |key| v4.and_then(|v4| v4.get(key));
        let dns = |section: Option<&HashMap<String, OwnedValue>>| -> Option<Vec<IpAddr>> {
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
                    .collect(),
            )
        };
        for server in [dns(v4), dns(settings.get("ipv6"))].into_iter().flatten().flatten() {
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
    fn open_portal(&mut self, proxy: Option<(String, String)>) -> bool {
        if SHUT_DOWN.load(std::sync::atomic::Ordering::SeqCst) {
            return false;
        }
        if portal().is_some() {
            return true;
        }
        let uri = self.check_uri();
        let Some((mut cmd, request)) = portal_command(proxy, &uri) else { return false };
        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write as _;
                    if let Err(e) = stdin.write_all(&request) {
                        tracing::error!(?e, "cannot pass the request to the helper");
                    }
                }
                tracing::info!(uri, "Opened the captive portal browser");
                {
                    let mut slot = portal();
                    *slot = Some(child);
                    if SHUT_DOWN.load(std::sync::atomic::Ordering::SeqCst) {
                        drop(slot);
                        close_portal();
                        return false;
                    }
                }
                self.portal_checked = None;
                self.portal_saw_offline = self.manager.connectivity() != Ok(CONNECTIVITY_FULL);
                true
            }
            Err(e) => {
                tracing::error!(?e, "cannot open {PORTAL_BROWSER} for the captive portal");
                false
            }
        }
    }

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
        if !closed && self.portal_checked.is_some_and(|at| at.elapsed() < PORTAL_RECHECK_INTERVAL) {
            return open;
        }
        self.portal_checked = Some(Instant::now());
        // NetworkManager only rechecks every few minutes on its own
        let online = match self.manager.check_connectivity() {
            Ok(connectivity) => connectivity == CONNECTIVITY_FULL,
            Err(e) => {
                tracing::debug!(?e, "cannot ask NetworkManager to check connectivity");
                self.manager.state().is_ok_and(|s| s == STATE_CONNECTED_GLOBAL)
            }
        };
        if !online {
            self.portal_saw_offline = true;
        }
        if online && open && self.portal_saw_offline {
            tracing::info!("Online, closing the captive portal browser");
            close_portal();
        }
        portal().is_some()
    }

    fn connect_wired_8021x(&mut self, login: &EnterpriseLogin) -> Result<(), Failure> {
        let dev = self.ethernet_device().ok_or(FAILED)?;
        let old = (self.saved().into_iter())
            .filter(|path| {
                let id = self
                    .settings_of(path)
                    .and_then(|mut s| s.get_mut("connection")?.remove("id"))
                    .and_then(|id| String::try_from(id).ok());
                id.as_deref() == Some(WIRED_8021X_ID)
            })
            .collect::<Vec<_>>();
        let was_active = self.active_on(&dev).filter(|active| old.contains(active));
        let settings = enterprise_settings(login, &[], false, KeyMgmt::Enterprise);
        self.replace(&dev, settings, old, was_active)
    }

    fn ethernet_device(&self) -> Option<OwnedObjectPath> {
        self.best_device(DEVICE_TYPE_ETHERNET)
    }

    fn active_on(&self, dev: &ObjectPath<'_>) -> Option<OwnedObjectPath> {
        let active = self.at::<nm::Device<'_>>(dev).ok()?.active_connection().ok()?;
        if active.as_str() == "/" {
            return None;
        }
        self.at::<nm::ActiveConnection<'_>>(&active).ok()?.connection().ok()
    }

    fn ethernet_connection(&self, dev: &ObjectPath<'_>) -> Option<OwnedObjectPath> {
        let device = self.at::<nm::Device<'_>>(dev).ok()?;
        self.active_on(dev)
            .or_else(|| device.available_connections().ok()?.into_iter().next())
            .or_else(|| {
                let iface = device.interface().ok()?;
                let mac = self
                    .at::<nm::Wired<'_>>(dev)
                    .and_then(|w| w.perm_hw_address())
                    .ok()
                    .and_then(|a| parse_mac(&a));
                self.settings().ok()?.list_connections().ok()?.into_iter().find(|path| {
                    let Some(settings) = self.settings_of(path) else { return false };
                    let Some(connection) = settings.get("connection") else { return false };
                    let text = |key: &str| {
                        connection.get(key).and_then(|v| String::try_from(v.clone()).ok())
                    };
                    let is_port = ["controller", "master"]
                        .iter()
                        .any(|key| text(key).is_some_and(|c| !c.is_empty()));
                    let bound_to_other_card = (settings.get("802-3-ethernet"))
                        .and_then(|e| e.get("mac-address"))
                        .and_then(|v| Vec::<u8>::try_from(v.clone()).ok())
                        .is_some_and(|m| {
                            !m.is_empty() && mac.as_ref().is_some_and(|mac| *mac != m)
                        });
                    text("type").as_deref() == Some("802-3-ethernet")
                        && text("interface-name")
                            .is_none_or(|name| name.is_empty() || name == iface)
                        && !is_port
                        && !bound_to_other_card
                })
            })
    }

    fn ip_connection(&self, ssid: &str) -> Option<OwnedObjectPath> {
        if ssid.is_empty() {
            let dev = self.ethernet_device()?;
            self.ethernet_connection(&dev)
        } else {
            self.known_one(&self.ssid_of(ssid))
        }
    }

    fn load_ip(&self, ssid: &str) -> IpConfig {
        let settings = self.ip_connection(ssid).and_then(|path| self.settings_of(&path));
        settings.map_or_else(
            || self.pending_ip.get(&self.ssid_of(ssid)).cloned().unwrap_or_default(),
            |settings| IpConfig::from_settings(&settings),
        )
    }

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
            let v6 = name == "ipv6";
            // NetworkManager refuses DNS servers without IPv6 addressing
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
                if let Ok(value) = OwnedValue::try_from(value) {
                    section.insert(key.to_owned(), value);
                }
            }
        }
        conn.update(settings)
    }

    fn save_ip(&mut self, ssid: &str, ip: IpConfig) -> Result<(), Failure> {
        let failed = |e: zbus::Error| {
            tracing::error!(?e, ssid, "cannot save IP settings");
            FAILED_IP
        };
        if !ssid.is_empty() {
            let bytes = self.ssid_of(ssid);
            let Some(conn) = self.known_one(&bytes) else {
                self.pending_ip.insert(bytes, ip);
                return Ok(());
            };
            self.pending_ip.remove(&bytes);
            self.update_ip(&conn, &ip).map_err(failed)?;
            for dev in self.devices_of(DEVICE_TYPE_WIFI) {
                if self.active_on(&dev).as_ref() == Some(&conn) {
                    return self.activate(&conn, &dev).map_err(|_| FAILED_IP_RECONNECT);
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
            self.settings().map_err(failed)?.add_connection(settings).map_err(failed)?
        };
        let plugged = self
            .at::<nm::Device<'_>>(&dev)
            .and_then(|d| d.state())
            .is_ok_and(|s| s >= DEVICE_STATE_DISCONNECTED);
        if plugged { self.activate(&conn, &dev).map_err(|_| FAILED_IP_RECONNECT) } else { Ok(()) }
    }
}

fn parse_mac(s: &str) -> Option<Vec<u8>> {
    s.split(':')
        .map(|b| u8::from_str_radix(b, 16).ok())
        .collect::<Option<Vec<_>>>()
        .filter(|b| b.len() == 6)
}

fn portal_command(
    proxy: Option<(String, String)>,
    uri: &str,
) -> Option<(std::process::Command, Vec<u8>)> {
    let runtime = std::env::var("XDG_RUNTIME_DIR").ok().filter(|d| d.starts_with('/'))?;
    let display = std::env::var("WAYLAND_DISPLAY").ok().filter(|d| !d.is_empty())?;
    let socket = if display.starts_with('/') { display } else { format!("{runtime}/{display}") };
    if !fits_bind_paths(&socket) {
        tracing::error!(socket, "cannot give the portal browser a display at this path");
        return None;
    }
    let acl = format!("u:{PORTAL_USER}:rw");
    match std::process::Command::new("setfacl").args(["-m", &acl, "--", &socket]).status() {
        Ok(status) if status.success() => {}
        res => {
            tracing::error!(?res, socket, "cannot let the portal browser use the display");
            return None;
        }
    }
    if let Err(e) = stop_portal_unit() {
        tracing::debug!(?e, "no earlier portal browser to stop");
    }
    let lang = std::env::var("LANG").ok();
    let proxy = proxy.and_then(|(url, bypass)| libtaidan::proxy::Proxy::new(&url, &bypass));
    let verb = libtaidan::Verb::StartPortal { socket, lang, uri: uri.to_owned() };
    let mut cmd = libtaidan::helper::std_command(&verb);
    let stdin = libtaidan::helper::stdin_with_proxy(&verb, &[], proxy.as_ref());
    cmd.stdin(std::process::Stdio::piped());
    Some((cmd, stdin))
}

fn fits_bind_paths(path: &str) -> bool {
    !path.contains(|c: char| c == ':' || c.is_whitespace())
}

fn stop_portal_unit() -> libtaidan::prelude::Res<()> {
    std::thread::spawn(|| crate::RT.block_on(libtaidan::root(libtaidan::Verb::StopPortal)))
        .join()
        .unwrap_or_else(|_| Err(libtaidan::prelude::Err::Msg("stopping the portal panicked")))
}

fn close_portal() {
    let Some(mut child) = portal().take() else { return };
    if let Err(e) = stop_portal_unit() {
        tracing::warn!(?e, "cannot stop the captive portal browser");
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    tracing::warn!("systemd-run didn't end with the captive portal browser");
}

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
        net.set_primary_connection(snap.primary.into());
        net.set_wifi_available(snap.wifi);
        net.set_wifi_enabled(snap.radio == Radio::On);
        net.set_wifi_hardware_enabled(snap.radio != Radio::HardwareOff);
        net.set_ethernet(snap.ethernet);
        net.set_wired(snap.wired);
        net.set_connected_ssid(
            snap.networks.iter().find(|n| n.connected).map(|n| n.ssid.clone()).unwrap_or_default(),
        );
        let model = net.get_networks();
        if let Some(model) = model.as_any().downcast_ref::<slint::VecModel<WifiNetwork>>() {
            sync_model(model, snap.networks, |n| n.ssid.clone());
        } else {
            net.set_networks(slint::ModelRc::new(slint::VecModel::from(snap.networks)));
        }
    });
}

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
            nm.scanning_until = Some(Instant::now() + SCAN_RESULTS_DELAY);
            return;
        }
        Cmd::Connect { ssid, password, visit } => {
            start_connecting(ui, &ssid);
            let res = nm.connect(&ssid, &password);
            let security =
                nm.aps.iter().find(|ap| ap.name == ssid).map(|ap| ap.key_mgmt.security());
            // on an open network a supplicant disconnect means something else, like a MAC filter
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
        Cmd::SetProxy { url, bypass, persist, visit } => {
            return set_proxy(nm, ui, url, bypass, persist, visit);
        }
        Cmd::OpenPortal { proxy, visit } => {
            if PAGE_VISIT.load(std::sync::atomic::Ordering::SeqCst) != visit {
                return;
            }
            let open = nm.open_portal(proxy);
            push(ui, move |net| net.set_portal_open(open));
            return;
        }
        Cmd::ClosePortal => {
            close_portal();
            nm.portal_checked = None;
            push(ui, |net| net.set_portal_open(false));
            return;
        }
        Cmd::LoadIp(ssid) => return load_ip_into_dialog(nm, ui, ssid),
        Cmd::SaveIp(ssid, ip) => return save_ip_from_dialog(nm, ui, ssid, &ip),
        Cmd::EnableWifi => return enable_wifi(nm, ui),
    };
    push(ui, move |net| {
        net.set_connecting_ssid("".into());
        match res {
            Ok(()) => net.set_password_ssid("".into()),
            Err(failure) => {
                net.set_error_ssid(ssid.as_str().into());
                net.set_error_id(failure.into());
                let page_left = PAGE_VISIT.load(std::sync::atomic::Ordering::SeqCst) != visit;
                let other_dialog = net.get_proxy_open()
                    || net.get_ip_open()
                    || net.get_hidden_open()
                    || net.get_enterprise_open()
                    || net.get_offline_prompt()
                    || !["", ssid.as_str()].contains(&net.get_password_ssid().as_str());
                let dismissed = net.get_dismissed_ssid().as_str() == ssid.as_str();
                if failure == FAILED_SECRETS && !page_left && !other_dialog && !dismissed {
                    if let Some(Security::Enterprise | Security::EnterpriseTls) = security {
                        let tls_only = security == Some(Security::EnterpriseTls);
                        net.invoke_open_enterprise(ssid.as_str().into(), false, tls_only);
                        net.set_enterprise_error_id(failure.into());
                    } else {
                        net.set_password_ssid(ssid.as_str().into());
                    }
                }
            }
        }
    });
}

fn load_ip_into_dialog(nm: &Nm, ui: &slint::Weak<AppWindow>, ssid: String) {
    let ip = nm.load_ip(&ssid).display();
    push(ui, move |net| {
        if !net.get_ip_open() || net.get_ip_ssid().as_str() != ssid.as_str() {
            return;
        }
        net.set_ip(ip);
        net.set_ip_loading(false);
    });
}

fn save_ip_from_dialog(nm: &mut Nm, ui: &slint::Weak<AppWindow>, ssid: String, ip: &IpSettings) {
    let res = IpConfig::parse(ip).and_then(|ip| nm.save_ip(&ssid, ip));
    push(ui, move |net| {
        net.set_ip_busy(false);
        if !net.get_ip_open() || net.get_ip_ssid().as_str() != ssid.as_str() {
            return;
        }
        match res {
            Ok(()) => net.set_ip_open(false),
            Err(failure) => net.set_ip_error_id(failure.into()),
        }
    });
}

fn connect_enterprise(
    nm: &mut Nm,
    ui: &slint::Weak<AppWindow>,
    mut login: EnterpriseLogin,
    tls_only: bool,
    visit: i32,
) {
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
                None => match nm.seen_key_mgmt.get(&ssid) {
                    Some(&key_mgmt) => (false, key_mgmt),
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
        if !net.get_enterprise_open() || net.get_enterprise_visit() != visit {
            return;
        }
        match res {
            Ok(()) => net.set_enterprise_open(false),
            Err(failure) => net.set_enterprise_error_id(failure.into()),
        }
    });
}

fn enable_wifi(nm: &mut Nm, ui: &slint::Weak<AppWindow>) {
    if let Err(e) = nm.manager.set_wireless_enabled(true) {
        tracing::error!(?e, "cannot turn on wi-fi");
        return;
    }
    push(ui, |net| net.set_scanning(true));
    nm.scanning_until = Some(Instant::now() + SCAN_RESULTS_DELAY * 2);
}

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

fn worker(ui: &slint::Weak<AppWindow>, rx: mpsc::Receiver<Cmd>) {
    let Some(mut nm) = connect_nm(ui, &rx) else { return };
    nm.rx = Some(rx);
    nm.scan();
    nm.publish();
    loop {
        if SHUT_DOWN.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        let poll = if ON_PAGE.load(std::sync::atomic::Ordering::Relaxed) {
            POLL_ON_PAGE
        } else {
            POLL_OFF_PAGE
        };
        let timeout = (nm.scanning_until)
            .map_or(poll, |until| until.saturating_duration_since(Instant::now()).min(poll));
        match nm.next_cmd(timeout) {
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

fn browse(ui: &slint::Weak<AppWindow>, dir: &str, pac: bool, all: bool) {
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

fn spawn_worker(ui: &slint::Weak<AppWindow>) -> mpsc::Sender<Cmd> {
    let (tx, rx) = mpsc::channel();
    let ui = ui.clone();
    std::thread::spawn(move || {
        let _guard = WorkerGone(ui.clone());
        worker(&ui, rx);
    });
    tx
}

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
            let visit = ui.upgrade().map_or(0, |ui| {
                let net = ui.global::<Network<'_>>();
                net.set_proxy_busy(true);
                net.set_proxy_error_id("".into());
                net.get_proxy_visit()
            });
            send(Cmd::SetProxy { url: url.trim().into(), bypass: bypass.into(), persist, visit });
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
            std::thread::spawn(close_portal);
            if let Some(ui) = ui.upgrade() {
                let net = ui.global::<Network<'_>>();
                net.set_portal_open(false);
                save(&net);
            }
        }
    });
}

fn save(net: &Network<'_>) {
    net.set_password_ssid("".into());
    net.set_enterprise_open(false);
    let mut settings = crate::SETTINGS.write();
    settings.proxy = libtaidan::proxy::Proxy::new(&net.get_proxy_url(), &net.get_proxy_bypass());
    net.get_device_name().trim().clone_into(&mut settings.device_name);
    settings.hostname = net.get_hostname().into();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_mgmt_merges_across_bands() {
        let psk = KeyMgmt::from_flags(AP_FLAGS_PRIVACY, 0, AP_SEC_KEY_MGMT_PSK);
        let sae = KeyMgmt::from_flags(AP_FLAGS_PRIVACY, 0, AP_SEC_KEY_MGMT_SAE);
        assert_eq!(sae.merge(psk), KeyMgmt::Psk);
        assert_eq!(psk.merge(sae), KeyMgmt::Psk);
        assert_eq!(sae.merge(sae), KeyMgmt::Sae);
        for (a, b) in [
            (AP_SEC_KEY_MGMT_OWE, 0),
            (AP_SEC_KEY_MGMT_SAE, AP_SEC_KEY_MGMT_OWE),
            (AP_SEC_KEY_MGMT_802_1X, AP_SEC_KEY_MGMT_PSK),
            (AP_SEC_KEY_MGMT_EAP_SUITE_B_192, AP_SEC_KEY_MGMT_SAE),
        ] {
            let one = KeyMgmt::from_flags(AP_FLAGS_PRIVACY, 0, a | b);
            let merged =
                KeyMgmt::from_flags(AP_FLAGS_PRIVACY, 0, a).merge(KeyMgmt::from_flags(0, 0, b));
            assert_eq!(merged, one, "{a:#x} {b:#x}");
        }
        assert_eq!(KeyMgmt::Open.merge(KeyMgmt::Wep), KeyMgmt::Wep);
    }

    #[test]
    fn hostnames() {
        assert_eq!(derive_hostname("Jane's Laptop"), "janes-laptop");
        assert_eq!(derive_hostname("  -- My  PC 2! "), "my-pc-2");
        assert_eq!(derive_hostname("ノートパソコン"), "");
        assert_eq!(device_hostname("Jane's Laptop", "jane"), "janes-laptop");
        assert_eq!(device_hostname("ノートパソコン", "taro"), "taro-pc");
        assert_eq!(device_hostname("ノートパソコン", "taro_y"), "taro-y-pc");
        assert_eq!(device_hostname("  ", "taro"), "");
        assert_eq!(device_hostname("ノートパソコン", ""), "");
    }

    #[test]
    fn cert_names_are_unique() {
        assert_eq!(cert_name(true, "ignored"), "wired");
        assert_eq!(cert_name(false, "Home"), "home-486f6d65");
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
        assert_eq!(IpConfig::from_settings(&owned(cfg.sections().into())), cfg);
        let shown = cfg.display();
        assert_eq!(shown.address, "192.168.1.20/24");
        assert_eq!(shown.dns, "1.1.1.1, 2606:4700:4700::1111");
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
        assert_eq!(check_psk_length(KeyMgmt::Psk, "short"), Err(FAILED_PSK_LENGTH));
        assert_eq!(check_psk_length(KeyMgmt::Psk, "12345678"), Ok(()));
        assert_eq!(check_psk_length(KeyMgmt::Psk, &"a".repeat(64)), Ok(()));
        assert_eq!(check_psk_length(KeyMgmt::Psk, &"g".repeat(64)), Err(FAILED_PSK_LENGTH));
        assert_eq!(check_psk_length(KeyMgmt::Psk, &"a".repeat(65)), Err(FAILED_PSK_LENGTH));
        assert_eq!(check_psk_length(KeyMgmt::Sae, "short"), Ok(()));
        assert_eq!(check_psk_length(KeyMgmt::Open, ""), Ok(()));
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
        assert_eq!(x["private-key-password-flags"], Value::from(SECRET_FLAG_NOT_REQUIRED));
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
