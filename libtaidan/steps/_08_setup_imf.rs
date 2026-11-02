use tokio::io::AsyncWriteExt;

use super::super::i18n;
use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetupImf;
impl super::Step for SetupImf {
    #[tracing::instrument]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        if settings.ims.is_empty() || settings.nointernet {
            return Ok(());
        }
        if let Err(e) = match settings.imf {
            i18n::I18nImf::IBus if cfg.taidan0.systemd_homed => {
                // the home isn't mounted, and dconf's database can't go in the skeleton
                tracing::warn!("cannot set up IBus for a systemd-homed user, skipping");
                Ok(())
            }
            i18n::I18nImf::IBus => write_ibus_profile(settings).await,
            // written into the skeleton the home was created from, see `write_fcitx5_to_skel`
            i18n::I18nImf::Fcitx5 if cfg.taidan0.systemd_homed => Ok(()),
            i18n::I18nImf::Fcitx5 => write_fcitx5_profile(settings).await,
        } {
            tracing::warn!(?e, "cannot setup IMFs");
        }
        Ok(())
    }
}

// kwinrc config for selecting fcitx5 as the virtual input
const KWINRC_FCITX5: &[u8] = b"
[Wayland]
InputMethod[$e]=/usr/share/applications/fcitx5-wayland-launcher.desktop
VirtualKeyboardEnabled=true
";

/// `~/.config/fcitx5/profile` with the keyboard layout and the chosen input methods
fn fcitx5_profile(
    crate::settings::Settings { ims, kb_layout, kb_variant, .. }: &crate::settings::Settings,
) -> String {
    let default_group_name = "Default"; // FIXME: this should be configurable? (was t!("default"))
    let var = kb_variant.as_ref().map(|variant| format!("-{variant}")).unwrap_or_default();
    let mut buf = format!(
        "\
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

    let refs = ims.iter().filter_map(|im| i18n::find_im(im)?.fcitx5_ref);
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

/// appends `content` to `path` as `user`, so the file is theirs
async fn tee_as(user: &str, path: &str, content: &[u8]) -> Res<()> {
    let mut p = tokio::process::Command::new("pkexec")
        .args(["--user", user, "tee"])
        .args(["-a", path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Err::FailToRunProgram("pkexec", e))?;
    if let Some(mut stdin) = p.stdin.take() {
        (stdin.write_all(content).await.map_err(Err::Io))
            .wrap_msg("cannot write to pkexec tee stdin")?;
    }

    let p = (p.wait_with_output().await)
        .map_err(|e| Err::FailToRunProgram("pkexec", e))
        .wrap_msg("cannot tee")?;

    if !p.status.success() {
        return Err(Err::ProgramFail {
            name: "pkexec (tee)",
            output: String::from_utf8_lossy(&p.stderr).to_string(),
            rc: p.status.code(),
        })
        .wrap_message(|_| format!("failed to write to {path}"));
    }
    Ok(())
}

async fn write_fcitx5_profile(settings: &crate::settings::Settings) -> Res<()> {
    let username = &settings.username;
    crate::pkexec(username, "mkdir", &["-p", &format!("/home/{username}/.config/fcitx5/")])
        .await
        .wrap_msg("cannot create ~/.config/fcitx5/")?;

    let profile_path = format!("/home/{username}/.config/fcitx5/profile");
    tee_as(username, &profile_path, fcitx5_profile(settings).as_bytes()).await?;

    // https://invent.kde.org/plasma/kwin/-/blob/master/src/kcms/virtualkeyboard/virtualkeyboardsettings.kcfg?ref_type=heads
    let kwinrc_path = format!("/home/{username}/.config/kwinrc");
    tee_as(username, &kwinrc_path, KWINRC_FCITX5).await
}

/// writes the Fcitx5 config into `skel`, a copy of `/etc/skel` the new user's home is made from,
/// for systemd-homed, where the home isn't mounted when [`SetupImf`] runs
///
/// # Errors
/// - cannot write the files
pub async fn write_fcitx5_to_skel(
    settings: &crate::settings::Settings,
    skel: &std::path::Path,
) -> Res<bool> {
    if settings.ims.is_empty() || settings.nointernet || !settings.imf.is_fcitx5() {
        return Ok(false);
    }
    let config = skel.join(".config");
    tokio::fs::create_dir_all(config.join("fcitx5")).await.map_err(Err::Io)?;
    tokio::fs::write(config.join("fcitx5/profile"), fcitx5_profile(settings))
        .await
        .map_err(Err::Io)?;
    let mut kwinrc = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(config.join("kwinrc"))
        .await
        .map_err(Err::Io)?;
    kwinrc.write_all(KWINRC_FCITX5).await.map_err(Err::Io)?;
    Ok(true)
}

async fn write_ibus_profile(
    crate::settings::Settings { username, ims, .. }: &crate::settings::Settings,
) -> Res<()> {
    let engines = ims
        .iter()
        .filter_map(|s| i18n::find_im(s)?.ibus_ref)
        .map(|r| format!("'{r}'"))
        .collect::<Vec<_>>()
        .join(", ");
    if engines.is_empty() {
        return Ok(());
    }

    // Use dbus-launch to run dconf commands as the target user
    let dbus_args = format!(
        "dconf write /desktop/ibus/general/use-system-keyboard-layout true && \
        dconf write /desktop/ibus/general/engines-order \"[{engines}]\" && \
        dconf write /desktop/ibus/general/preload-engines \"[{engines}]\""
    );

    let p = tokio::process::Command::new("pkexec")
        .args([
            "--user",
            username,
            "dbus-launch",
            "--sh-syntax",
            "--exit-with-session",
            "bash",
            "-c",
            &dbus_args,
        ])
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Err::FailToRunProgram("pkexec", e))
        .wrap_msg("fail to spawn pkexec for dbus command")?;

    let p = (p.wait_with_output().await)
        .map_err(|e| Err::FailToRunProgram("pkexec", e))
        .wrap_msg("cannot run dbus-launch with pkexec")?;

    if !p.status.success() {
        return Err(Err::ProgramFail {
            name: "pkexec (dbus-launch)",
            output: String::from_utf8_lossy(&p.stderr).to_string(),
            rc: p.status.code(),
        })
        .wrap_msg("failed to set the IBus engines");
    }

    Ok(())
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
        assert!(profile.contains("Default Layout=us\n"));
        assert!(profile.contains("[Groups/0/Items/1]\n# Name\nName=pinyin\n"));
        assert!(profile.contains("[Groups/0/Items/2]\n# Name\nName=mozc\n"));
        assert!(!profile.contains("Items/3"));
    }
}
