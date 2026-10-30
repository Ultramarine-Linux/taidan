//! wi-fi and connectivity through the D-Bus API of `NetworkManager`
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use slint::ComponentHandle;
use zbus::blocking::Connection;
use zbus::proxy::{CacheProperties, Defaults};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, Value};

use crate::ui::{AppWindow, EapMethod, EnterpriseLogin, NetState, Network, Security, WifiNetwork};

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
    }

    #[zbus::proxy(
        interface = "org.freedesktop.NetworkManager.Settings.Connection",
        default_service = "org.freedesktop.NetworkManager",
        gen_async = false,
        blocking_name = "SettingsConnection"
    )]
    pub trait SettingsConnection {
        fn get_settings(&self) -> zbus::Result<HashMap<String, HashMap<String, OwnedValue>>>;
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
    }
}

// values from NetworkManager's D-Bus API docs
const DEVICE_TYPE_ETHERNET: u32 = 1;
const DEVICE_TYPE_WIFI: u32 = 2;
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
    wired: bool,
    networks: Vec<WifiNetwork>,
}

enum Cmd {
    Refresh,
    Connect { ssid: String, password: String },
    ConnectEnterprise(EnterpriseLogin),
}

/// why a connection failed, as a translation id
type Failure = &'static str;
const FAILED: Failure = "page-internet-error";
const FAILED_SECRETS: Failure = "page-internet-error-password";
const FAILED_TIMEOUT: Failure = "page-internet-error-timeout";

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
}

impl Nm {
    fn new() -> zbus::Result<Self> {
        let conn = Connection::system()?;
        let nm = zbus::blocking::proxy::Builder::new(&conn)
            .cache_properties(CacheProperties::No)
            .build()?;
        Ok(Self { conn, manager: nm, aps: vec![] })
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
        let wired = self.devices_of(DEVICE_TYPE_ETHERNET).iter().any(|path| {
            self.at::<nm::Device<'_>>(path)
                .and_then(|d| d.state())
                .is_ok_and(|s| s == DEVICE_STATE_ACTIVATED)
        });
        let Some(dev) = self.wifi_device() else {
            self.aps.clear();
            return Snapshot { state, wifi: false, wired, networks: vec![] };
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
        Snapshot { state, wifi: true, wired, networks }
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
    fn add_and_connect(&self, ssid: &str, settings: nm::ConnSettings<'_>) -> Result<(), Failure> {
        let dev = self.wifi_device().ok_or(FAILED)?;
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

/// the part of `username` after the `@`, which for eduroam is the user's institution
fn realm(username: &str) -> &str {
    username.rsplit_once('@').map_or("", |(_, realm)| realm.trim())
}

fn enterprise_settings(login: &EnterpriseLogin) -> nm::ConnSettings<'_> {
    let mut settings = base_settings(&login.ssid);
    settings.insert("802-11-wireless-security", HashMap::from([("key-mgmt", "wpa-eap".into())]));
    let (eap, phase2) = match login.method {
        EapMethod::Peap => ("peap", "mschapv2"),
        EapMethod::Ttls => ("ttls", "pap"),
    };
    let mut x = HashMap::from([
        ("eap", Value::from(vec![eap])),
        ("phase2-auth", phase2.into()),
        ("identity", login.username.as_str().into()),
        ("password", login.password.as_str().into()),
    ]);
    let realm = realm(&login.username);
    if !realm.is_empty() {
        // keeps the username private from anyone but the user's own institution
        x.insert("anonymous-identity", format!("anonymous@{realm}").into());
    }
    if !login.domain.is_empty() {
        // only send the password to a server with a valid certificate for this domain
        x.insert("domain-suffix-match", login.domain.as_str().into());
        x.insert("system-ca-certs", true.into());
    }
    settings.insert("802-1x", x);
    settings
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
        net.set_wired(snap.wired);
        net.set_eduroam_nearby(snap.networks.iter().any(|n| n.ssid == "eduroam"));
        net.set_connected_ssid(
            snap.networks.iter().find(|n| n.connected).map(|n| n.ssid.clone()).unwrap_or_default(),
        );
        net.set_networks(slint::ModelRc::new(slint::VecModel::from(snap.networks)));
    });
}

fn handle(nm: &Nm, ui: &slint::Weak<AppWindow>, cmd: Cmd) {
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
        Cmd::ConnectEnterprise(login) => {
            start_connecting(ui, &login.ssid);
            let res = nm.add_and_connect(&login.ssid, enterprise_settings(&login));
            (login.ssid.into(), res, true)
        }
    };
    push(ui, move |net| {
        net.set_connecting_ssid("".into());
        match res {
            Ok(()) => {
                net.set_password_ssid("".into());
                net.set_enterprise_ssid("".into());
            }
            Err(failure) => {
                net.set_error_ssid(ssid.as_str().into());
                net.set_error_id(failure.into());
                // ask for the password if a saved one didn't work, or one was given and was wrong
                if failure == FAILED_SECRETS {
                    if enterprise {
                        net.set_enterprise_ssid(ssid.as_str().into());
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
    loop {
        match rx.recv_timeout(POLL) {
            Ok(cmd) => handle(&nm, ui, cmd),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
        let snap = nm.snapshot();
        // replacing the model rebuilds every row, so only do it when something changed
        if last.as_ref() != Some(&snap) {
            last = Some(snap.clone());
            push_snapshot(ui, snap);
        }
    }
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, net: &Network<'_>) {
    let (tx, rx) = mpsc::channel();
    let worker_ui = ui.clone();
    std::thread::spawn(move || worker(&worker_ui, &rx));
    net.set_persist_offline_btn(crate::CFG.taidan0.persist_no_internet_btn);
    net.on_derive_hostname(|name| derive_hostname(&name).into());
    net.on_realm(|username| realm(&username).into());
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
    net.on_connect_enterprise(move |login| send(Cmd::ConnectEnterprise(login)));
    net.on_open_portal(|| {
        // a small browser for logging in to the network
        if let Err(e) = std::process::Command::new("io.elementary.capnet-assist").spawn() {
            tracing::error!(?e, "cannot open capnet-assist");
        }
    });
    net.on_save({
        let ui = ui.clone();
        move |offline| {
            let Some(ui) = ui.upgrade() else { return };
            let net = ui.global::<Network<'_>>();
            let mut settings = crate::SETTINGS.write();
            settings.nointernet = offline;
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

    #[test]
    fn enterprise_settings_verify_server() {
        let login = EnterpriseLogin {
            ssid: "eduroam".into(),
            username: "jane@uni.example.edu".into(),
            password: "pw".into(),
            method: EapMethod::Peap,
            domain: "uni.example.edu".into(),
        };
        let s = enterprise_settings(&login);
        let x = &s["802-1x"];
        assert_eq!(x["anonymous-identity"], Value::from("anonymous@uni.example.edu"));
        assert_eq!(x["domain-suffix-match"], Value::from("uni.example.edu"));
        assert_eq!(x["system-ca-certs"], Value::from(true));
    }
}
