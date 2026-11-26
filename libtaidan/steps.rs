mod _00_useradd;
mod _04_systemsetup;
mod _05_dnfdownloadapps;
mod _06_dnfinstallapps;
mod _07_drivers_codecs;
mod _08_apply_system_settings;
mod _12_downloadupdates;

use crate::prelude::*;
use enum_dispatch::enum_dispatch;

pub use _00_useradd::UserAdd;
pub use _04_systemsetup::SystemSetup;
pub use _05_dnfdownloadapps::DnfDownloadApps;
pub use _06_dnfinstallapps::DnfInstallApps;
pub use _07_drivers_codecs::DriversCodecs;
pub use _08_apply_system_settings::{
    ApplySystemSettings, Grub, Hostname, Imf, ImfStart, Locale, SystemSetting, Theme, Timezone,
};
pub use _12_downloadupdates::DownloadUpdates;

/// A configured installation operation. Dispatch stays static, including async methods.
#[enum_dispatch]
#[allow(async_fn_in_trait)]
pub trait InstallStep {
    fn stage(&self) -> Stage;

    async fn pre(
        &self,
        _settings: &mut crate::settings::Settings,
        _cfg: &crate::cfg::Config,
    ) -> Res<()> {
        Ok(())
    }

    async fn run(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        callback: &impl crate::Callback,
    ) -> Res<()>;
}

/// Serde-tagged step objects; configuration does not determine execution order.
#[enum_dispatch(InstallStep)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum Module {
    UserAdd,
    SystemSetup,
    DnfDownloadApps,
    DnfInstallApps,
    DriversCodecs,
    ApplySystemSettings,
    DownloadUpdates,
}

pub(crate) fn deserialize_modules<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<Vec<Module>, D::Error> {
    use serde::de::Error as _;
    let modules = <Vec<Module> as serde::Deserialize>::deserialize(de)?;
    for (index, module) in modules.iter().enumerate() {
        if modules[..index].iter().any(|other| other.stage() == module.stage()) {
            return Err(D::Error::custom("duplicate stage configuration"));
        }
    }
    Ok(modules)
}

/// Fill omitted stages with default objects and retain the dependency-safe order.
#[must_use]
pub fn configured_steps(cfg: &crate::cfg::Config) -> Vec<Module> {
    Stage::all().iter().map(|stage| stage.configured(cfg)).collect()
}

pub(crate) use _00_useradd::previous_uid;
pub use _00_useradd::unlock as unlock_user;
pub(crate) use _04_systemsetup::LANGPACK_PREFIXES;
pub(crate) use _05_dnfdownloadapps::{DNF_EXTRAS_ARGS, imf_packages};
pub(crate) use _07_drivers_codecs::secure_boot_enforced;
pub(crate) use _07_drivers_codecs::{Codecs, NVIDIA_PRIMARY_GPU_CONF, driver_packages};
pub use _07_drivers_codecs::{Hardware, detect_hardware};
pub use _08_apply_system_settings::fix_homed as fix_homed_ims;

pub(crate) async fn is_ostree() -> bool {
    tokio::fs::try_exists("/ostree").await.unwrap_or_default()
}

pub(crate) async fn can_use_dnf(cfg: &crate::cfg::Config) -> bool {
    !cfg.skip_dnf() && !is_ostree().await
}

async fn flatpaks_and_packages<C: crate::Callback>(
    callback: &C,
    refs: &[String],
    pkgs: &[String],
    download_only: bool,
) -> Res<()> {
    let flatpaks = async {
        if refs.is_empty() {
            return Ok(());
        }
        let verb = crate::Verb::FlatpakInstall { refs: refs.to_vec(), download_only };
        crate::dnf::run_logged(callback, "flatpak", &verb, &[], |_| {}).await
    };
    let packages = async {
        if pkgs.is_empty() {
            return Ok(());
        }
        let verb = crate::Verb::DnfExtras { pkgs: pkgs.to_vec(), download_only };
        crate::dnf::handle_dnf(callback, verb).await
    };
    // flatpak and dnf don't affect each other, and neither is cancelled when the other fails
    let (flatpaks, packages) = futures::future::join(flatpaks, packages).await;
    flatpaks.and(packages)
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    #[default]
    UserAdd,
    SystemSetup,
    DnfDownloadApps,
    DnfInstallApps,
    DriversCodecs,
    ApplySystemSettings,
    DownloadUpdates,
}

impl Stage {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::UserAdd,
            Self::SystemSetup,
            Self::DnfDownloadApps,
            Self::DnfInstallApps,
            Self::DriversCodecs,
            Self::ApplySystemSettings,
            Self::DownloadUpdates,
        ]
    }

    #[must_use]
    pub const fn l10n_id(self) -> &'static str {
        match self {
            Self::UserAdd => "steps-useradd",
            Self::SystemSetup => "steps-systemsetup",
            Self::DnfDownloadApps => "steps-dnfdownloadapps",
            Self::DnfInstallApps => "steps-dnfinstallapps",
            Self::DriversCodecs => "steps-driverscodecs",
            Self::ApplySystemSettings => "steps-applysystemsettings",
            Self::DownloadUpdates => "steps-dnfdownloadupdate",
        }
    }

    #[must_use]
    pub fn configured(self, cfg: &crate::cfg::Config) -> Module {
        cfg.steps.iter().find(|module| module.stage() == self).cloned().unwrap_or_else(
            || match self {
                Self::UserAdd => UserAdd::default().into(),
                Self::SystemSetup => SystemSetup::default().into(),
                Self::DnfDownloadApps => DnfDownloadApps::default().into(),
                Self::DnfInstallApps => DnfInstallApps::default().into(),
                Self::DriversCodecs => DriversCodecs::default().into(),
                Self::ApplySystemSettings => ApplySystemSettings::default().into(),
                Self::DownloadUpdates => DownloadUpdates::default().into(),
            },
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub stage: Stage,
    pub num: usize,
    pub count: usize,
}

impl Stage {
    async fn is_lengthy(
        self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
    ) -> bool {
        let online = !settings.nointernet;
        let dnf = can_use_dnf(cfg).await;
        let [repos, pkgs, flatpaks, scripts, coprs] = &settings.actions;
        match self {
            Self::SystemSetup => true,
            Self::UserAdd | Self::ApplySystemSettings => false,
            Self::DnfDownloadApps => {
                online
                    && (!flatpaks.is_empty()
                        || dnf && [repos, pkgs, coprs].iter().any(|a| !a.is_empty()))
            }
            Self::DnfInstallApps => {
                online
                    && (!flatpaks.is_empty()
                        || dnf && [pkgs, scripts].iter().any(|a| !a.is_empty()))
            }
            Self::DriversCodecs => {
                settings.install_codecs_drivers && online && {
                    let hw = detect_hardware().await;
                    (hw.nvidia || hw.broadcom_wifi) && !hw.secure_boot_blocks
                }
            }
            Self::DownloadUpdates => dnf && online && settings.download_updates,
        }
    }
}

pub async fn plan(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Vec<Step> {
    let mut stages = Vec::with_capacity(Stage::all().len());
    for stage in Stage::all() {
        stages.push((*stage, stage.is_lengthy(settings, cfg).await));
    }
    group_quick_stages(&stages)
}

#[must_use]
pub fn group_quick_stages(stages: &[(Stage, bool)]) -> Vec<Step> {
    let count = stages.iter().filter(|(_, lengthy)| *lengthy).count();
    let first = stages.iter().find(|(_, lengthy)| *lengthy).map_or(Stage::SystemSetup, |s| s.0);
    let mut step = Step { stage: first, num: 1, count: count.max(1) };
    let mut seen = 0;
    stages
        .iter()
        .map(|&(stage, lengthy)| {
            if lengthy {
                seen += 1;
                step = Step { stage, num: seen, ..step };
            }
            step
        })
        .collect()
}

impl From<Stage> for u8 {
    fn from(stage: Stage) -> Self {
        stage as Self
    }
}

pub async fn acmd(
    name: &'static str,
    args: impl IntoIterator<Item: AsRef<std::ffi::OsStr>>,
) -> Res<()> {
    let p = tokio::process::Command::new(name)
        .args(args)
        .output()
        .await
        .inspect_err(|e| crate::command::report_spawn_error(name, e))
        .map_err(|e| Err::FailToRunProgram(name, e))?;
    if !p.status.success() {
        return Err(Err::program_fail(name, &p));
    }
    Ok(())
}

pub(crate) use super::{root, root_with_stdin};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_numbers_are_contiguous_for_progress() {
        for (index, stage) in Stage::all().iter().enumerate() {
            assert_eq!(usize::from(u8::from(*stage)), index);
        }
    }

    #[test]
    fn stage_config_names_round_trip() {
        for stage in Stage::all() {
            let value = serde_json::to_value(stage).unwrap();
            let restored: Stage = serde_json::from_value(value).unwrap();
            assert_eq!(restored, *stage);
        }
        assert_eq!(
            serde_json::to_value(Stage::ApplySystemSettings).unwrap(),
            "apply_system_settings"
        );
    }

    #[test]
    fn configured_modules_fill_defaults_and_keep_stage_order() {
        let cfg: crate::cfg::Config = basic_toml::from_str(
            r#"
                [[steps]]
                stage = "download_updates"
                [[steps]]
                stage = "apply_system_settings"
                [[steps.apply_system_settings]]
                [steps.apply_system_settings.imf]
                start = "xdg_autostart"
                [[steps]]
                stage = "user_add"
            "#,
        )
        .unwrap();
        let modules = configured_steps(&cfg);
        assert_eq!(modules.iter().map(InstallStep::stage).collect::<Vec<_>>(), Stage::all());
        let Module::ApplySystemSettings(settings) = &modules[5] else {
            panic!("expected apply-system-settings module");
        };
        assert_eq!(settings.imf().start, Some(ImfStart::XdgAutostart));

        for module in modules {
            let value = serde_json::to_value(&module).unwrap();
            let restored: Module = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(restored.stage(), module.stage());
            assert_eq!(serde_json::to_value(restored).unwrap(), value);
        }
    }

    #[tokio::test]
    async fn enum_dispatch_forwards_async_pre_and_run_with_generic_callback() {
        // download-updates is a no-op without dnf. Unexpected callback calls fail the test.
        let settings = crate::settings::Settings { nointernet: true, ..Default::default() };
        let cfg = crate::cfg::Config {
            steps: vec![SystemSetup { skip_dnf: Some(true), ..Default::default() }.into()],
            ..Default::default()
        };
        let module: Module = DownloadUpdates::default().into();
        let mut prepared = settings.clone();
        module.pre(&mut prepared, &cfg).await.unwrap();
        module.run(&prepared, &cfg, &|_| panic!("offline step sent a message")).await.unwrap();
    }

    #[test]
    fn quick_stages_join_a_lengthy_one() {
        use Stage::*;
        let steps = group_quick_stages(&[
            (UserAdd, false),
            (SystemSetup, true),
            (DnfDownloadApps, true),
            (DnfInstallApps, true),
            (DriversCodecs, false),
            (ApplySystemSettings, false),
            (DownloadUpdates, true),
        ]);
        let shown = steps.iter().map(|s| (s.stage, s.num, s.count)).collect::<Vec<_>>();
        assert_eq!(
            shown,
            [
                (SystemSetup, 1, 4),
                (SystemSetup, 1, 4),
                (DnfDownloadApps, 2, 4),
                (DnfInstallApps, 3, 4),
                (DnfInstallApps, 3, 4),
                (DnfInstallApps, 3, 4),
                (DownloadUpdates, 4, 4),
            ]
        );
    }

    #[test]
    fn nothing_lengthy_is_one_step() {
        let steps =
            group_quick_stages(&[(Stage::UserAdd, false), (Stage::ApplySystemSettings, false)]);
        assert!(steps.iter().all(|s| *s == Step { stage: Stage::SystemSetup, num: 1, count: 1 }));
    }
}
