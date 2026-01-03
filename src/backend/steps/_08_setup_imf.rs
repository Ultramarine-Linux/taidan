use tokio::io::AsyncWriteExt;

use super::super::i18n;
use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetupImf;
impl super::Step for SetupImf {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        sender: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        if settings.ims.is_empty() || settings.nointernet {
            return Ok(());
        }
        if let Err(e) = match CFG.i18n.imf {
            i18n::I18nImf::IBus => write_ibus_profile(settings).await,
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

async fn write_fcitx5_profile(
    crate::backend::settings::Settings {
        username,
        ims,
        kb_layout,
        kb_variant,
        ..
    }: &crate::backend::settings::Settings,
) -> color_eyre::Result<()> {
    let default_group_name = t!("default");
    crate::backend::pkexec(
        username,
        "mkdir",
        &["-p", &format!("/home/{username}/.config/fcitx5/")],
    )
    .await
    .wrap_err("cannot create ~/.config/fcitx5/")?;

    let profile_path = format!("/home/{username}/.config/fcitx5/profile");
    let mut p = tokio::process::Command::new("pkexec")
        .args(["--user", username, "tee"])
        .args(["-a", &profile_path])
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .wrap_err("failed to spawn pkexec")?;
    let mut buf = vec![];
    buf.extend_from_slice(
        format!(
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
            var = &kb_variant
                .as_ref()
                .map(|variant| format!("-{variant}"))
                .unwrap_or_default(),
        )
        .as_bytes(),
    );

    for (i, &im) in ims.iter().enumerate() {
        buf.extend_from_slice(
            format!(
                "\
                    [Groups/0/Items/{}]\n\
                    # Name\n\
                    Name={}\n\
                    # Layout\n\
                    Layout=\n\n\
                ",
                i + 1,
                i18n::str_to_im(im).fcitx5_ref.unwrap()
            )
            .as_bytes(),
        );
    }

    buf.extend_from_slice(format!("[GroupOrder]\n0={default_group_name}\n").as_bytes());

    p.stdin.as_mut().unwrap().write_all(&buf).await?;
    p.stdin.take().unwrap(); // drop the stream

    let p = p.wait_with_output().await?;

    if !p.status.success() {
        return Err(eyre!("failed to write to ~/.config/fcitx5/profile")
            .note(String::from_utf8_lossy(&p.stderr).to_string()));
    }

    // https://invent.kde.org/plasma/kwin/-/blob/master/src/kcms/virtualkeyboard/virtualkeyboardsettings.kcfg?ref_type=heads
    let kwinrc_path = format!("/home/{username}/.config/kwinrc");
    let mut p = tokio::process::Command::new("pkexec")
        .args(["--user", username, "tee"])
        .args(["-a", &kwinrc_path])
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .wrap_err("failed to spawn pkexec")?;
    p.stdin.as_mut().unwrap().write_all(KWINRC_FCITX5).await?;
    p.stdin.take().unwrap(); // drop the stream

    let p = p.wait_with_output().await?;

    if !p.status.success() {
        return Err(eyre!("failed to write to ~/.config/kwinrc")
            .note(String::from_utf8_lossy(&p.stderr).to_string()));
    }

    Ok(())
}

async fn write_ibus_profile(
    crate::backend::settings::Settings { username, ims, .. }: &crate::backend::settings::Settings,
) -> color_eyre::Result<()> {
    if ims.is_empty() {
        return Ok(());
    }

    let engines = ims
        .iter()
        .map(|s| format!("'{}'", i18n::str_to_im(s).ibus_ref.unwrap()))
        .collect::<Vec<_>>()
        .join(", ");

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
        .wrap_err("failed to spawn pkexec for dbus commands")?;

    let output = p.wait_with_output().await?;

    if !output.status.success() {
        return Err(eyre!("failed to set ibus configuration via dbus")
            .note(String::from_utf8_lossy(&output.stderr).to_string()));
    }

    Ok(())
}
