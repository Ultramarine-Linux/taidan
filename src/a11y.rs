//! the accessibility page: applies the options to the OOBE's own KWin session as they change, and
//! keeps them in the settings so they're carried over to the new user
use std::sync::mpsc;
use std::time::{Duration, Instant};

use libtaidan::a11y::A11y as Opts;
use slint::ComponentHandle;
use zbus::blocking::Connection;

use crate::ui::{A11y, AppWindow};

/// about [`libtaidan::a11y::ZOOM`]: zoom in multiplies by KWin's default zoom factor, 1.2
const ZOOM_STEPS: usize = 4;

/// the screen reader, here rather than in the worker so [`shutdown`] can stop it, with a number
/// for each start so a watcher only watches the one it was started for
static ORCA: std::sync::Mutex<Option<(u64, std::process::Child)>> = std::sync::Mutex::new(None);

/// counts Orca starts, see [`ORCA`]
static ORCA_STARTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// how long Orca gets to exit after SIGTERM before it's killed
const ORCA_STOP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// how often the watcher checks whether Orca exited by itself
const ORCA_WATCH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// set by [`shutdown`], so a change still queued doesn't start Orca again as taidan exits
static SHUT_DOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// the UI's language as a POSIX locale, which Orca is started in, see [`language_changed`]
static ORCA_LOCALE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// the worker, once [`initialize_ui`] started it
static WORKER: std::sync::OnceLock<mpsc::Sender<Msg>> = std::sync::OnceLock::new();

/// where the options are kept while the OOBE runs, so they survive taidan (and KWin) restarting.
/// In the unit's runtime directory, which is gone on the next boot
const SAVED: &str = "a11y.json";

/// how long the language has to stay the same before Orca restarts to speak it
const ORCA_RESTART_DELAY: Duration = Duration::from_millis(1500);

enum Msg {
    Opts(Opts),
    /// Orca is started again, if it runs, to speak the new language
    RestartOrca,
}

/// called when the UI's language changes, `locale` being its POSIX locale (`pt_BR.UTF-8`).
/// Doesn't block, a running Orca is restarted by the worker
pub fn language_changed(locale: &str) {
    let mut current = ORCA_LOCALE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if current.as_deref() == Some(locale) {
        return;
    }
    *current = Some(locale.to_owned());
    drop(current);
    // before the worker exists Orca can't be running yet
    if let Some(tx) = WORKER.get()
        && let Err(e) = tx.send(Msg::RestartOrca)
    {
        tracing::error!(?e, "accessibility worker is gone");
    }
}

fn saved_path() -> Option<std::path::PathBuf> {
    // set by systemd for `RuntimeDirectory=`, and passed on through KWin to the session
    std::env::var_os("RUNTIME_DIRECTORY")
        .filter(|v| !v.is_empty())
        .map(|dir| std::path::PathBuf::from(dir).join(SAVED))
}

/// the options saved by an earlier taidan in this boot, or the defaults
fn load() -> Opts {
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

/// saves the options for [`load`], replacing the file so a crash never leaves half of it
fn save(opts: &Opts) {
    let Some(path) = saved_path() else { return };
    let saved = serde_json::to_vec(opts).map_err(std::io::Error::other).and_then(|json| {
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &path)
    });
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

/// stops Orca, which would outlive taidan otherwise
pub fn shutdown() {
    SHUT_DOWN.store(true, std::sync::atomic::Ordering::SeqCst);
    stop_orca();
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, a11y: &A11y<'_>) {
    let (tx, rx) = mpsc::channel();
    // one thread, so changes are applied in order
    std::thread::spawn({
        let ui = ui.clone();
        move || worker(&rx, &ui)
    });
    // what was on before taidan restarted, if it did in this boot, otherwise all off. Either way
    // the worker writes all of it, resetting anything else an earlier run left in the config
    let opts = load();
    set(a11y, &opts);
    crate::SETTINGS.write().a11y = opts;
    if let Err(e) = tx.send(Msg::Opts(opts)) {
        tracing::error!(?e, "accessibility worker is gone");
    }
    if WORKER.set(tx.clone()).is_err() {
        tracing::warn!("accessibility worker already started");
    }
    a11y.on_changed({
        let ui = ui.clone();
        move || {
            let Some(ui) = ui.upgrade() else { return };
            let opts = read(&ui.global::<A11y<'_>>());
            crate::SETTINGS.write().a11y = opts;
            if let Err(e) = tx.send(Msg::Opts(opts)) {
                tracing::error!(?e, "accessibility worker is gone");
            }
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
    // the session's config may be left over from an earlier run, so the first one writes it all
    let mut applied: Option<Opts> = None;
    // when Orca restarts for a new language. Not right away, the arrow keys on the language page
    // switch the language with every press, and each restart would cut Orca off
    let mut restart_at: Option<Instant> = None;
    loop {
        let msg = match restart_at {
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
        let mut next = None;
        // only the latest options matter if several came in at once
        for msg in msg.into_iter().chain(rx.try_iter()) {
            match msg {
                Msg::Opts(opts) => next = Some(opts),
                Msg::RestartOrca => restart_at = Some(Instant::now() + ORCA_RESTART_DELAY),
            }
        }
        let restart_orca = restart_at.is_some_and(|at| at <= Instant::now());
        if restart_orca {
            restart_at = None;
        }
        let prev = applied.unwrap_or_default();
        let Some(next) = next.or(applied) else { continue };
        if applied != Some(next) {
            save(&next);
            write_config(applied.as_ref(), &next);
            if let Some(bus) = &bus {
                apply_effects(bus, applied.is_none(), &prev, &next);
            }
        }
        if next.screen_reader != prev.screen_reader || (restart_orca && next.screen_reader) {
            // starting it picks up the latest language anyway
            restart_at = None;
            stop_orca();
            if next.screen_reader {
                start_orca(ui);
            }
        }
        applied = Some(next);
    }
    stop_orca();
}

/// writes what KWin watches for (keyboard filters, animation speed, plugins), notifying it
fn write_config(prev: Option<&Opts>, next: &Opts) {
    let old = prev.map(Opts::session_entries).unwrap_or_default();
    let mut entries = next.session_entries();
    if prev.is_none() {
        // KWin saves its zoom here and zooms back in when it starts, e.g. after a crash
        entries.push(libtaidan::a11y::Entry {
            file: "kwinrc",
            group: "Effect-zoom",
            key: "InitialZoom",
            value: None,
            bool: false,
        });
    }
    for entry in entries {
        if old.contains(&entry) {
            continue;
        }
        // in the OOBE user's own config, the one this KWin session reads
        let mut args = entry.kwriteconfig_args(entry.file);
        args.push("--notify".to_owned());
        match std::process::Command::new("kwriteconfig6").args(&args).status() {
            Ok(status) if status.success() => {}
            Ok(status) => tracing::warn!(?status, ?entry, "kwriteconfig6 failed"),
            Err(e) => tracing::warn!(?e, ?entry, "cannot run kwriteconfig6"),
        }
    }
}

/// zoom, the color filter and dwell click are KWin effects, loaded and set over D-Bus
fn apply_effects(bus: &Connection, first: bool, prev: &Opts, next: &Opts) {
    if first || prev.color_filter != next.color_filter {
        let method = match (prev.color_filter, next.color_filter) {
            (_, 0) => "unloadEffect",
            // the mode was written with the config, a loaded effect has to read it again
            (1.., _) if !first => "reconfigureEffect",
            _ => "loadEffect",
        };
        effect(bus, method, "colorblindnesscorrection");
    }
    if first || prev.dwell_click != next.dwell_click {
        effect(bus, if next.dwell_click { "loadEffect" } else { "unloadEffect" }, "dwellclicker");
    }
    // on the first change too, a KWin that was zoomed in before taidan (re)started would stay
    // that way while the tile says zoom is off
    if first || prev.zoom != next.zoom {
        if next.zoom {
            for _ in 0..ZOOM_STEPS {
                shortcut(bus, "view_zoom_in");
            }
        } else {
            shortcut(bus, "view_actual_size");
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

/// runs one of KWin's global shortcuts, which KWin handles itself on Wayland
fn shortcut(bus: &Connection, name: &str) {
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

fn lock_orca() -> std::sync::MutexGuard<'static, Option<(u64, std::process::Child)>> {
    ORCA.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// stops Orca with SIGTERM, so it can put the keyboard back and leave AT-SPI, and kills it if it
/// doesn't exit in time
fn stop_orca() {
    let Some((_, mut child)) = lock_orca().take() else { return };
    if let Ok(pid) = libc::pid_t::try_from(child.id()) {
        // SAFETY: the child hasn't been waited for, so the pid is still Orca's and not reused
        if unsafe { libc::kill(pid, libc::SIGTERM) } != 0 {
            tracing::warn!(e = ?std::io::Error::last_os_error(), "cannot stop Orca");
        }
    }
    let deadline = std::time::Instant::now() + ORCA_STOP_TIMEOUT;
    while std::time::Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(e) => {
                tracing::warn!(?e, "cannot wait for Orca");
                break;
            }
        }
    }
    tracing::warn!("Orca didn't exit, killing it");
    if let Err(e) = child.kill() {
        tracing::warn!(?e, "cannot kill Orca");
    }
    if let Err(e) = child.wait() {
        tracing::warn!(?e, "cannot wait for Orca");
    }
}

fn start_orca(ui: &slint::Weak<AppWindow>) {
    let mut cmd = std::process::Command::new("orca");
    cmd.arg("--replace");
    // in the UI's language, Orca's own messages through gettext and its voice
    let locale = ORCA_LOCALE.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
    if let Some(locale) = locale {
        cmd.envs(locale_env(&locale)).env_remove("LC_ALL").env_remove("LC_MESSAGES");
    }
    // without it, Orca's help and links open apps that lead to a terminal, see `sandbox`
    if let Err(e) = sandbox::apply(&mut cmd) {
        tracing::error!(?e, "cannot set up Orca's sandbox, not starting it");
        screen_reader_off(ui);
        return;
    }
    let mut orca = lock_orca();
    // under the lock, so `shutdown` either sees this Orca or stops it from starting
    if SHUT_DOWN.load(std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    // Orca turns on AT-SPI's screen reader flag, which makes Slint expose the UI to it
    match cmd.spawn() {
        Ok(child) => {
            let n = ORCA_STARTS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            *orca = Some((n, child));
            let ui = ui.clone();
            std::thread::spawn(move || watch_orca(n, &ui));
        }
        Err(e) => {
            tracing::error!(?e, "cannot start Orca");
            drop(orca);
            screen_reader_off(ui);
        }
    }
}

/// `LANG` and `LANGUAGE` for a POSIX locale, `LANGUAGE` being gettext's list of languages, which
/// takes the name without the codeset
fn locale_env(locale: &str) -> [(&'static str, String); 2] {
    let language = locale.split_once('.').map_or(locale, |(name, _)| name);
    [("LANG", locale.to_owned()), ("LANGUAGE", language.to_owned())]
}

/// turns the tile off when Orca `n` exits by itself, e.g. when it crashes
fn watch_orca(n: u64, ui: &slint::Weak<AppWindow>) {
    loop {
        std::thread::sleep(ORCA_WATCH_INTERVAL);
        let mut orca = lock_orca();
        // stopped or replaced, by the worker or `shutdown`
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

/// shows the screen reader as off, going through `changed` so the settings and the worker agree
fn screen_reader_off(ui: &slint::Weak<AppWindow>) {
    let sent = ui.upgrade_in_event_loop(|ui| {
        let a11y = ui.global::<A11y<'_>>();
        if a11y.get_screen_reader() {
            a11y.set_screen_reader(false);
            a11y.invoke_changed();
        }
    });
    if let Err(e) = sent {
        tracing::warn!(?e, "cannot turn the screen reader tile off");
    }
}

/// Orca runs as taidan, which polkit lets run anything as root while the OOBE is on. Its help
/// (`help:orca`) and links go through GIO to whatever handles them: Yelp or `KHelpCenter`, then a
/// browser, a file manager, and a terminal. So Orca gets XDG directories where GIO finds no
/// `.desktop` files and no default apps, and `g_app_info_launch_default_for_uri` fails.
///
/// GIO falls back to the `OpenURI` portal only with `GIO_USE_PORTALS=1` or in Flatpak and Snap, and
/// GTK 3 only with `GTK_USE_PORTAL=1`, so those are unset. Orca itself never runs `xdg-open`,
/// `kioclient` or Python's `webbrowser`, which reads `BROWSER`
mod sandbox {
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::path::{Path, PathBuf};

    /// what Orca's help, links and "show in folder" would open, mapped to nothing
    const MIME_TYPES: &[&str] = &[
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

    const UNSET: &[&str] = &["BROWSER", "GIO_USE_PORTALS", "GTK_USE_PORTAL"];

    /// the XDG directories the sandbox is made from, see [`build`]
    pub(super) struct Real {
        pub config_home: Option<PathBuf>,
        pub data_home: Option<PathBuf>,
        pub data_dirs: Vec<PathBuf>,
    }

    impl Real {
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
                // relative ones are invalid by the spec
                data_dirs: std::env::split_paths(&data_dirs).filter(|p| p.is_absolute()).collect(),
            };
            let base = var("XDG_RUNTIME_DIR").or_else(|| home.map(|h| h.join(".cache")));
            (base, real)
        }
    }

    /// sets up the sandbox and points `cmd` at it
    pub(super) fn apply(cmd: &mut std::process::Command) -> io::Result<()> {
        let (base, real) = Real::from_env();
        let base = base.ok_or_else(|| io::Error::other("no XDG_RUNTIME_DIR or HOME"))?;
        for var in UNSET {
            cmd.env_remove(var);
        }
        cmd.envs(build(&base.join("taidan-orca"), &real)?);
        Ok(())
    }

    /// makes `root` from scratch, with symlinks to everything in the real XDG directories except
    /// `applications` and `mimeapps.list`, and returns the variables that point at it. Each data
    /// directory gets one of its own so they still stack in the same order
    pub(super) fn build(root: &Path, real: &Real) -> io::Result<Vec<(&'static str, OsString)>> {
        match std::fs::remove_dir_all(root) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        std::os::unix::fs::DirBuilderExt::mode(std::fs::DirBuilder::new().recursive(true), 0o700)
            .create(root)?;

        let config = root.join("config");
        if let Some(real) = &real.config_home {
            // dconf reads its database from here but writes it through its service, which uses
            // the real one, so the link has to be there before Orca saves any settings
            std::fs::create_dir_all(real.join("dconf"))?;
        }
        link_all(real.config_home.as_deref(), &config, |name| {
            name.to_str().is_some_and(|n| n == "mimeapps.list" || n.ends_with("-mimeapps.list"))
        })?;
        let mut list = "[Default Applications]\n".to_owned();
        for ty in MIME_TYPES {
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

    /// makes `to` with a symlink to each entry in `from` that isn't `skip`ped
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
            let real = Real {
                config_home: Some(config_home.clone()),
                data_home: Some(fake.join("missing")),
                data_dirs: vec![share, fake.join("missing")],
            };
            let root = tmp.path().join("sandbox");
            // twice, the second one replaces the first
            build(&root, &real).unwrap();
            let env = build(&root, &real).unwrap();

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
            let dirs = env.iter().find(|(k, _)| *k == "XDG_DATA_DIRS").unwrap();
            assert_eq!(
                std::env::split_paths(&dirs.1).collect::<Vec<_>>(),
                [root.join("share/0"), root.join("share/1")]
            );
        }

        /// against this machine's real apps, if it has GIO's tool
        #[test]
        fn gio_finds_no_handler() {
            if !Path::new("/usr/bin/gio").exists() {
                return;
            }
            let tmp = tempfile::tempdir().unwrap();
            let (_, mut real) = Real::from_env();
            // the real config would get a `dconf` directory
            real.config_home = None;
            let env = build(tmp.path(), &real).unwrap();
            for ty in MIME_TYPES {
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
        // missing fields are off, a broken file is all off
        assert_eq!(parse(br#"{"zoom":true}"#), Opts { zoom: true, ..Opts::default() });
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
