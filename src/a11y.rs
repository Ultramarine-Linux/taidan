use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{OnceLock, mpsc};
use std::time::{Duration, Instant};

use libtaidan::a11y::A11y as Opts;
use parking_lot::Mutex;
use slint::ComponentHandle;
use zbus::blocking::Connection;

use crate::child::ChildSlot;
use crate::ui::{A11y, AppWindow};

const KWIN_ZOOM_IN_PRESSES: usize = 4;
const ORCA_RESTART_DELAY: Duration = Duration::from_millis(1500);
const ORCA_EXIT_POLL: Duration = Duration::from_secs(1);

type OrcaStartNumber = u64;

enum Msg {
    Opts(Opts),
    RestartOrcaInNewLanguage,
}

struct Orca {
    process: ChildSlot<OrcaStartNumber>,
    starts: AtomicU64,
    shut_down: AtomicBool,
    posix_locale: Mutex<Option<String>>,
    worker: OnceLock<mpsc::Sender<Msg>>,
}

static ORCA: Orca = Orca {
    process: ChildSlot::new("orca"),
    starts: AtomicU64::new(0),
    shut_down: AtomicBool::new(false),
    posix_locale: Mutex::new(None),
    worker: OnceLock::new(),
};

impl Orca {
    fn send(&self, msg: Msg) {
        if let Some(tx) = self.worker.get()
            && let Err(e) = tx.send(msg)
        {
            tracing::error!(?e, "accessibility worker is gone");
        }
    }

    fn language_changed(&self, posix_locale: &str) {
        let mut current = self.posix_locale.lock();
        if current.as_deref() == Some(posix_locale) {
            return;
        }
        *current = Some(posix_locale.to_owned());
        drop(current);
        self.send(Msg::RestartOrcaInNewLanguage);
    }

    fn shutdown(&self) {
        self.shut_down.store(true, Ordering::SeqCst);
        self.process.stop();
    }

    fn start(&'static self, ui: &slint::Weak<AppWindow>) {
        let mut cmd = std::process::Command::new("orca");
        cmd.arg("--replace");
        let locale = self.posix_locale.lock().clone();
        if let Some(locale) = locale {
            cmd.envs(locale_env(&locale)).env_remove("LC_ALL").env_remove("LC_MESSAGES");
        }
        if let Err(e) = sandbox::apply(&mut cmd) {
            tracing::error!(?e, "cannot set up Orca's sandbox, not starting it");
            screen_reader_off(ui);
            return;
        }
        let mut orca = self.process.lock();
        if self.shut_down.load(Ordering::SeqCst) {
            return;
        }
        match cmd.spawn() {
            Ok(child) => {
                let n = self.starts.fetch_add(1, Ordering::SeqCst);
                *orca = Some((n, child));
                let ui = ui.clone();
                std::thread::spawn(move || self.turn_tile_off_when_exits(n, &ui));
            }
            Err(e) => {
                tracing::error!(?e, "cannot start Orca");
                drop(orca);
                screen_reader_off(ui);
            }
        }
    }

    fn turn_tile_off_when_exits(&self, n: OrcaStartNumber, ui: &slint::Weak<AppWindow>) {
        loop {
            std::thread::sleep(ORCA_EXIT_POLL);
            let mut orca = self.process.lock();
            let Some((started, child)) = orca.as_mut().filter(|(started, _)| *started == n) else {
                return;
            };
            match child.try_wait() {
                Ok(None) => continue,
                Ok(Some(status)) => tracing::warn!(?status, started, "Orca exited by itself"),
                Err(e) => tracing::warn!(?e, "cannot wait for Orca, taking it as gone"),
            }
            *orca = None;
            drop(orca);
            screen_reader_off(ui);
            return;
        }
    }
}

pub fn language_changed(posix_locale: &str) {
    ORCA.language_changed(posix_locale);
}

fn saved_path() -> Option<std::path::PathBuf> {
    std::env::var_os("RUNTIME_DIRECTORY")
        .filter(|v| !v.is_empty())
        .map(|dir| std::path::PathBuf::from(dir).join("a11y.json"))
}

fn load_saved_this_boot() -> Opts {
    let Some(path) = saved_path() else { return Opts::default() };
    match std::fs::read(&path) {
        Ok(json) => parse(&json),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Opts::default(),
        Err(e) => {
            tracing::warn!(?e, ?path, "cannot read the saved accessibility options");
            Opts::default()
        }
    }
}

fn parse(json: &[u8]) -> Opts {
    serde_json::from_slice(json)
        .inspect_err(|e| tracing::warn!(?e, "the saved accessibility options are broken"))
        .unwrap_or_default()
}

fn write_atomically(path: &std::path::Path, contents: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

fn save(opts: &Opts) {
    let Some(path) = saved_path() else { return };
    let saved = serde_json::to_vec(opts)
        .map_err(std::io::Error::other)
        .and_then(|json| write_atomically(&path, &json));
    if let Err(e) = saved {
        tracing::warn!(?e, ?path, "cannot save the accessibility options");
    }
}

fn set(a11y: &A11y<'_>, opts: &Opts) {
    a11y.set_screen_reader(opts.screen_reader);
    a11y.set_zoom(opts.zoom);
    a11y.set_sticky_keys(opts.sticky_keys);
    a11y.set_slow_keys(opts.slow_keys);
    a11y.set_bounce_keys(opts.bounce_keys);
    a11y.set_dwell_click(opts.dwell_click);
    a11y.set_larger_text(opts.larger_text);
    a11y.set_legible_font(opts.legible_font);
    a11y.set_reduce_motion(opts.reduce_motion);
    a11y.set_color_filter(i32::from(opts.color_filter));
}

pub fn shutdown() {
    ORCA.shutdown();
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, a11y: &A11y<'_>) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn({
        let ui = ui.clone();
        move || worker(&rx, &ui)
    });
    let opts = load_saved_this_boot();
    set(a11y, &opts);
    if ORCA.worker.set(tx).is_err() {
        tracing::warn!("accessibility worker already started");
    }
    crate::SETTINGS.write().a11y = opts;
    ORCA.send(Msg::Opts(opts));
    a11y.on_changed({
        let ui = ui.clone();
        move || {
            let Some(ui) = ui.upgrade() else { return };
            let opts = read(&ui.global::<A11y<'_>>());
            crate::SETTINGS.write().a11y = opts;
            ORCA.send(Msg::Opts(opts));
        }
    });
}

fn read(a11y: &A11y<'_>) -> Opts {
    Opts {
        screen_reader: a11y.get_screen_reader(),
        zoom: a11y.get_zoom(),
        sticky_keys: a11y.get_sticky_keys(),
        slow_keys: a11y.get_slow_keys(),
        bounce_keys: a11y.get_bounce_keys(),
        dwell_click: a11y.get_dwell_click(),
        larger_text: a11y.get_larger_text(),
        legible_font: a11y.get_legible_font(),
        reduce_motion: a11y.get_reduce_motion(),
        color_filter: u8::try_from(a11y.get_color_filter()).unwrap_or_default(),
    }
}

fn worker(rx: &mpsc::Receiver<Msg>, ui: &slint::Weak<AppWindow>) {
    let bus = Connection::session().inspect_err(|e| tracing::error!(?e, "no session bus")).ok();
    let mut applied: Option<Opts> = None;
    let mut orca_restart_at: Option<Instant> = None;
    loop {
        let msg = match orca_restart_at {
            Some(at) => match rx.recv_timeout(at.saturating_duration_since(Instant::now())) {
                Ok(msg) => Some(msg),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            },
            None => match rx.recv() {
                Ok(msg) => Some(msg),
                Err(_) => break,
            },
        };
        let mut latest_opts = None;
        for msg in msg.into_iter().chain(rx.try_iter()) {
            match msg {
                Msg::Opts(opts) => latest_opts = Some(opts),
                Msg::RestartOrcaInNewLanguage => {
                    orca_restart_at = Some(Instant::now() + ORCA_RESTART_DELAY);
                }
            }
        }
        let restart_orca = orca_restart_at.is_some_and(|at| at <= Instant::now());
        if restart_orca {
            orca_restart_at = None;
        }
        let prev = applied.unwrap_or_default();
        let Some(next) = latest_opts.or(applied) else { continue };
        if applied != Some(next) {
            save(&next);
            write_kwin_config(applied.as_ref(), &next);
            if let Some(bus) = &bus {
                apply_kwin_effects(bus, applied.is_none(), &prev, &next);
            }
        }
        if next.screen_reader != prev.screen_reader || (restart_orca && next.screen_reader) {
            orca_restart_at = None;
            ORCA.process.stop();
            if next.screen_reader {
                ORCA.start(ui);
            }
        }
        applied = Some(next);
    }
    ORCA.process.stop();
}

const fn forget_kwin_saved_zoom() -> libtaidan::kconfig::KConfigEntry {
    libtaidan::kconfig::KConfigEntry::delete("kwinrc", "Effect-zoom", "InitialZoom")
}

fn write_kwin_config(prev: Option<&Opts>, next: &Opts) {
    let old = prev.map_or_default(Opts::session_entries);
    let mut entries = next.session_entries();
    if prev.is_none() {
        entries.push(forget_kwin_saved_zoom());
    }
    for entry in entries {
        if old.contains(&entry) {
            continue;
        }
        let mut kwriteconfig = std::process::Command::new("kwriteconfig6");
        match kwriteconfig.arg("--notify").args(entry.args(entry.file)).status() {
            Ok(status) if status.success() => {}
            Ok(status) => tracing::warn!(?status, ?entry, "kwriteconfig6 failed"),
            Err(e) => tracing::warn!(?e, ?entry, "cannot run kwriteconfig6"),
        }
    }
}

fn apply_kwin_effects(bus: &Connection, first: bool, prev: &Opts, next: &Opts) {
    if first || prev.color_filter != next.color_filter {
        let method = match (prev.color_filter, next.color_filter) {
            (_, 0) => "unloadEffect",
            (1.., _) if !first => "reconfigureEffect",
            _ => "loadEffect",
        };
        effect(bus, method, "colorblindnesscorrection");
    }
    if first || prev.dwell_click != next.dwell_click {
        effect(bus, if next.dwell_click { "loadEffect" } else { "unloadEffect" }, "dwellclicker");
    }
    if first || prev.zoom != next.zoom {
        if next.zoom {
            for _ in 0..KWIN_ZOOM_IN_PRESSES {
                run_kwin_shortcut(bus, "view_zoom_in");
            }
        } else {
            run_kwin_shortcut(bus, "view_actual_size");
        }
    }
}

fn effect(bus: &Connection, method: &str, name: &str) {
    if let Err(e) = bus.call_method(
        Some("org.kde.KWin"),
        "/Effects",
        Some("org.kde.kwin.Effects"),
        method,
        &(name,),
    ) {
        tracing::warn!(?e, method, name, "cannot change KWin effect");
    }
}

fn run_kwin_shortcut(bus: &Connection, name: &str) {
    if let Err(e) = bus.call_method(
        Some("org.kde.kglobalaccel"),
        "/component/kwin",
        Some("org.kde.kglobalaccel.Component"),
        "invokeShortcut",
        &(name,),
    ) {
        tracing::warn!(?e, name, "cannot run KWin shortcut");
    }
}

fn locale_env(posix_locale: &str) -> [(&'static str, String); 2] {
    [
        ("LANG", posix_locale.to_owned()),
        ("LANGUAGE", crate::l10n::without_codeset(posix_locale).to_owned()),
    ]
}

fn screen_reader_off(ui: &slint::Weak<AppWindow>) {
    crate::ui::on_ui(ui, |ui| {
        let a11y = ui.global::<A11y<'_>>();
        if a11y.get_screen_reader() {
            a11y.set_screen_reader(false);
            a11y.invoke_changed();
        }
    });
}

// polkit lets taidan run anything as root, and Orca's help and links could open apps that reach a
// terminal
mod sandbox {
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::path::{Path, PathBuf};

    const MIME_TYPES_WITHOUT_HANDLER: &[&str] = &[
        "x-scheme-handler/help",
        "x-scheme-handler/ghelp",
        "x-scheme-handler/man",
        "x-scheme-handler/info",
        "x-scheme-handler/http",
        "x-scheme-handler/https",
        "x-scheme-handler/ftp",
        "x-scheme-handler/file",
        "x-scheme-handler/mailto",
        "x-scheme-handler/terminal",
        "inode/directory",
        "text/html",
        "application/xhtml+xml",
        "application/x-desktop",
    ];

    const BROWSER_AND_PORTAL_VARS: &[&str] = &["BROWSER", "GIO_USE_PORTALS", "GTK_USE_PORTAL"];

    struct RealXdgDirs {
        config_home: Option<PathBuf>,
        data_home: Option<PathBuf>,
        data_dirs: Vec<PathBuf>,
    }

    impl RealXdgDirs {
        fn from_env() -> (Option<PathBuf>, Self) {
            let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
            let home = var("HOME");
            let data_dirs = std::env::var_os("XDG_DATA_DIRS")
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
            let real = Self {
                config_home: var("XDG_CONFIG_HOME")
                    .or_else(|| home.as_ref().map(|h| h.join(".config"))),
                data_home: var("XDG_DATA_HOME")
                    .or_else(|| home.as_ref().map(|h| h.join(".local/share"))),
                data_dirs: std::env::split_paths(&data_dirs).filter(|p| p.is_absolute()).collect(),
            };
            let base = var("XDG_RUNTIME_DIR").or_else(|| home.map(|h| h.join(".cache")));
            (base, real)
        }
    }

    pub(super) fn apply(cmd: &mut std::process::Command) -> io::Result<()> {
        let (base, real) = RealXdgDirs::from_env();
        let base = base.ok_or_else(|| io::Error::other("no XDG_RUNTIME_DIR or HOME"))?;
        for var in BROWSER_AND_PORTAL_VARS {
            cmd.env_remove(var);
        }
        cmd.envs(build(&base.join("taidan-orca"), &real)?);
        Ok(())
    }

    fn build(root: &Path, real: &RealXdgDirs) -> io::Result<Vec<(&'static str, OsString)>> {
        if let Err(e) = std::fs::remove_dir_all(root)
            && e.kind() != io::ErrorKind::NotFound
        {
            return Err(e);
        }
        std::os::unix::fs::DirBuilderExt::mode(std::fs::DirBuilder::new().recursive(true), 0o700)
            .create(root)?;

        let config = root.join("config");
        if let Some(real) = &real.config_home {
            // dconf's service writes to the real config, so its directory has to be linked
            std::fs::create_dir_all(real.join("dconf"))?;
        }
        link_all(real.config_home.as_deref(), &config, |name| {
            name.to_str().is_some_and(|n| n == "mimeapps.list" || n.ends_with("-mimeapps.list"))
        })?;
        let mut list = "[Default Applications]\n".to_owned();
        for ty in MIME_TYPES_WITHOUT_HANDLER {
            list += ty;
            list += "=\n";
        }
        std::fs::write(config.join("mimeapps.list"), list)?;

        let data = root.join("data");
        link_all(real.data_home.as_deref(), &data, is_applications)?;

        let mut dirs = vec![];
        for (i, dir) in real.data_dirs.iter().enumerate() {
            let share = root.join("share").join(i.to_string());
            link_all(Some(dir), &share, is_applications)?;
            dirs.push(share);
        }
        let dirs = std::env::join_paths(dirs).map_err(io::Error::other)?;

        Ok(vec![
            ("XDG_CONFIG_HOME", config.into_os_string()),
            ("XDG_DATA_HOME", data.into_os_string()),
            ("XDG_DATA_DIRS", dirs),
        ])
    }

    fn is_applications(name: &OsStr) -> bool {
        name == "applications"
    }

    fn link_all(from: Option<&Path>, to: &Path, skip: impl Fn(&OsStr) -> bool) -> io::Result<()> {
        std::fs::create_dir_all(to)?;
        let Some(from) = from else { return Ok(()) };
        let entries = match std::fs::read_dir(from) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        for entry in entries {
            let name = entry?.file_name();
            if !skip(&name) {
                std::os::unix::fs::symlink(from.join(&name), to.join(&name))?;
            }
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn gio_mime(env: &[(&'static str, OsString)], ty: &str) -> String {
            let out = std::process::Command::new("gio")
                .args(["mime", ty])
                .env_clear()
                .envs(env.iter().cloned())
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).into_owned()
        }

        #[test]
        fn hides_apps_and_defaults() {
            let tmp = tempfile::tempdir().unwrap();
            let fake = tmp.path().join("fake");
            let config_home = fake.join("config");
            let share = fake.join("share");
            for dir in
                [config_home.join("gtk-3.0"), share.join("applications"), share.join("icons")]
            {
                std::fs::create_dir_all(dir).unwrap();
            }
            std::fs::write(config_home.join("mimeapps.list"), "").unwrap();
            std::fs::write(config_home.join("kde-mimeapps.list"), "").unwrap();
            let real = RealXdgDirs {
                config_home: Some(config_home.clone()),
                data_home: Some(fake.join("missing")),
                data_dirs: vec![share, fake.join("missing")],
            };
            let root = tmp.path().join("sandbox");
            build(&root, &real).unwrap();
            let env_after_rebuild = build(&root, &real).unwrap();

            let config = root.join("config");
            assert_eq!(
                std::fs::read_link(config.join("gtk-3.0")).unwrap(),
                config_home.join("gtk-3.0")
            );
            assert!(config.join("dconf").is_dir() && config_home.join("dconf").is_dir());
            assert!(!config.join("kde-mimeapps.list").exists());
            let list = std::fs::read_to_string(config.join("mimeapps.list")).unwrap();
            assert!(list.contains("\nx-scheme-handler/help=\n"));
            assert!(
                root.join("share/0/icons").exists() && !root.join("share/0/applications").exists()
            );
            assert!(root.join("share/1").is_dir() && root.join("data").is_dir());
            let dirs = env_after_rebuild.iter().find(|(k, _)| *k == "XDG_DATA_DIRS").unwrap();
            assert_eq!(
                std::env::split_paths(&dirs.1).collect::<Vec<_>>(),
                [root.join("share/0"), root.join("share/1")]
            );
        }

        #[test]
        fn gio_finds_no_handler_among_this_machines_apps() {
            if !Path::new("/usr/bin/gio").exists() {
                return;
            }
            let tmp = tempfile::tempdir().unwrap();
            let (_, real) = RealXdgDirs::from_env();
            let real_without_config_to_touch = RealXdgDirs { config_home: None, ..real };
            let env = build(tmp.path(), &real_without_config_to_touch).unwrap();
            for ty in MIME_TYPES_WITHOUT_HANDLER {
                let out = gio_mime(&env, ty);
                assert!(out.starts_with("No default applications"), "{ty}: {out}");
                assert!(!out.contains("Registered applications"), "{ty}: {out}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_saved_options() {
        let opts = Opts { screen_reader: true, color_filter: 3, ..Opts::default() };
        assert_eq!(parse(&serde_json::to_vec(&opts).unwrap()), opts);
    }

    #[test]
    fn missing_saved_fields_are_off() {
        assert_eq!(parse(br#"{"zoom":true}"#), Opts { zoom: true, ..Opts::default() });
    }

    #[test]
    fn broken_saved_file_is_all_off() {
        assert_eq!(parse(b"{\"zoom\":"), Opts::default());
        assert_eq!(parse(b""), Opts::default());
    }

    #[test]
    fn locale_envs() {
        assert_eq!(
            locale_env("pt_BR.UTF-8"),
            [("LANG", "pt_BR.UTF-8".to_owned()), ("LANGUAGE", "pt_BR".to_owned())]
        );
        assert_eq!(locale_env("eo"), [("LANG", "eo".to_owned()), ("LANGUAGE", "eo".to_owned())]);
    }
}
