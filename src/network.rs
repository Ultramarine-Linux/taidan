//! wi-fi and connectivity through the D-Bus API of `NetworkManager`
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use slint::ComponentHandle;
use zbus::blocking::Connection;
use zbus::proxy::{CacheProperties, Defaults};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

use crate::ui::{
    AppWindow, CaSource, EapMethod, EnterpriseLogin, IpSettings, NetState, Network, Security,
    WifiNetwork,
};

/// how often the state and network list are refreshed while nothing else happens
const POLL: Duration = Duration::from_secs(3);
/// how long a connection may take to come up
const CONNECT_TIMEOUT: Duration = Duration::from_secs(45);

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
    ssid: String,
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

struct Nm {
    conn: Connection,
    manager: nm::NetworkManager<'static>,
    /// access points from the last refresh, strongest per network
    aps: Vec<Ap>,
    /// IP settings for wi-fi networks without a saved connection yet, used once connecting
    pending_ip: HashMap<String, IpConfig>,
    /// the browser opened for logging in to a captive portal
    portal: Option<std::process::Child>,
}

impl Nm {
    fn new() -> zbus::Result<Self> {
        let conn = Connection::system()?;
        let nm = zbus::blocking::proxy::Builder::new(&conn)
            .cache_properties(CacheProperties::No)
            .build()?;
        Ok(Self { conn, manager: nm, aps: vec![], pending_ip: HashMap::new(), portal: None })
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

    fn access_points(&self, dev: &ObjectPath<'_>) -> (Vec<Ap>, Option<String>) {
        let Ok(wireless) = self.at::<nm::Wireless<'_>>(dev) else { return (vec![], None) };
        let read = |path: &ObjectPath<'_>| -> zbus::Result<Ap> {
            let ap = self.at::<nm::AccessPoint<'_>>(path)?;
            Ok(Ap {
                ssid: String::from_utf8_lossy(&ap.ssid()?).into_owned(),
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
        let portal = self.manager.connectivity().is_ok_and(|c| c == 2);
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
                ssid: ap.ssid.as_str().into(),
                strength: bars(ap.strength),
                security: ap.key_mgmt.security(),
                known: known.contains_key(ap.ssid.as_bytes()),
                connected: active.as_deref() == Some(ap.ssid.as_str()),
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

    /// waits for `active` to come up on `dev`
    fn wait(&self, active: &ObjectPath<'_>, dev: &ObjectPath<'_>) -> Result<(), Failure> {
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        let (active, dev) =
            (self.at::<nm::ActiveConnection<'_>>(active), self.at::<nm::Device<'_>>(dev));
        let mut reason = 0;
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

    /// connects with new `settings`, replacing any saved connection for `ssid`, and doesn't keep
    /// them if connecting fails, so a wrong password isn't saved
    fn add_and_connect(
        &self,
        ssid: &str,
        mut settings: nm::ConnSettings<'_>,
    ) -> Result<(), Failure> {
        let dev = self.wifi_device().ok_or(FAILED)?;
        if let Some(ip) = self.pending_ip.get(ssid) {
            for (name, values) in ip.sections() {
                settings.entry(name).or_default().extend(values);
            }
        }
        if let Some(old) = self.known().get(ssid.as_bytes()) {
            self.delete(old);
        }
        let root = ObjectPath::from_static_str_unchecked("/");
        let (conn, active) =
            self.manager.add_and_activate_connection(settings, &dev, &root).map_err(|e| {
                tracing::error!(?e, ssid, "cannot add connection");
                FAILED
            })?;
        let res = self.wait(&active, &dev);
        if res.is_err() {
            self.delete(&conn);
        }
        res
    }

    fn connect(&self, ssid: &str, password: &str) -> Result<(), Failure> {
        let dev = self.wifi_device().ok_or(FAILED)?;
        if password.is_empty()
            && let Some(saved) = self.known().get(ssid.as_bytes())
        {
            let root = ObjectPath::from_static_str_unchecked("/");
            let active = self.manager.activate_connection(saved, &dev, &root).map_err(|e| {
                tracing::error!(?e, ssid, "cannot activate connection");
                FAILED
            })?;
            return self.wait(&active, &dev);
        }
        let key_mgmt =
            self.aps.iter().find(|ap| ap.ssid == ssid).map_or(KeyMgmt::Psk, |ap| ap.key_mgmt);
        self.add_and_connect(ssid, wifi_settings(ssid, key_mgmt, password))
    }
}

fn wifi_settings<'a>(ssid: &'a str, key_mgmt: KeyMgmt, password: &'a str) -> nm::ConnSettings<'a> {
    let mut settings = base_settings(ssid);
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

fn base_settings(ssid: &str) -> nm::ConnSettings<'_> {
    HashMap::from([
        ("connection", HashMap::from([("id", ssid.into()), ("type", "802-11-wireless".into())])),
        (
            "802-11-wireless",
            HashMap::from([
                ("ssid", Value::from(ssid.as_bytes().to_vec())),
                ("mode", "infrastructure".into()),
            ]),
        ),
    ])
}

/// name of the wired connection taidan makes for 802.1X, so a retry replaces it
const WIRED_8021X_ID: &str = "Wired 802.1X";

/// the part of `username` after the `@`, which for eduroam is the user's institution
fn realm(username: &str) -> &str {
    username.rsplit_once('@').map_or("", |(_, realm)| realm.trim())
}

/// settings for an 802.1X login, `hidden` for a wi-fi network that doesn't announce its name
fn enterprise_settings(login: &EnterpriseLogin, hidden: bool) -> nm::ConnSettings<'_> {
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
        let mut settings = base_settings(&login.ssid);
        if hidden && let Some(wifi) = settings.get_mut("802-11-wireless") {
            wifi.insert("hidden", true.into());
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

/// copies the files `login` uses to [`CERT_DIR`], readable only by root, and points it at them
fn install_certs(login: &mut EnterpriseLogin) -> Result<(), Failure> {
    let name = derive_hostname(if login.wired { "wired" } else { login.ssid.as_str() });
    let name = if name.is_empty() { "network".to_owned() } else { name };
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
        if self.portal.is_some() {
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
        if let Some((url, bypass)) = proxy {
            cmd.args(["--proxy", &url]);
            for host in bypass.split(',').map(str::trim).filter(|h| !h.is_empty()) {
                cmd.args(["--ignore-host", host]);
            }
        }
        match cmd.arg(&uri).spawn() {
            Ok(child) => {
                tracing::info!(uri, "Opened the captive portal browser");
                self.portal = Some(child);
                true
            }
            Err(e) => {
                tracing::error!(?e, "cannot open {PORTAL_BROWSER} for the captive portal");
                false
            }
        }
    }

    fn close_portal(&mut self) {
        if let Some(mut child) = self.portal.take() {
            if let Err(e) = child.kill() {
                tracing::debug!(?e, "captive portal browser already gone");
            }
            if let Err(e) = child.wait() {
                tracing::debug!(?e, "cannot reap the captive portal browser");
            }
        }
    }

    /// whether the portal browser is still open, it's closed once the internet works
    fn check_portal(&mut self, state: NetState) -> bool {
        if let Some(child) = &mut self.portal
            && !matches!(child.try_wait(), Ok(None))
        {
            self.portal = None;
        }
        if self.portal.is_some() && state == NetState::Online {
            tracing::info!("Online, closing the captive portal browser");
            self.close_portal();
        }
        self.portal.is_some()
    }

    /// logs in to the wired network with 802.1X, replacing an earlier attempt, and doesn't keep
    /// the connection if it fails, so a wrong password isn't saved
    fn connect_wired_8021x(&self, login: &EnterpriseLogin) -> Result<(), Failure> {
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
            .add_and_activate_connection(enterprise_settings(login, false), &dev, &root)
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
            self.known().remove(ssid.as_bytes())
        }
    }

    fn load_ip(&self, ssid: &str) -> IpConfig {
        let settings = self.ip_connection(ssid).and_then(|path| {
            self.at::<nm::SettingsConnection<'_>>(&path).ok()?.get_settings().ok()
        });
        settings.map_or_else(
            || self.pending_ip.get(ssid).cloned().unwrap_or_default(),
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
    fn activate(&self, conn: &ObjectPath<'_>, dev: &ObjectPath<'_>) -> Result<(), Failure> {
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
            let Some(conn) = self.known().remove(ssid.as_bytes()) else {
                // applied once connecting creates the connection
                self.pending_ip.insert(ssid.to_owned(), ip);
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
        net.set_networks(slint::ModelRc::new(slint::VecModel::from(snap.networks)));
    });
}

fn handle(nm: &mut Nm, ui: &slint::Weak<AppWindow>, cmd: Cmd) {
    let (ssid, res, enterprise) = match cmd {
        Cmd::Refresh => {
            push(ui, |net| net.set_scanning(true));
            nm.scan();
            // results come in over the next few seconds
            std::thread::sleep(Duration::from_secs(3));
            push(ui, |net| net.set_scanning(false));
            return;
        }
        Cmd::Connect { ssid, password } => {
            start_connecting(ui, &ssid);
            let res = nm.connect(&ssid, &password);
            let enterprise =
                nm.aps.iter().any(|ap| ap.ssid == ssid && ap.key_mgmt == KeyMgmt::Enterprise);
            (ssid, res, enterprise)
        }
        Cmd::ConnectEnterprise(mut login) => {
            login.ssid = login.ssid.trim().into();
            push(ui, |net| {
                net.set_enterprise_busy(true);
                net.set_enterprise_error_id("".into());
            });
            if !login.wired {
                start_connecting(ui, &login.ssid);
            }
            let res = if let Err(failure) = install_certs(&mut login) {
                Err(failure)
            } else if login.wired {
                nm.connect_wired_8021x(&login)
            } else {
                let hidden = !nm.aps.iter().any(|ap| ap.ssid == login.ssid.as_str());
                nm.add_and_connect(&login.ssid, enterprise_settings(&login, hidden))
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
            nm.close_portal();
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

fn start_connecting(ui: &slint::Weak<AppWindow>, ssid: &str) {
    let ssid = slint::SharedString::from(ssid);
    push(ui, move |net| {
        net.set_error_id("".into());
        net.set_error_ssid("".into());
        net.set_connecting_ssid(ssid);
    });
}

fn worker(ui: &slint::Weak<AppWindow>, rx: &mpsc::Receiver<Cmd>) {
    let mut nm = match Nm::new() {
        Ok(nm) => nm,
        Err(e) => {
            tracing::error!(?e, "cannot talk to NetworkManager");
            push(ui, |net| net.set_wifi_available(false));
            return;
        }
    };
    nm.scan();
    let mut last = None;
    let mut portal_open = false;
    loop {
        match rx.recv_timeout(POLL) {
            Ok(cmd) => handle(&mut nm, ui, cmd),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                nm.close_portal();
                return;
            }
        }
        let snap = nm.snapshot();
        let open = nm.check_portal(snap.state);
        if open != portal_open {
            portal_open = open;
            push(ui, move |net| net.set_portal_open(open));
        }
        // replacing the model rebuilds every row, so only do it when something changed
        if last.as_ref() != Some(&snap) {
            last = Some(snap.clone());
            push_snapshot(ui, snap);
        }
    }
}

/// shows `dir` in the file browser, or the first place if it's empty
fn browse(net: &Network<'_>, dir: slint::SharedString, pac: bool, all: bool) {
    let places = browse_places();
    let dir = if dir.is_empty() {
        places.first().map_or_else(|| "/".into(), |p| p.path.clone())
    } else {
        dir
    };
    let path = std::path::Path::new(dir.as_str());
    let parent = path.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    net.set_browse_entries(slint::ModelRc::new(slint::VecModel::from(list_dir(path, pac, all))));
    net.set_browse_places(slint::ModelRc::new(slint::VecModel::from(places)));
    net.set_browse_parent(parent.into());
    net.set_browse_dir(dir);
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

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, net: &Network<'_>) {
    let (tx, rx) = mpsc::channel();
    let worker_ui = ui.clone();
    std::thread::spawn(move || worker(&worker_ui, &rx));
    net.set_persist_offline_btn(crate::CFG.taidan0.persist_no_internet_btn);
    net.on_derive_hostname(|name| derive_hostname(&name).into());
    net.on_realm(|username| realm(&username).into());
    net.on_is_pkcs12(|path| is_pkcs12(&path));
    net.on_file_name(|path| {
        std::path::Path::new(path.as_str())
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().as_ref().into())
    });
    net.on_browse({
        let ui = ui.clone();
        move |dir, pac, all| {
            if let Some(ui) = ui.upgrade() {
                browse(&ui.global::<Network<'_>>(), dir, pac, all);
            }
        }
    });
    let send = move |cmd| {
        if let Err(e) = tx.send(cmd) {
            tracing::error!(?e, "network worker is gone");
        }
    };
    net.on_refresh({
        let send = send.clone();
        move || send(Cmd::Refresh)
    });
    net.on_connect({
        let send = send.clone();
        move |ssid, password| send(Cmd::Connect { ssid: ssid.into(), password: password.into() })
    });
    net.on_connect_enterprise({
        let send = send.clone();
        move |login| send(Cmd::ConnectEnterprise(login))
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
        let s = enterprise_settings(&login, false);
        let x = &s["802-1x"];
        assert_eq!(x["eap"], Value::from(vec!["peap"]));
        assert_eq!(x["phase2-auth"], Value::from("mschapv2"));
        assert_eq!(x["anonymous-identity"], Value::from("anonymous@uni.example.edu"));
        assert_eq!(x["domain-suffix-match"], Value::from("uni.example.edu"));
        assert_eq!(x["system-ca-certs"], Value::from(true));
        assert!(!x.contains_key("phase1-peapver"));
        assert!(!s["802-11-wireless"].contains_key("hidden"));
        assert_eq!(
            enterprise_settings(&login, true)["802-11-wireless"]["hidden"],
            Value::from(true)
        );
        let wired = EnterpriseLogin { wired: true, ..login };
        let s = enterprise_settings(&wired, false);
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
