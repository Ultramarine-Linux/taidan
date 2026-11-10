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
        // the name goes into commands run as root, so recheck what the UI checked. reserved names
        // too, since `remove_previous` couldn't remove one on a retry. A user an earlier attempt
        // made counts as free, see `username_problem`
        if let Some(problem) = crate::passwd::username_problem(&settings.username) {
            return Err(Err::Message(format!(
                "username `{}` can't be used: {problem:?}",
                settings.username
            )));
        }
        // a passwordless wheel user would be root for anyone at the keyboard. the UI never sends
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
        // still there if `remove_previous` couldn't remove it
        if exists() {
            return Err(Err::Message(format!("user `{}` already exists", settings.username)));
        }
        // `useradd -m` takes over a home that's already there (a reinstall that kept /home), and
        // that one isn't ours to remove with the user. homectl refuses to make a user whose home
        // is there, so it never gets one it didn't make
        let keep_home = !cfg.taidan0.systemd_homed && home_exists(&settings.username);
        if keep_home {
            tracing::info!(user = settings.username, "home is already there, keeping it");
            // another user's, or a symlink to anywhere, would hand whoever controls it the new
            // admin's session
            if previous_uid(&settings.username).is_none() {
                return Err(Err::Message(format!(
                    "{} is already there and belongs to another user, or isn't a directory. \
                     Move it away or pick another username",
                    home_path(&settings.username).display()
                )));
            }
        }
        // recorded before the user is made, so the user never exists without a record, which would
        // keep the next attempt from removing it. the next attempt clears a record of a user that
        // never got made
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
    // read here, as taidan, so it can only be a file taidan may read anyway
    let image = tokio::fs::read(avatar).await.map_err(Err::Io)?;
    super::root_with_stdin(crate::Verb::SetAvatar { user: settings.username.clone() }, &image).await
}

fn hash_password(settings: &crate::settings::Settings) -> Res<String> {
    let crypt_setting = xcrypt::crypt_gensalt(None, 0, None)?;
    Ok(xcrypt::crypt(settings.passwd.expose_secret(), &crypt_setting)?)
}

async fn useradd(settings: &crate::settings::Settings) -> Res<()> {
    let pass = hash_password(settings)?;
    let fullname = crate::passwd::sanitize_fullname(&settings.fullname);
    let user = settings.username.as_str();
    // `useradd` doesn't chown a home kept from a reinstall, so the user gets its old uid back to
    // be able to get into it
    let uid = previous_uid(user);
    if let Some(uid) = uid {
        tracing::info!(uid, "giving the user the uid that owns its home");
    }
    // the hash goes through stdin since anyone can read a command's arguments. the helper sets
    // the password and locks the user until the installation finishes, see `unlock`
    let verb = crate::Verb::UserAdd { user: user.to_owned(), fullname, uid };
    super::root_with_stdin(verb, format!("{pass}\n").as_bytes())
        .await
        .wrap_msg("cannot create the user")
}

/// lets the user log in once the installation is done. until then the user is locked, so if the
/// OOBE gives up partway (it keeps crashing, the power goes out) the display manager can't log
/// in to an unfinished account. A login would make it look like someone's account (see
/// `passwd::login_state`), and the next run of the OOBE would keep it as it is
///
/// # Errors
/// - cannot unlock the user
pub async fn unlock(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    let user = settings.username.as_str();
    if !cfg.taidan0.systemd_homed {
        return super::root(crate::Verb::UnlockUser { user: user.to_owned() })
            .await
            .wrap_msg("cannot unlock the user");
    }
    // updating an inactive home needs its password, through stdin, since anyone can read a
    // command's arguments
    super::root_with_stdin(
        crate::Verb::HomedUnlock { user: user.to_owned() },
        format!("{}\n", settings.passwd.expose_secret()).as_bytes(),
    )
    .await
    .wrap_msg("cannot unlock the user with homectl")
}

async fn homed(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    let user = settings.username.as_str();
    let fullname = crate::passwd::sanitize_fullname(&settings.fullname);
    // the password goes through stdin, in plain text to encrypt the home with and hashed to check
    // logins against, so homectl doesn't need a terminal to ask for it. the user stays locked
    // until the installation finishes, see `unlock`
    let identity = serde_json::json!({
        "secret": { "password": [settings.passwd.expose_secret()] },
        "privileged": { "hashedPassword": [hash_password(settings)?] },
        "locked": true,
    });
    let skel = stage_skel(settings, cfg).await;
    // in `wheel` and `video`, groups that don't exist are left out
    let verb = crate::Verb::HomedCreate { user: user.to_owned(), fullname, skel };
    super::root_with_stdin(verb, identity.to_string().as_bytes())
        .await
        .wrap_msg("cannot create the user with homectl")
}

/// where `useradd` puts `user`'s home: under the `HOME=` from `/etc/default/useradd`, or `/home`
fn home_path(user: &str) -> std::path::PathBuf {
    let base = std::fs::read_to_string("/etc/default/useradd")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| l.trim().strip_prefix("HOME=").map(|h| h.trim().to_owned()))
        })
        .filter(|h| h.starts_with('/'))
        .unwrap_or_else(|| "/home".to_owned());
    std::path::Path::new(&base).join(user)
}

/// the uid that owns `user`'s home if it's already there, so the user can get it back and keep
/// access to their files, [`None`] if it isn't a regular uid or another user has it now
pub(crate) fn previous_uid(user: &str) -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    let meta = home_path(user).symlink_metadata().ok().filter(std::fs::Metadata::is_dir)?;
    let uid = meta.uid();
    // with a homed uid, a retry would take the user for a homed one and fail to remove it
    (crate::passwd::REGULAR_UIDS.contains(&uid)
        && crate::passwd::removable(user, uid)
        && uzers::get_user_by_uid(uid).is_none())
    .then_some(uid)
}

/// whether the home `useradd -m` would give `user` is already there. when that can't be told,
/// assume it is. wrongly keeping a home only leaves it behind, wrongly taking it over can get
/// someone's files removed
fn home_exists(user: &str) -> bool {
    // a dangling symlink counts too, userdel -r would remove what it points to
    match home_path(user).symlink_metadata() {
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
/// all start over from no user. only a regular user the record names that nobody has logged in as
/// is removed, and the record stays if it can't be, so the user is never left behind without it
async fn remove_previous() -> Res<()> {
    let Some(crate::passwd::Record { user: prev, keep_home }) = crate::passwd::record() else {
        return Ok(());
    };
    let logins = crate::passwd::login_state(&prev);
    // if lastlog2 can't tell whether it was used, remove it but keep its home
    let keep_home = keep_home || logins.is_none();
    match uzers::get_user_by_name(&prev) {
        None => tracing::info!(prev, "user from an earlier attempt is already gone"),
        // the record outlived the install that made it (it couldn't be cleared, or Taidan died
        // after making the user) and the account is in use, so it's theirs to keep
        Some(_) if logins == Some(true) => {
            tracing::warn!(prev, "user from an earlier attempt has logged in, keeping it");
        }
        // homed can't remove a user and keep its home, so one that may be in use stays put
        Some(u) if logins.is_none() && crate::passwd::HOMED_UIDS.contains(&u.uid()) => {
            return Err(Err::Message(format!(
                "cannot tell whether `{prev}` from an earlier attempt has been used, so it can't \
                 be removed safely. lastlog2 failed, see the log"
            )));
        }
        Some(u) if !crate::passwd::removable(&prev, u.uid()) => {
            return Err(Err::Message(format!(
                "user `{prev}` from an earlier attempt isn't a regular one (uid {}), not removing it",
                u.uid()
            )));
        }
        Some(u) => {
            tracing::info!(prev, "removing the user from an earlier attempt");
            // homed or not by uid rather than the config, in case that changed since the user was
            // made. the helper checks again that it's the recorded, removable user, and removes its
            // icon too
            if keep_home && !crate::passwd::HOMED_UIDS.contains(&u.uid()) {
                tracing::info!(prev, "its home may be in use, keeping it");
            }
            super::root(crate::Verb::RemoveUser { user: prev.clone(), keep_home })
                .await
                .wrap_msg("cannot remove the user from an earlier attempt")?;
        }
    }
    crate::passwd::clear_record().await
}

/// a copy of `/etc/skel` with the user's settings written in, for homed, which doesn't leave the
/// home mounted after creating it, so they can't be written there later. [`None`] if there's
/// nothing to write or it can't be made, then homed uses `/etc/skel`
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
        let a11y = crate::a11y::write_to_skel(settings, cfg, &skel).await?;
        let imf = super::_08_setup_imf::write_to_skel(settings, cfg, &skel).await?;
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
