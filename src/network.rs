//! wi-fi and connectivity through the D-Bus API of `NetworkManager`
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::mpsc;
use std::time::{Duration, Instant};

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
        #[zbus(property)]
        fn state(&self) -> zbus::Result<u32>;
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
    /// old WEP security, which isn't supported
    Wep,
}

impl KeyMgmt {
    const fn from_flags(flags: u32, wpa: u32, rsn: u32) -> Self {
        let sec = wpa | rsn;
        if sec & (AP_SEC_KEY_MGMT_802_1X | AP_SEC_KEY_MGMT_EAP_SUITE_B_192) != 0 {
            Self::Enterprise
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
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Snapshot {
    state: NetState,
    wifi: bool,
    ethernet: bool,
    wired: bool,
    networks: Vec<WifiNetwork>,
}

enum Cmd {
    Refresh,
    Connect {
        ssid: String,
        password: String,
    },
    ConnectEnterprise(EnterpriseLogin),
    /// a network that doesn't announce its name, so it isn't in the list
    ConnectHidden {
        ssid: String,
        key_mgmt: KeyMgmt,
        password: String,
    },
    SetProxy {
        url: String,
        bypass: String,
    },
    /// with the proxy and the hosts it's not used for, if one is set
    OpenPortal(Option<(String, String)>),
    ClosePortal,
    LoadIp(String),
    SaveIp(String, IpSettings),
}

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
const PORTAL_BROWSER: &str = "cog";
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
    close_portal();
}

struct Nm {
    conn: Connection,
    manager: nm::NetworkManager<'static>,
    ui: slint::Weak<AppWindow>,
    /// access points from the last refresh, strongest per network
    aps: Vec<Ap>,
    /// IP settings for wi-fi networks without a saved connection yet by SSID, used once connecting
    pending_ip: HashMap<Vec<u8>, IpConfig>,
    /// the last snapshot shown
    last: Option<Snapshot>,
    portal_open: bool,
    /// when `NetworkManager` last checked connectivity for the portal browser
    portal_checked: Option<Instant>,
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
            pending_ip: HashMap::new(),
            last: None,
            portal_open: false,
            portal_checked: None,
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

    fn wifi_device(&self) -> Option<OwnedObjectPath> {
        self.devices_of(DEVICE_TYPE_WIFI).into_iter().next()
    }

    fn scan(&self) {
        let Some(dev) = self.wifi_device() else { return };
        // scans that come too soon after another are refused, which is fine
        if let Err(e) =
            self.at::<nm::Wireless<'_>>(&dev).and_then(|w| w.request_scan(HashMap::new()))
        {
            tracing::debug!(?e, "cannot request wi-fi scan");
        }
    }

    /// saved wi-fi connections by SSID
    fn known(&self) -> HashMap<Vec<u8>, OwnedObjectPath> {
        let settings: zbus::Result<nm::Settings<'_>> =
            zbus::blocking::proxy::Builder::new(&self.conn).build();
        let paths = settings.and_then(|s| s.list_connections()).unwrap_or_default();
        paths
            .into_iter()
            .filter_map(|path| {
                let mut settings =
                    self.at::<nm::SettingsConnection<'_>>(&path).ok()?.get_settings().ok()?;
                let ssid = settings.get_mut("802-11-wireless")?.remove("ssid")?;
                Some((Vec::<u8>::try_from(ssid).ok()?, path))
            })
            .collect()
    }

    fn access_points(&self, dev: &ObjectPath<'_>) -> (Vec<Ap>, Option<Vec<u8>>) {
        let Ok(wireless) = self.at::<nm::Wireless<'_>>(dev) else { return (vec![], None) };
        let read = |path: &ObjectPath<'_>| -> zbus::Result<Ap> {
            let ap = self.at::<nm::AccessPoint<'_>>(path)?;
            let ssid = ap.ssid()?;
            Ok(Ap {
                name: String::from_utf8_lossy(&ssid).into_owned(),
                ssid,
                strength: ap.strength()?,
                key_mgmt: KeyMgmt::from_flags(ap.flags()?, ap.wpa_flags()?, ap.rsn_flags()?),
            })
        };
        let active = wireless
            .active_access_point()
            .ok()
            .filter(|path| path.as_str() != "/")
            .and_then(|path| read(&path).ok())
            .map(|ap| ap.ssid);
        let mut aps: Vec<Ap> = vec![];
        for path in wireless.get_all_access_points().unwrap_or_default() {
            // hidden networks have no name to show
            let Ok(ap) = read(&path) else { continue };
            if ap.ssid.is_empty() {
                continue;
            }
            match aps.iter_mut().find(|a| a.ssid == ap.ssid) {
                Some(a) if a.strength < ap.strength => *a = ap,
                Some(_) => {}
                None => aps.push(ap),
            }
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
        let Some(dev) = self.wifi_device() else {
            self.aps.clear();
            return Snapshot { state, wifi: false, ethernet, wired, networks: vec![] };
        };
        let (aps, active) = self.access_points(&dev);
        let known = self.known();
        let mut networks = aps
            .iter()
            .map(|ap| WifiNetwork {
                ssid: ap.name.as_str().into(),
                strength: bars(ap.strength),
                security: ap.key_mgmt.security(),
                known: known.contains_key(&ap.ssid),
                connected: active.as_deref() == Some(ap.ssid.as_slice()),
            })
            .collect::<Vec<_>>();
        // bucketed strength keeps the list from reshuffling on every small change
        networks.sort_by(|a, b| {
            (b.connected, b.strength).cmp(&(a.connected, a.strength)).then(a.ssid.cmp(&b.ssid))
        });
        self.aps = aps;
        Snapshot { state, wifi: true, ethernet, wired, networks }
    }

    fn delete(&self, path: &ObjectPath<'_>) {
        if let Err(e) = self.at::<nm::SettingsConnection<'_>>(path).and_then(|c| c.delete()) {
            tracing::warn!(?e, ?path, "cannot delete connection");
        }
    }

    /// waits for `active` to come up on `dev`, showing the state meanwhile
    fn wait(&mut self, active: &ObjectPath<'_>, dev: &ObjectPath<'_>) -> Result<(), Failure> {
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        let (active, dev) =
            (self.at::<nm::ActiveConnection<'_>>(active), self.at::<nm::Device<'_>>(dev));
        let mut reason = 0;
        let mut published = Instant::now();
        loop {
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
                return Err(FAILED_TIMEOUT);
            }
            // the network list and the end of a scan, which would wait for this otherwise
            if published.elapsed() >= POLL {
                published = Instant::now();
                self.publish();
            }
            std::thread::sleep(Duration::from_millis(300));
        }
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
        let dev = self.wifi_device().ok_or(FAILED)?;
        let old = self.known().remove(ssid);
        let ip = self.pending_ip.get(ssid).cloned().or_else(|| {
            let old = self.settings_of(old.as_ref()?)?;
            Some(IpConfig::from_settings(&old)).filter(|ip| *ip != IpConfig::default())
        });
        if let Some(ip) = ip {
            for (name, values) in ip.sections() {
                settings.entry(name).or_default().extend(values);
            }
        }
        let was_active = old.is_some() && self.active_on(&dev) == old;
        let root = ObjectPath::from_static_str_unchecked("/");
        let (conn, active) =
            self.manager.add_and_activate_connection(settings, &dev, &root).map_err(|e| {
                let ssid = String::from_utf8_lossy(ssid);
                tracing::error!(?e, %ssid, "cannot add connection");
                FAILED
            })?;
        let res = self.wait(&active, &dev);
        if res.is_err() {
            self.delete(&conn);
            // go back to the network it was on before
            if was_active
                && let Some(old) = &old
                && let Err(e) = self.manager.activate_connection(old, &dev, &root)
            {
                tracing::warn!(?e, "cannot reactivate the previous connection");
            }
        } else if let Some(old) = old {
            self.delete(&old);
        }
        res
    }

    /// connects to a network from the list, `name` as the UI shows it
    fn connect(&mut self, name: &str, password: &str) -> Result<(), Failure> {
        let dev = self.wifi_device().ok_or(FAILED)?;
        let ssid = self.ssid_of(name);
        let saved = self.known().remove(&ssid);
        if password.is_empty()
            && let Some(saved) = &saved
        {
            let root = ObjectPath::from_static_str_unchecked("/");
            let active = self.manager.activate_connection(saved, &dev, &root).map_err(|e| {
                tracing::error!(?e, name, "cannot activate connection");
                FAILED
            })?;
            return self.wait(&active, &dev);
        }
        let (key_mgmt, hidden) = match self.aps.iter().find(|ap| ap.ssid == ssid) {
            Some(ap) => (ap.key_mgmt, false),
            // gone from the scan since it was picked, or a hidden network that only showed while
            // connected, so go by how it was saved
            None => saved.and_then(|path| saved_security(&self.settings_of(&path)?)).ok_or_else(
                || {
                    tracing::warn!(name, "network is gone");
                    FAILED
                },
            )?,
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
        KeyMgmt::Open | KeyMgmt::Enterprise | KeyMgmt::Wep => &[],
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
/// that doesn't announce its name
fn enterprise_settings<'a>(
    login: &'a EnterpriseLogin,
    ssid: &[u8],
    hidden: bool,
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
        settings
            .insert("802-11-wireless-security", HashMap::from([("key-mgmt", "wpa-eap".into())]));
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
        if !login.private_key_password.is_empty() {
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
const CERT_DIR: &str = "/etc/pki/taidan/8021x";

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

/// copies the files `login` uses to [`CERT_DIR`], readable only by root, and points it at them
fn install_certs(login: &mut EnterpriseLogin) -> Result<(), Failure> {
    let name = cert_name(login.wired, &login.ssid);
    let files = [
        ("ca", &mut login.ca_cert),
        ("client", &mut login.client_cert),
        ("key", &mut login.private_key),
        ("pac", &mut login.pac_file),
    ];
    for (kind, path) in files {
        if path.is_empty() || path.starts_with(CERT_DIR) {
            continue;
        }
        let ext = std::path::Path::new(path.as_str())
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_ascii_lowercase()))
            .unwrap_or_default();
        let dest = format!("{CERT_DIR}/{name}-{kind}{ext}");
        let res = crate::RT.block_on(libtaidan::root(
            "install",
            ["-D", "-m", "0600", path.as_str(), dest.as_str()],
        ));
        if let Err(e) = res {
            tracing::error!(?e, %path, dest, "cannot copy certificate");
            return Err("page-internet-8021x-error-file");
        }
        *path = dest.into();
    }
    Ok(())
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
fn set_proxy(nm: &Nm, ui: &slint::Weak<AppWindow>, url: &str, bypass: &str) {
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
        match res {
            Ok(proxy) => {
                let (url, bypass) = proxy.map(|p| (p.url, p.no_proxy)).unwrap_or_default();
                net.set_proxy_url(url.into());
                if !bypass.is_empty() {
                    net.set_proxy_bypass(bypass.replace(',', ", ").into());
                }
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
        if portal().is_some() {
            return true;
        }
        let uri = (self.manager.connectivity_check_uri().ok())
            .filter(|uri| !uri.is_empty())
            .unwrap_or_else(|| CHECK_URI.to_owned());
        let mut cmd = std::process::Command::new(PORTAL_BROWSER);
        // the `wl` platform needs `EGL_WL_bind_wayland_display`, which Mesa 25.2+ only has with
        // `legacy-wayland` and Fedora doesn't build that (https://github.com/Igalia/cog/issues/776),
        // `gtk4` is native Wayland too, and its window has a close button
        cmd.args(["--platform", "gtk4"]);
        // through the environment rather than `--proxy`, since anyone can read a command's
        // arguments and the URL may carry a password. WebKit then uses GLib's default proxy
        // resolver, and the libproxy one reads `http_proxy` and friends. GNOME's would go by its
        // own settings instead, so it's picked explicitly
        if let Some(proxy) =
            proxy.and_then(|(url, bypass)| libtaidan::proxy::Proxy::new(&url, &bypass))
        {
            cmd.envs(proxy.vars()).env("GIO_USE_PROXY_RESOLVER", "libproxy");
        }
        match cmd.arg(&uri).spawn() {
            Ok(child) => {
                tracing::info!(uri, "Opened the captive portal browser");
                *portal() = Some(child);
                self.portal_checked = None;
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
        if online && open {
            tracing::info!("Online, closing the captive portal browser");
            close_portal();
        }
        portal().is_some()
    }

    /// logs in to the wired network with 802.1X, replacing an earlier attempt, and doesn't keep
    /// the connection if it fails, so a wrong password isn't saved
    fn connect_wired_8021x(&mut self, login: &EnterpriseLogin) -> Result<(), Failure> {
        let dev = self.ethernet_device().ok_or(FAILED)?;
        let settings: zbus::Result<nm::Settings<'_>> =
            zbus::blocking::proxy::Builder::new(&self.conn).build();
        for path in settings.and_then(|s| s.list_connections()).unwrap_or_default() {
            let id = self
                .at::<nm::SettingsConnection<'_>>(&path)
                .and_then(|c| c.get_settings())
                .ok()
                .and_then(|mut s| s.get_mut("connection")?.remove("id"))
                .and_then(|id| String::try_from(id).ok());
            if id.as_deref() == Some(WIRED_8021X_ID) {
                self.delete(&path);
            }
        }
        let root = ObjectPath::from_static_str_unchecked("/");
        let (conn, active) = self
            .manager
            .add_and_activate_connection(enterprise_settings(login, &[], false), &dev, &root)
            .map_err(|e| {
                tracing::error!(?e, "cannot add wired 802.1X connection");
                FAILED
            })?;
        let res = self.wait(&active, &dev);
        if res.is_err() {
            self.delete(&conn);
        }
        res
    }

    fn ethernet_device(&self) -> Option<OwnedObjectPath> {
        self.devices_of(DEVICE_TYPE_ETHERNET).into_iter().next()
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
        self.active_on(dev).or_else(|| {
            let available = self.at::<nm::Device<'_>>(dev).ok()?.available_connections().ok()?;
            available.into_iter().next()
        })
    }

    /// the saved connection the dialog is for, a wi-fi network if `ssid` isn't empty
    fn ip_connection(&self, ssid: &str) -> Option<OwnedObjectPath> {
        if ssid.is_empty() {
            let dev = self.ethernet_device()?;
            self.ethernet_connection(&dev)
        } else {
            self.known().remove(&self.ssid_of(ssid))
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
            section.retain(|key, _| !IP_KEYS.contains(&key.as_str()));
            for (key, value) in values {
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
        let active = self.manager.activate_connection(conn, dev, &root).map_err(|e| {
            tracing::error!(?e, "cannot activate connection");
            FAILED_IP
        })?;
        self.wait(&active, dev)
    }

    fn save_ip(&mut self, ssid: &str, ip: IpConfig) -> Result<(), Failure> {
        let failed = |e: zbus::Error| {
            tracing::error!(?e, ssid, "cannot save IP settings");
            FAILED_IP
        };
        if !ssid.is_empty() {
            let bytes = self.ssid_of(ssid);
            let Some(conn) = self.known().remove(&bytes) else {
                // applied once connecting creates the connection
                self.pending_ip.insert(bytes, ip);
                return Ok(());
            };
            self.update_ip(&conn, &ip).map_err(failed)?;
            let dev = self.wifi_device().ok_or(FAILED_IP)?;
            if self.active_on(&dev).as_ref() == Some(&conn) {
                return self.activate(&conn, &dev);
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

/// asks the portal browser to quit, so it can shut down its web process too, and kills it if it
/// hasn't within a couple of seconds
fn close_portal() {
    let Some(mut child) = portal().take() else { return };
    let Ok(pid) = libc::pid_t::try_from(child.id()) else { return };
    // SAFETY: `pid` is our own child, which isn't reaped until `wait()` below, so the id can't
    // have been reused by another process
    if unsafe { libc::kill(pid, libc::SIGTERM) } == 0 {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if !matches!(child.try_wait(), Ok(None)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        tracing::warn!("captive portal browser didn't quit, killing it");
    }
    if let Err(e) = child.kill() {
        tracing::debug!(?e, "captive portal browser already gone");
    }
    if let Err(e) = child.wait() {
        tracing::debug!(?e, "cannot reap the captive portal browser");
    }
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
    let (ssid, res, enterprise) = match cmd {
        Cmd::Refresh => {
            push(ui, |net| net.set_scanning(true));
            nm.scan();
            // results come in over the next few seconds, `Nm::publish` says when it's done
            nm.scanning_until = Some(Instant::now() + SCAN_TIME);
            return;
        }
        Cmd::Connect { ssid, password } => {
            start_connecting(ui, &ssid);
            let res = nm.connect(&ssid, &password);
            let enterprise =
                nm.aps.iter().any(|ap| ap.name == ssid && ap.key_mgmt == KeyMgmt::Enterprise);
            (ssid, res, enterprise)
        }
        Cmd::ConnectHidden { ssid, key_mgmt, password } => {
            return connect_hidden(nm, ui, ssid, key_mgmt, &password);
        }
        Cmd::ConnectEnterprise(mut login) => {
            login.ssid = login.ssid.trim().into();
            if !login.wired {
                start_connecting(ui, &login.ssid);
            }
            let res = if let Err(failure) = install_certs(&mut login) {
                Err(failure)
            } else if login.wired {
                nm.connect_wired_8021x(&login)
            } else {
                let hidden = !nm.aps.iter().any(|ap| ap.name == login.ssid.as_str());
                let ssid = nm.ssid_of(&login.ssid);
                nm.add_and_connect(&ssid, enterprise_settings(&login, &ssid, hidden))
            };
            push(ui, move |net| {
                net.set_connecting_ssid("".into());
                net.set_enterprise_busy(false);
                match res {
                    Ok(()) => net.set_enterprise_open(false),
                    Err(failure) => net.set_enterprise_error_id(failure.into()),
                }
            });
            return;
        }
        Cmd::SetProxy { url, bypass } => return set_proxy(nm, ui, &url, &bypass),
        Cmd::OpenPortal(proxy) => {
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
                net.set_ip(ip);
                net.set_ip_loading(false);
            });
            return;
        }
        Cmd::SaveIp(ssid, ip) => {
            let res = IpConfig::parse(&ip).and_then(|ip| nm.save_ip(&ssid, ip));
            push(ui, move |net| {
                net.set_ip_busy(false);
                match res {
                    Ok(()) => net.set_ip_open(false),
                    Err(failure) => net.set_ip_error_id(failure.into()),
                }
            });
            return;
        }
    };
    push(ui, move |net| {
        net.set_connecting_ssid("".into());
        match res {
            Ok(()) => net.set_password_ssid("".into()),
            Err(failure) => {
                net.set_error_ssid(ssid.as_str().into());
                net.set_error_id(failure.into());
                // ask for the password if a saved one didn't work, or one was given and was wrong
                if failure == FAILED_SECRETS {
                    if enterprise {
                        net.set_enterprise_ssid(ssid.as_str().into());
                        net.set_enterprise_wired(false);
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

fn worker(ui: &slint::Weak<AppWindow>, rx: &mpsc::Receiver<Cmd>) {
    let mut nm = match Nm::new(ui.clone()) {
        Ok(nm) => nm,
        Err(e) => {
            tracing::error!(?e, "cannot talk to NetworkManager");
            push(ui, |net| net.set_wifi_available(false));
            return;
        }
    };
    nm.scan();
    loop {
        // sooner if a scan is due to end
        let timeout = (nm.scanning_until)
            .map_or(POLL, |until| until.saturating_duration_since(Instant::now()).min(POLL));
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

/// connecting to a network from the list or the password dialog
fn init_connect(
    ui: &slint::Weak<AppWindow>,
    net: &Network<'_>,
    send: impl Fn(Cmd) + Clone + 'static,
) {
    net.on_connect({
        let (ui, send) = (ui.clone(), send.clone());
        move |ssid, password| {
            if start_busy(&ui, &ssid) {
                send(Cmd::Connect { ssid: ssid.into(), password: password.into() });
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
        tracing::error!("network worker stopped");
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
            send(Cmd::OpenPortal(proxy));
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

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, net: &Network<'_>) {
    let tx = spawn_worker(ui);
    net.set_persist_offline_btn(crate::CFG.taidan0.persist_no_internet_btn);
    net.on_derive_hostname(|name, username| device_hostname(&name, &username).into());
    net.on_realm(|username| realm(&username).into());
    net.on_is_pkcs12(|path| is_pkcs12(&path));
    net.on_file_name(|path| {
        std::path::Path::new(path.as_str())
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().as_ref().into())
    });
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
            if let Some(ui) = ui.upgrade() {
                let net = ui.global::<Network<'_>>();
                if net.get_enterprise_busy() {
                    return;
                }
                net.set_enterprise_busy(true);
                net.set_enterprise_error_id("".into());
            }
            send(Cmd::ConnectEnterprise(login));
        }
    });
    net.on_set_proxy({
        let (send, ui) = (send.clone(), ui.clone());
        move |url, bypass| {
            if let Some(ui) = ui.upgrade() {
                let net = ui.global::<Network<'_>>();
                net.set_proxy_busy(true);
                net.set_proxy_error_id("".into());
            }
            send(Cmd::SetProxy { url: url.trim().into(), bypass: bypass.into() });
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
    init_portal(ui, net, send.clone());
    net.on_save({
        let ui = ui.clone();
        move |offline| {
            // done with the network page, so done logging in to the portal too
            send(Cmd::ClosePortal);
            let Some(ui) = ui.upgrade() else { return };
            let net = ui.global::<Network<'_>>();
            let mut settings = crate::SETTINGS.write();
            settings.nointernet = offline;
            let url = net.get_proxy_url();
            settings.proxy = (!url.is_empty())
                .then(|| libtaidan::proxy::Proxy::new(&url, &net.get_proxy_bypass()))
                .flatten();
            net.get_device_name().trim().clone_into(&mut settings.device_name);
            settings.hostname = net.get_hostname().into();
        }
    });
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
        let s = enterprise_settings(&login, b"eduroam", false);
        let x = &s["802-1x"];
        assert_eq!(x["eap"], Value::from(vec!["peap"]));
        assert_eq!(x["phase2-auth"], Value::from("mschapv2"));
        assert_eq!(x["anonymous-identity"], Value::from("anonymous@uni.example.edu"));
        assert_eq!(x["domain-suffix-match"], Value::from("uni.example.edu"));
        assert_eq!(x["system-ca-certs"], Value::from(true));
        assert!(!x.contains_key("phase1-peapver"));
        assert!(!s["802-11-wireless"].contains_key("hidden"));
        assert_eq!(
            enterprise_settings(&login, b"eduroam", true)["802-11-wireless"]["hidden"],
            Value::from(true)
        );
        let wired = EnterpriseLogin { wired: true, ..login };
        let s = enterprise_settings(&wired, &[], false);
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
        assert!(is_pkcs12("/c/ME.PFX"));
        assert!(!is_pkcs12("/c/me.pem"));
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
