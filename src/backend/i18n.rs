use super::theme::pkexec;

taidan_proc_macros::keymap!(LAYOUTS);

async fn set_kde_keymap(user: &str, layout: &str, variant: Option<&str>) -> color_eyre::Result<()> {
    let args = [
        "--file",
        "kxkbrc",
        "--group",
        "Layout",
        "--key",
        "LayoutList",
        layout,
    ];
    pkexec(user, "kwriteconfig6", &args).await?;
    let variant = variant.unwrap_or("");
    let args = [
        "--file",
        "kxkbrc",
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
async fn set_gsettings_keymap(
    user: &str,
    layout: &str,
    variant: Option<&str>,
) -> color_eyre::Result<()> {
    // gsettings describe org.gnome.desktop.input-sources sources
    // List of input source identifiers available. Each source is specified as a tuple of 2 strings. The first string is the type and can be one of “xkb” or “ibus”. For “xkb” sources the second string is “xkb_layout+xkb_variant” or just “xkb_layout” if a XKB variant isn’t needed. For “ibus” sources the second string is the IBus engine name. An empty list means that the X server’s current XKB layout and variant won’t be touched and IBus won’t be used.
    let name = format!(
        "{layout}{}",
        variant.map(|v| format!("+{v}")).unwrap_or_default()
    );
    let args = [
        ["set", "org.gnome.desktop.input-sources"],
        ["sources", &format!("[('xkb', '{name}')]")],
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

pub async fn set_keymap(
    user: Option<&str>,
    layout: &str,
    variant: Option<&str>,
) -> color_eyre::Result<()> {
    let mut tmp = std::ffi::OsString::default();
    let user = user.unwrap_or_else(|| {
        tmp = uzers::get_current_username().expect("can't get current username");
        tmp.to_str().unwrap()
    });
    if (tokio::fs::try_exists("/usr/share/kwriteconfig6").await).is_ok() {
        set_kde_keymap(user, layout, variant).await
    } else {
        set_gsettings_keymap(user, layout, variant).await
    }
}
