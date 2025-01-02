use crate::prelude::*;

use super::theme::pkexec;

pub fn list_layouts() -> color_eyre::Result<Vec<String>> {
    Ok(std::process::Command::new("localectl")
        .arg("list-x11-keymap-layouts")
        .stdout(std::process::Stdio::piped())
        .output()
        .wrap_err("failed to run `localectl`")?
        .stdout
        .split(|&c| c == b'\n')
        .map(String::from_utf8_lossy)
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .collect())
}
pub fn list_variants(layout: &str) -> color_eyre::Result<Vec<String>> {
    Ok(std::process::Command::new("localectl")
        .arg("list-x11-keymap-variants")
        .arg(layout)
        .stdout(std::process::Stdio::piped())
        .output()
        .wrap_err("failed to run `localectl`")?
        .stdout
        .split(|&c| c == b'\n')
        .map(String::from_utf8_lossy)
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .collect())
}

async fn set_kde_keymap(user: &str, layout: &str, variant: &str) -> color_eyre::Result<()> {
    let args = [
        "--file",
        "~/.config/kxkbrc",
        "--group",
        "Layout",
        "--key",
        "LayoutList",
        layout,
    ];
    pkexec(user, "kwriteconfig6", &args).await?;
    let args = [
        "--file",
        "~/.config/kxkbrc",
        "--group",
        "Layout",
        "--key",
        "VariantList",
        variant,
    ];
    pkexec(user, "kwriteconfig6", &args).await?;
    let args = [
        ["--session", "--type=signal"],
        ["--reply-timeout=100", "--reply-timeout=100"],
        ["/Layouts", "org.kde.keyboard.reloadConfig"],
    ];
    pkexec(user, "dbus-send", &args.concat()).await?;
    Ok(())
}
async fn set_gsettings_keymap(user: &str, layout: &str, variant: &str) -> color_eyre::Result<()> {
    // gsettings describe org.gnome.desktop.input-sources sources
    // List of input source identifiers available. Each source is specified as a tuple of 2 strings. The first string is the type and can be one of “xkb” or “ibus”. For “xkb” sources the second string is “xkb_layout+xkb_variant” or just “xkb_layout” if a XKB variant isn’t needed. For “ibus” sources the second string is the IBus engine name. An empty list means that the X server’s current XKB layout and variant won’t be touched and IBus won’t be used.
    let args = [
        ["set", "org.gnome.desktop.input-sources"],
        ["sources", &format!("[('xkb', '{layout}+{variant}')]")],
    ];
    pkexec(user, "gsettings", &args.concat()).await?;
    Ok(())
}
async fn set_gsettings_im(user: &str, im: &str) -> color_eyre::Result<()> {
    // gsettings describe org.gnome.desktop.input-sources sources
    // List of input source identifiers available. Each source is specified as a tuple of 2 strings. The first string is the type and can be one of “xkb” or “ibus”. For “xkb” sources the second string is “xkb_layout+xkb_variant” or just “xkb_layout” if a XKB variant isn’t needed. For “ibus” sources the second string is the IBus engine name. An empty list means that the X server’s current XKB layout and variant won’t be touched and IBus won’t be used.
    let args = [
        ["set", "org.gnome.desktop.input-sources"],
        ["sources", &format!("[('ibus', '{im}')]")],
    ];
    pkexec(user, "gsettings", &args.concat()).await?;
    Ok(())
}

pub async fn set_keymap(user: &str, layout: &str, variant: &str) -> color_eyre::Result<()> {
    if tokio::fs::try_exists("/usr/share/kwriteconfig6")
        .await
        .is_ok()
    {
        set_kde_keymap(user, layout, variant).await?;
    } else {
        set_gsettings_keymap(user, layout, variant).await?;
    }
    Ok(())
}
