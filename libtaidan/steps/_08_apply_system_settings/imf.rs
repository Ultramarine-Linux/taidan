use crate::{
    cfg::Desktop,
    home::{Home, HomeCmd},
    i18n,
    kconfig::KConfigEntry,
    prelude::*,
};

#[derive(Clone, Copy, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Imf {
    pub imf: Option<i18n::I18nImf>,
    pub start: Option<ImfStart>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImfStart {
    Kwinrc,
    XdgAutostart,
}

impl Imf {
    #[must_use]
    pub fn framework(self, cfg: &crate::cfg::Config) -> i18n::I18nImf {
        self.imf.or(cfg.legacy_imf()).unwrap_or(match cfg.desktop() {
            Desktop::Gnome => i18n::I18nImf::IBus,
            _ => i18n::I18nImf::Fcitx5,
        })
    }

    #[must_use]
    pub fn start(self, desktop: Desktop) -> ImfStart {
        self.start.unwrap_or(match desktop {
            Desktop::Plasma => ImfStart::Kwinrc,
            _ => ImfStart::XdgAutostart,
        })
    }

    #[must_use]
    pub fn configuration_warning(self, cfg: &crate::cfg::Config) -> Option<String> {
        (self.start == Some(ImfStart::Kwinrc) && cfg.desktop() != Desktop::Plasma).then(|| {
            format!(
                "The IMF configuration specifies start = \"kwinrc\", but gui_env is {}.\n\n\
                 The kwinrc startup method requires Plasma. Set gui_env = \"plasma\" for a \
                 Plasma session, or use start = \"xdg_autostart\" for another session.",
                cfg.gui_env
                    .as_deref()
                    .map_or_else(|| "unset".to_owned(), |env| format!("\"{env}\"")),
            )
        })
    }

    pub async fn apply_to_skel(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        skel: &Home,
    ) -> Res<bool> {
        match &with_installed_ims(settings, cfg, InstallPhase::Before).await {
            Some(installed) => apply(installed, skel, self.start(cfg.desktop())).await,
            None => Ok(false),
        }
    }

    pub async fn fix_homed(&self, settings: &crate::settings::Settings, cfg: &crate::cfg::Config) {
        if !cfg.systemd_homed() || settings.ims.is_empty() {
            return;
        }
        let in_skel = with_installed_ims(settings, cfg, InstallPhase::Before).await;
        let installed = with_installed_ims(settings, cfg, InstallPhase::After).await;
        if in_skel.as_ref().map(|s| &s.ims) == installed.as_ref().map(|s| &s.ims) {
            return;
        }
        tracing::info!(?installed, "some input methods didn't install, fixing the home");
        let settings = &installed
            .unwrap_or_else(|| crate::settings::Settings { ims: vec![], ..settings.clone() });
        let res = async {
            apply(settings, &Home::user(settings, cfg).await?, self.start(cfg.desktop())).await
        };
        if let Err(e) = res.await {
            tracing::warn!(?e, "cannot fix the input methods in the systemd-homed home");
        }
    }
}

impl crate::steps::InstallStep for Imf {
    fn stage(&self) -> crate::steps::Stage {
        crate::steps::Stage::ApplySystemSettings
    }

    async fn run(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _callback: &impl crate::Callback,
    ) -> Res<()> {
        run(*self, settings, cfg).await
    }
}

async fn run(imf: Imf, settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    if cfg.systemd_homed() {
        return Ok(());
    }
    let Some(installed) = &with_installed_ims(settings, cfg, InstallPhase::After).await else {
        let res = async {
            let home = Home::user(settings, cfg).await?;
            home.run(vec![remove_stale_fcitx5_config(&home)]).await
        };
        if let Err(e) = res.await {
            tracing::warn!(?e, "cannot remove the Fcitx5 config of an earlier attempt");
        }
        return Ok(());
    };
    let res = async {
        apply(installed, &Home::user(settings, cfg).await?, imf.start(cfg.desktop())).await
    };
    if let Err(e) = res.await {
        tracing::warn!(?e, "cannot setup IMFs");
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InstallPhase {
    Before,
    After,
}

async fn with_installed_ims(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    phase: InstallPhase,
) -> Option<crate::settings::Settings> {
    if phase == InstallPhase::Before && !settings.nointernet && crate::steps::can_use_dnf(cfg).await
    {
        return (!settings.ims.is_empty()).then(|| settings.clone());
    }
    let mut ims = vec![];
    for name in &settings.ims {
        let Some(pkg) = i18n::find_im(name).and_then(|im| im.pkg(settings.imf)) else { continue };
        match tokio::process::Command::new("rpm")
            .args(["-q", "--quiet", pkg])
            .status()
            .await
            .inspect_err(|e| crate::command::report_spawn_error("rpm", e))
        {
            Ok(status) if status.success() => ims.push(name.clone()),
            Ok(_) => tracing::info!(name, pkg, "input method isn't installed, not setting it up"),
            Err(e) => tracing::warn!(?e, "cannot run rpm"),
        }
    }
    (!ims.is_empty()).then(|| crate::settings::Settings { ims, ..settings.clone() })
}

const FCITX5_LAUNCHER: &str = "/usr/share/applications/fcitx5-wayland-launcher.desktop";

const FCITX5_PROFILE: &str = ".config/fcitx5/profile";
const FCITX5_AUTOSTART: &str = ".config/autostart/org.fcitx.Fcitx5.desktop";
const FCITX5_ENVIRONMENT: &str = ".config/environment.d/90-taidan-fcitx5.conf";

// https://invent.kde.org/plasma/kwin/-/blob/master/src/kcms/virtualkeyboard/virtualkeyboardsettings.kcfg
fn kwinrc_select_fcitx5() -> [KConfigEntry; 2] {
    [
        KConfigEntry::value("kwinrc", "Wayland", "InputMethod", Some(FCITX5_LAUNCHER)),
        KConfigEntry::bool("kwinrc", "Wayland", "VirtualKeyboardEnabled", true),
    ]
}

const FCITX5_PROFILE_MARKER: &str = "# written by Taidan";

fn fcitx5_environment() -> String {
    format!(
        "{FCITX5_PROFILE_MARKER}\n\
         INPUT_METHOD=fcitx\n\
         GTK_IM_MODULE=fcitx\n\
         QT_IM_MODULE=fcitx\n\
         XMODIFIERS=@im=fcitx\n"
    )
}

fn fcitx5_autostart() -> String {
    format!(
        "{FCITX5_PROFILE_MARKER}\n\
         [Desktop Entry]\n\
         Type=Application\n\
         Name=Fcitx 5\n\
         Exec=fcitx5 -d\n\
         TryExec=fcitx5\n\
         Terminal=false\n"
    )
}

fn remove_stale_fcitx5_config(home: &Home) -> HomeCmd {
    const SCRIPT: &str = r#"
        profile="$1/fcitx5/profile"
        if [ "$(head -n 1 -- "$profile" 2>/dev/null)" = "$2" ]; then
            if [ -e "$1/kwinrc" ] && command -v kreadconfig6 >/dev/null 2>&1 \
                && [ "$(kreadconfig6 --file "$1/kwinrc" --group Wayland --key InputMethod)" = "$3" ]; then
                kwriteconfig6 --file "$1/kwinrc" --group Wayland --key InputMethod --delete || exit
                kwriteconfig6 --file "$1/kwinrc" --group Wayland --key VirtualKeyboardEnabled --delete || exit
            fi
            rm -f -- "$profile" || exit
        fi
        autostart="$1/autostart/org.fcitx.Fcitx5.desktop"
        if [ "$(head -n 1 -- "$autostart" 2>/dev/null)" = "$2" ]; then
            rm -f -- "$autostart" || exit
        fi
        environment="$1/environment.d/90-taidan-fcitx5.conf"
        if [ "$(head -n 1 -- "$environment" 2>/dev/null)" = "$2" ]; then
            rm -f -- "$environment" || exit
        fi
        exit 0
    "#;
    let config = home.config();
    let config = config.to_string_lossy();
    HomeCmd::new(["sh", "-c", SCRIPT, "sh", &config, FCITX5_PROFILE_MARKER, FCITX5_LAUNCHER])
}

fn fcitx5_profile(
    crate::settings::Settings { ims, kb_layout, kb_variant, .. }: &crate::settings::Settings,
) -> String {
    let default_group_name = "Default"; // FIXME: this should be configurable? (was t!("default"))
    let var = kb_variant.as_ref().map(|variant| format!("-{variant}")).unwrap_or_default();
    let mut buf = format!(
        "\
            {FCITX5_PROFILE_MARKER}\n\
            [Groups/0]\n\
            # Group Name\n\
            Name={default_group_name}\n\
            # Layout\n\
            Default Layout={kb_layout}{var}\n\
            # Default Input Method\n\
            DefaultIM=keyboard-{kb_layout}{var}\n\n\
            [Groups/0/Items/0]\n\
            # Name\n\
            Name=keyboard-{kb_layout}{var}\n\
            # Layout\n\
            Layout=\n\n\
        ",
    );

    let latin_fallback = i18n::XkbKeymap::new(kb_layout, kb_variant.as_deref())
        .has_latin_fallback()
        .then_some("keyboard-us");
    // after the input methods, since fcitx5's Ctrl+Space toggles between the layout and the first
    let refs = ims.iter().filter_map(|im| i18n::find_im(im)?.fcitx5_ref).chain(latin_fallback);
    for (i, im) in refs.enumerate() {
        buf.push_str(&format!(
            "\
                [Groups/0/Items/{}]\n\
                # Name\n\
                Name={im}\n\
                # Layout\n\
                Layout=\n\n\
            ",
            i + 1,
        ));
    }

    buf.push_str(&format!("[GroupOrder]\n0={default_group_name}\n"));
    buf
}

fn im_cmds(settings: &crate::settings::Settings, home: &Home, start: ImfStart) -> Vec<HomeCmd> {
    if settings.imf == i18n::I18nImf::IBus && !settings.ims.is_empty() && !has_ibus_engine(settings)
    {
        return vec![];
    }
    let mut cmds = vec![remove_stale_fcitx5_config(home)];
    match settings.imf {
        i18n::I18nImf::IBus => {
            cmds.extend(home.session(ibus_dconf_writes(settings)).map(HomeCmd::best_effort));
        }
        i18n::I18nImf::Fcitx5 if settings.ims.is_empty() => {}
        i18n::I18nImf::Fcitx5 => {
            cmds.push(home.write_file(FCITX5_PROFILE, fcitx5_profile(settings)));
            cmds.push(home.write_file(FCITX5_ENVIRONMENT, fcitx5_environment()));
            match start {
                ImfStart::Kwinrc => cmds.extend(home.kconfig(&kwinrc_select_fcitx5())),
                ImfStart::XdgAutostart => {
                    cmds.push(home.write_file(FCITX5_AUTOSTART, fcitx5_autostart()));
                }
            }
        }
    }
    cmds
}

async fn apply(settings: &crate::settings::Settings, home: &Home, start: ImfStart) -> Res<bool> {
    let cmds = im_cmds(settings, home, start);
    let wrote = !cmds.is_empty();
    home.run(cmds).await?;
    Ok(wrote)
}

pub(in crate::steps) async fn apply_to_skel(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    skel: &Home,
) -> Res<bool> {
    cfg.imf_config().apply_to_skel(settings, cfg, skel).await
}

const EMPTY_STRING_ARRAY_GVARIANT: &str = "@as []";

fn has_ibus_engine(settings: &crate::settings::Settings) -> bool {
    settings.ims.iter().any(|s| i18n::find_im(s).is_some_and(|im| im.ibus_ref.is_some()))
}

fn ibus_dconf_writes(
    crate::settings::Settings { ims, kb_layout, kb_variant, .. }: &crate::settings::Settings,
) -> Vec<Vec<String>> {
    let engines: Vec<_> = ims.iter().filter_map(|s| i18n::find_im(s)?.ibus_ref).collect();
    let order = if engines.is_empty() {
        EMPTY_STRING_ARRAY_GVARIANT.to_owned()
    } else {
        format!("[{}]", engines.iter().map(|r| format!("'{r}'")).collect::<Vec<_>>().join(", "))
    };

    let layout = if kb_layout.is_empty() { "us" } else { kb_layout };
    let sources = i18n::XkbKeymap::new(layout, kb_variant.as_deref())
        .gnome_sources()
        .into_iter()
        .chain(engines.iter().map(|r| format!("('ibus', '{r}')")))
        .collect::<Vec<_>>()
        .join(", ");

    let write = |key: &str, value: String| {
        ["dbus-run-session", "--", "dconf", "write", key]
            .iter()
            .map(|&s| s.to_owned())
            .chain([value])
            .collect()
    };
    vec![
        write("/desktop/ibus/general/use-system-keyboard-layout", "true".to_owned()),
        write("/desktop/ibus/general/engines-order", order.clone()),
        write("/desktop/ibus/general/preload-engines", order),
        // GNOME and Budgie ignore `preload-engines` and drive IBus from `input-sources`
        write("/org/gnome/desktop/input-sources/sources", format!("[{sources}]")),
    ]
}

pub async fn fix_homed(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) {
    cfg.imf_config().fix_homed(settings, cfg).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_lists_ims() {
        let settings = crate::settings::Settings {
            kb_layout: "us".into(),
            ims: vec!["Pinyin".into(), "bogus".into(), "Mozc".into()],
            ..Default::default()
        };
        let profile = fcitx5_profile(&settings);
        assert!(profile.starts_with(&format!("{FCITX5_PROFILE_MARKER}\n[Groups/0]\n")));
        assert!(profile.contains("Default Layout=us\n"));
        assert!(profile.contains("[Groups/0/Items/1]\n# Name\nName=pinyin\n"));
        assert!(profile.contains("[Groups/0/Items/2]\n# Name\nName=mozc\n"));
        assert!(!profile.contains("Items/3"));
    }

    #[test]
    fn retry_replaces_input_method_choices_even_offline() {
        let first = crate::settings::Settings {
            kb_layout: "us".into(),
            ims: vec!["Pinyin".into()],
            nointernet: true,
            ..Default::default()
        };
        let retry = crate::settings::Settings { ims: vec!["Mozc".into()], ..first.clone() };
        let first_profile = fcitx5_profile(&first);
        let retry_profile = fcitx5_profile(&retry);
        assert!(first_profile.contains("Name=pinyin\n"));
        assert!(retry_profile.contains("Name=mozc\n"));
        assert!(!retry_profile.contains("Name=pinyin\n"));
    }

    #[test]
    fn ibus_engines_go_to_gnome_input_sources() {
        let settings = crate::settings::Settings {
            kb_layout: "de".into(),
            kb_variant: Some("nodeadkeys".into()),
            ims: vec!["Pinyin".into(), "bogus".into(), "Mozc".into()],
            ..Default::default()
        };
        let script = ibus_dconf_writes(&settings).concat().join(" ");
        let refs: Vec<_> = ["Pinyin", "Mozc"]
            .iter()
            .map(|im| i18n::find_im(im).unwrap().ibus_ref.unwrap())
            .collect();
        let sources = format!(
            "/org/gnome/desktop/input-sources/sources [('xkb', 'de+nodeadkeys'), ('ibus', '{}'), ('ibus', '{}')]",
            refs[0], refs[1],
        );
        assert!(script.contains(&sources), "{script}");
        let preload = format!("preload-engines ['{}', '{}']", refs[0], refs[1]);
        assert!(script.contains(&preload), "{script}");
    }

    #[test]
    fn non_latin_layouts_get_us_too() {
        let settings = crate::settings::Settings {
            kb_layout: "ru".into(),
            ims: vec!["Mozc".into()],
            ..Default::default()
        };
        let profile = fcitx5_profile(&settings);
        assert!(profile.contains("[Groups/0/Items/1]\n# Name\nName=mozc\n"));
        assert!(profile.contains("[Groups/0/Items/2]\n# Name\nName=keyboard-us\n"));
        let script = ibus_dconf_writes(&settings).concat().join(" ");
        assert!(script.contains("[('xkb', 'ru'), ('xkb', 'us'), ('ibus', "), "{script}");
    }

    #[test]
    fn no_ibus_engines_writes_nothing() {
        let skel = Home::Skel("/skel".into());
        let settings = crate::settings::Settings {
            ims: vec!["bogus".into()],
            imf: i18n::I18nImf::IBus,
            ..Default::default()
        };
        assert!(im_cmds(&settings, &skel, ImfStart::XdgAutostart).is_empty());
        let none = crate::settings::Settings { ims: vec![], ..settings };
        assert_eq!(im_cmds(&none, &skel, ImfStart::XdgAutostart).len(), 2);
    }

    #[test]
    fn fcitx5_on_plasma_also_selects_it_in_kwin() {
        let skel = Home::Skel("/skel".into());
        let settings = crate::settings::Settings {
            kb_layout: "us".into(),
            ims: vec!["Mozc".into()],
            ..Default::default()
        };
        let kwin_cmds = im_cmds(&settings, &skel, ImfStart::Kwinrc);
        assert_eq!(kwin_cmds.len(), 5);
        assert_eq!(kwin_cmds[3..], skel.kconfig(&kwinrc_select_fcitx5()).collect::<Vec<_>>());
        assert_eq!(im_cmds(&settings, &skel, ImfStart::XdgAutostart).len(), 4);
        let none = crate::settings::Settings { ims: vec![], ..settings };
        assert_eq!(im_cmds(&none, &skel, ImfStart::Kwinrc), [remove_stale_fcitx5_config(&skel)]);
    }

    #[tokio::test]
    async fn niri_and_noctalia_write_the_same_profile_and_autostart() {
        for gui_env in ["niri", "noctalia", "unknown"] {
            let cfg = crate::cfg::Config { gui_env: Some(gui_env.into()), ..Default::default() };
            let dir = tempfile::tempdir().unwrap();
            let skel = Home::Skel(dir.path().to_path_buf());
            let settings = crate::settings::Settings {
                kb_layout: "us".into(),
                ims: vec!["Mozc".into()],
                ..Default::default()
            };
            assert!(apply(&settings, &skel, cfg.imf_config().start(cfg.desktop())).await.unwrap());
            assert_eq!(
                std::fs::read_to_string(dir.path().join(FCITX5_PROFILE)).unwrap(),
                fcitx5_profile(&settings)
            );
            let autostart = std::fs::read_to_string(dir.path().join(FCITX5_AUTOSTART)).unwrap();
            assert!(autostart.contains("[Desktop Entry]\nType=Application\n"));
            assert!(autostart.contains("Exec=fcitx5 -d\n"));
            assert!(!dir.path().join(".config/kwinrc").exists());
            assert_eq!(
                std::fs::read_to_string(dir.path().join(FCITX5_ENVIRONMENT)).unwrap(),
                fcitx5_environment()
            );

            let none = crate::settings::Settings { ims: vec![], ..settings };
            apply(&none, &skel, cfg.imf_config().start(cfg.desktop())).await.unwrap();
            assert!(!dir.path().join(FCITX5_PROFILE).exists());
            assert!(!dir.path().join(FCITX5_AUTOSTART).exists());
            assert!(!dir.path().join(FCITX5_ENVIRONMENT).exists());
        }
    }

    #[tokio::test]
    async fn cleanup_preserves_unmarked_files_and_removes_orphaned_autostart() {
        let dir = tempfile::tempdir().unwrap();
        let skel = Home::Skel(dir.path().to_path_buf());
        skel.run(vec![
            skel.write_file(FCITX5_PROFILE, "user profile\n"),
            skel.write_file(FCITX5_AUTOSTART, "[Desktop Entry]\nExec=user-command\n"),
            skel.write_file(FCITX5_ENVIRONMENT, "QT_IM_MODULE=user-choice\n"),
        ])
        .await
        .unwrap();
        skel.run(vec![remove_stale_fcitx5_config(&skel)]).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join(FCITX5_PROFILE)).unwrap(),
            "user profile\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join(FCITX5_AUTOSTART)).unwrap(),
            "[Desktop Entry]\nExec=user-command\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join(FCITX5_ENVIRONMENT)).unwrap(),
            "QT_IM_MODULE=user-choice\n"
        );
        skel.run(vec![
            skel.remove_file(FCITX5_PROFILE),
            skel.write_file(FCITX5_AUTOSTART, fcitx5_autostart()),
            skel.write_file(FCITX5_ENVIRONMENT, fcitx5_environment()),
        ])
        .await
        .unwrap();
        skel.run(vec![remove_stale_fcitx5_config(&skel)]).await.unwrap();
        assert!(!dir.path().join(FCITX5_AUTOSTART).exists());
        assert!(!dir.path().join(FCITX5_ENVIRONMENT).exists());
    }

    #[tokio::test]
    async fn skeleton_uses_the_configured_start_method() {
        let cfg: crate::cfg::Config = basic_toml::from_str(
            r#"
                gui_env = "plasma"
                [[steps]]
                stage = "apply_system_settings"
                [[steps.apply_system_settings]]
                [steps.apply_system_settings.imf]
                start = "xdg_autostart"
            "#,
        )
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let skel = Home::Skel(dir.path().to_path_buf());
        let settings = crate::settings::Settings { ims: vec!["Mozc".into()], ..Default::default() };
        assert!(apply_to_skel(&settings, &cfg, &skel).await.unwrap());
        assert!(dir.path().join(FCITX5_AUTOSTART).exists());
        assert!(dir.path().join(FCITX5_ENVIRONMENT).exists());
        assert!(!dir.path().join(".config/kwinrc").exists());
    }

    #[tokio::test]
    async fn imf_object_owns_its_startup_options() {
        let cfg = crate::cfg::Config { gui_env: Some("plasma".into()), ..Default::default() };
        assert_eq!(cfg.imf_config().start(cfg.desktop()), ImfStart::Kwinrc);
        let imf = Imf { start: Some(ImfStart::XdgAutostart), ..Default::default() };
        let dir = tempfile::tempdir().unwrap();
        let skel = Home::Skel(dir.path().to_path_buf());
        let settings = crate::settings::Settings { ims: vec!["Mozc".into()], ..Default::default() };
        assert!(imf.apply_to_skel(&settings, &cfg, &skel).await.unwrap());
        assert!(dir.path().join(FCITX5_AUTOSTART).exists());
        assert!(!dir.path().join(".config/kwinrc").exists());
    }

    #[test]
    fn start_does_not_change_ibus_setup() {
        let skel = Home::Skel("/skel".into());
        let settings = crate::settings::Settings {
            ims: vec!["Pinyin".into()],
            imf: i18n::I18nImf::IBus,
            ..Default::default()
        };
        let cmds = im_cmds(&settings, &skel, ImfStart::Kwinrc);
        assert_eq!(cmds, im_cmds(&settings, &skel, ImfStart::XdgAutostart));
        assert_eq!(
            cmds,
            [
                remove_stale_fcitx5_config(&skel),
                skel.session(ibus_dconf_writes(&settings)).unwrap().best_effort(),
            ]
        );
    }

    #[test]
    fn kwinrc_warns_only_for_explicit_non_plasma_configuration() {
        let imf = Imf { start: Some(ImfStart::Kwinrc), ..Default::default() };
        for gui_env in [None, Some("niri"), Some("atomic-gnome")] {
            let cfg =
                crate::cfg::Config { gui_env: gui_env.map(str::to_owned), ..Default::default() };
            assert!(imf.configuration_warning(&cfg).unwrap().contains("xdg_autostart"));
            assert!(Imf::default().configuration_warning(&cfg).is_none());
            let cfg = crate::cfg::Config {
                steps: vec![
                    crate::steps::ApplySystemSettings { apply_system_settings: vec![imf.into()] }
                        .into(),
                ],
                ..cfg
            };
            assert_eq!(cfg.warnings().len(), 1);
        }
        for gui_env in ["plasma", "atomic-plasma", "PLASMA"] {
            let cfg = crate::cfg::Config { gui_env: Some(gui_env.into()), ..Default::default() };
            assert!(imf.configuration_warning(&cfg).is_none());
        }
    }

    #[test]
    fn environment_drop_in_uses_environment_d_not_shell_syntax() {
        let environment = fcitx5_environment();
        assert_eq!(
            environment.lines().skip(1).collect::<Vec<_>>(),
            [
                "INPUT_METHOD=fcitx",
                "GTK_IM_MODULE=fcitx",
                "QT_IM_MODULE=fcitx",
                "XMODIFIERS=@im=fcitx",
            ]
        );
        assert!(!environment.contains("export "));
    }
}
