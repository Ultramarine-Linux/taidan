use secrecy::ExposeSecret;

use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct UserAdd;
impl super::Step for UserAdd {
    #[tracing::instrument(skip(settings))]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        // it goes into commands run as root, so make sure the UI's check held
        if matches!(
            crate::passwd::username_problem(&settings.username),
            Some(crate::passwd::UsernameProblem::Invalid | crate::passwd::UsernameProblem::TooLong)
        ) {
            return Err(Err::Message(format!("invalid username `{}`", settings.username)));
        }
        // a passwordless wheel user would be root for anyone at the keyboard. The UI never sends
        // one, but settings from elsewhere (tdnx) can leave it out
        if settings.passwd.expose_secret().is_empty() {
            return Err(Err::Msg("refusing to create a user without a password"));
        }
        // only one an earlier attempt made may be taken over, see `remove_previous`
        let exists = || uzers::get_user_by_name(&settings.username).is_some();
        if exists() && !crate::passwd::created_by_us(&settings.username) {
            return Err(Err::Message(format!("user `{}` already exists", settings.username)));
        }
        remove_previous().await?;
        // left behind if it couldn't be removed
        if exists() {
            return Err(Err::Message(format!("user `{}` already exists", settings.username)));
        }
        // `useradd -m` takes over a home that's already there (a reinstall that kept /home), and
        // that one isn't ours to remove with the user. homectl refuses to make a user whose home
        // is there, so it never gets one it didn't make
        let keep_home = !cfg.taidan0.systemd_homed && home_exists(&settings.username);
        if keep_home {
            tracing::info!(user = settings.username, "home is already there, keeping it");
        }
        // before the user is made, so it's never there without the record, which would keep the
        // next attempt from removing it. A record of a user that never got made is cleared then
        crate::passwd::record_created(&settings.username, keep_home).await?;
        if cfg.taidan0.systemd_homed {
            homed(settings, cfg).await?;
        } else {
            useradd(settings).await?;
        }
        // not worth failing the setup over
        if let Err(e) = set_avatar(settings).await {
            tracing::warn!(?e, "cannot set user icon");
        }
        Ok(())
    }
}

/// sets the user's icon through AccountsService, where display managers (GDM, SDDM, LightDM) and
/// desktops look for it
async fn set_avatar(settings: &crate::settings::Settings) -> Res<()> {
    let Some(avatar) = &settings.avatar else { return Ok(()) };
    let icon = format!("/var/lib/AccountsService/icons/{}", settings.username);
    super::root("install", [std::ffi::OsStr::new("-Dm644"), avatar.as_os_str(), icon.as_ref()])
        .await?;
    // a new user has no settings there yet, so this doesn't overwrite anything of theirs
    let user = crate::TEMP_DIR.join("accountsservice-user");
    tokio::fs::write(&user, format!("[User]\nIcon={icon}\nSystemAccount=false\n"))
        .await
        .map_err(Err::Io)?;
    let dest = format!("/var/lib/AccountsService/users/{}", settings.username);
    super::root("install", [std::ffi::OsStr::new("-Dm600"), user.as_os_str(), dest.as_ref()]).await
}

fn hash_password(settings: &crate::settings::Settings) -> Res<String> {
    let crypt_setting = xcrypt::crypt_gensalt(None, 0, None)?;
    Ok(xcrypt::crypt(settings.passwd.expose_secret(), &crypt_setting)?)
}

async fn useradd(settings: &crate::settings::Settings) -> Res<()> {
    let pass = hash_password(settings)?;
    let fullname = crate::passwd::sanitize_fullname(&settings.fullname);
    let user = settings.username.as_str();
    super::super::root("useradd", ["-c", &fullname, "-m", "-G", "wheel", "--", user])
        .await
        .wrap_msg("cannot create the user")?;
    // through stdin, since anyone can read a command's arguments
    super::super::root_with_stdin("chpasswd", ["-e"], format!("{user}:{pass}\n").as_bytes())
        .await
        .wrap_msg("cannot set the password")
}

async fn homed(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    let user = settings.username.as_str();
    let fullname = crate::passwd::sanitize_fullname(&settings.fullname);
    // the password goes through stdin, as the plain text one homed encrypts the home with and the
    // hash it checks logins against, so homectl doesn't need a terminal to ask for it
    let identity = serde_json::json!({
        "secret": { "password": [settings.passwd.expose_secret()] },
        "privileged": { "hashedPassword": [hash_password(settings)?] },
    });
    let skel = stage_skel(settings, cfg).await;
    let mut args: Vec<std::ffi::OsString> = vec![
        "create".into(),
        "--identity=-".into(),
        "--real-name".into(),
        fullname.into(),
        // groups that don't exist are left out
        "--member-of=wheel,video".into(),
        "--enforce-password-policy=no".into(),
    ];
    if let Some(skel) = skel {
        let mut arg = std::ffi::OsString::from("--skel=");
        arg.push(skel);
        args.push(arg);
    }
    // validated to start with a letter, so it can't be taken for an option
    args.push(user.into());
    super::super::root_with_stdin("homectl", args, identity.to_string().as_bytes())
        .await
        .wrap_msg("cannot create the user with homectl")
}

/// whether the home `useradd -m` would give `user` is already there: under the `HOME=` from
/// `/etc/default/useradd`, or `/home`. If it can't be told, it's taken to be, since that only
/// leaves a home behind while the opposite can remove someone's files
fn home_exists(user: &str) -> bool {
    let base = std::fs::read_to_string("/etc/default/useradd")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| l.trim().strip_prefix("HOME=").map(|h| h.trim().to_owned()))
        })
        .filter(|h| h.starts_with('/'))
        .unwrap_or_else(|| "/home".to_owned());
    // a dangling symlink counts too, userdel -r would remove what it points to
    match std::path::Path::new(&base).join(user).symlink_metadata() {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => {
            tracing::warn!(?e, "cannot tell whether the home is already there, keeping it");
            true
        }
    }
}

/// removes the user an earlier, unfinished attempt created (see
/// [`crate::passwd::CREATED_RECORD`]), so retrying, renaming the user or changing the password
/// all start over from no user. Only a regular user the record names that nobody has logged in as
/// is removed, and the record stays if it can't be, so the user is never left behind without it
async fn remove_previous() -> Res<()> {
    let Some(crate::passwd::Record { user: prev, keep_home }) = crate::passwd::record() else {
        return Ok(());
    };
    match uzers::get_user_by_name(&prev) {
        None => tracing::info!(prev, "user from an earlier attempt is already gone"),
        // the record outlived the install that made it (it couldn't be cleared, or Taidan died
        // after making the user) and the account is in use, so it's theirs to keep
        Some(_) if crate::passwd::has_logged_in(&prev) => {
            tracing::warn!(prev, "user from an earlier attempt has logged in, keeping it");
        }
        Some(u) if !crate::passwd::removable(&prev, u.uid()) => {
            return Err(Err::Message(format!(
                "user `{prev}` from an earlier attempt isn't a regular one (uid {}), not removing it",
                u.uid()
            )));
        }
        Some(u) => {
            tracing::info!(prev, "removing the user from an earlier attempt");
            // by uid rather than the config, in case it changed since the user was made
            if crate::passwd::HOMED_UIDS.contains(&u.uid()) {
                super::super::root("homectl", ["remove", prev.as_str()]).await
            } else if keep_home {
                tracing::info!(prev, "its home was there before it, keeping it");
                super::super::root("userdel", ["--", prev.as_str()]).await
            } else {
                super::super::root("userdel", ["-r", "--", prev.as_str()]).await
            }
            .wrap_msg("cannot remove the user from an earlier attempt")?;
            let icon = format!("/var/lib/AccountsService/icons/{prev}");
            let user = format!("/var/lib/AccountsService/users/{prev}");
            if let Err(e) = super::super::root("rm", ["-f", &icon, &user]).await {
                tracing::warn!(?e, "cannot remove the old user icon");
            }
        }
    }
    crate::passwd::clear_record().await
}

/// a copy of `/etc/skel` with the user's settings written in, for homed, where the home isn't
/// mounted after it's created so they can't be written there later, [`None`] if there's nothing to
/// write or it can't be made, then homed uses `/etc/skel`
async fn stage_skel(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
) -> Option<std::path::PathBuf> {
    let skel = crate::TEMP_DIR.join("skel");
    let res = async {
        if tokio::fs::try_exists(&skel).await.map_err(Err::Io)? {
            tokio::fs::remove_dir_all(&skel).await.map_err(Err::Io)?;
        }
        tokio::fs::create_dir_all(&skel).await.map_err(Err::Io)?;
        if tokio::fs::try_exists("/etc/skel").await.unwrap_or_default() {
            let to = skel.to_string_lossy();
            super::acmd("cp", &["-rT", "--no-preserve=ownership", "/etc/skel", &to]).await?;
        }
        let a11y = crate::a11y::write_to_skel(settings, &skel).await?;
        let imf = super::_08_setup_imf::write_fcitx5_to_skel(settings, cfg, &skel).await?;
        let theme = super::_01_settheme::write_to_skel(settings, cfg, &skel).await?;
        let keymap = crate::i18n::write_keymap_to_skel(settings, &skel).await?;
        Ok::<_, Err>(a11y || imf || theme || keymap)
    };
    match res.await {
        Ok(true) => Some(skel),
        Ok(false) => None,
        Err(e) => {
            tracing::warn!(?e, "cannot make the home skeleton, using /etc/skel");
            None
        }
    }
}
