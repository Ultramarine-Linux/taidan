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
        if let Some(problem) = crate::passwd::username_problem(&settings.username) {
            return Err(Err::Message(format!(
                "username `{}` can't be used: {problem:?}",
                settings.username
            )));
        }
        if settings.passwd.expose_secret().is_empty() {
            return Err(Err::Msg("refusing to create a user without a password"));
        }
        let user_exists = || uzers::get_user_by_name(&settings.username).is_some();
        let exists_err = || Err::Message(format!("user `{}` already exists", settings.username));
        if user_exists() && !crate::passwd::created_by_us(&settings.username) {
            return Err(exists_err());
        }
        remove_previous().await?;
        if user_exists() {
            return Err(exists_err());
        }
        // `useradd -m` takes over an existing home, homectl refuses to
        let keep_home = !cfg.taidan0.systemd_homed && home_exists(&settings.username);
        if keep_home {
            tracing::info!(user = settings.username, "home is already there, keeping it");
            if previous_uid(&settings.username).is_none() {
                return Err(Err::Message(format!(
                    "{} is already there and belongs to another user, or isn't a directory. \
                     Move it away or pick another username",
                    home_path(&settings.username).display()
                )));
            }
        }
        crate::passwd::record_created(&settings.username, keep_home).await?;
        if cfg.taidan0.systemd_homed {
            homed(settings, cfg).await?;
        } else {
            useradd(settings).await?;
        }
        if let Err(e) = set_avatar(settings).await {
            tracing::warn!(?e, "cannot set user icon");
        }
        Ok(())
    }
}

async fn set_avatar(settings: &crate::settings::Settings) -> Res<()> {
    let Some(avatar) = &settings.avatar else { return Ok(()) };
    let image_read_as_taidan = tokio::fs::read(avatar).await.map_err(Err::Io)?;
    let verb = crate::Verb::SetAvatar { user: settings.username.clone() };
    super::root_with_stdin(verb, &image_read_as_taidan).await
}

fn hash_password(settings: &crate::settings::Settings) -> Res<String> {
    let crypt_setting = xcrypt::crypt_gensalt(None, 0, None)?;
    Ok(xcrypt::crypt(settings.passwd.expose_secret(), &crypt_setting)?)
}

async fn useradd(settings: &crate::settings::Settings) -> Res<()> {
    let password_hash = hash_password(settings)?;
    let fullname = crate::passwd::sanitize_fullname(&settings.fullname);
    let user = settings.username.as_str();
    let uid = previous_uid(user);
    if let Some(uid) = uid {
        tracing::info!(uid, "giving the user the uid that owns its home");
    }
    let verb = crate::Verb::UserAdd { user: user.to_owned(), fullname, uid };
    super::root_with_stdin(verb, format!("{password_hash}\n").as_bytes())
        .await
        .wrap_msg("cannot create the user")
}

pub async fn unlock(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    let user = settings.username.as_str();
    if !cfg.taidan0.systemd_homed {
        return super::root(crate::Verb::UnlockUser { user: user.to_owned() })
            .await
            .wrap_msg("cannot unlock the user");
    }
    // homectl needs the password to update an inactive home
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
    let identity = serde_json::json!({
        "secret": { "password": [settings.passwd.expose_secret()] },
        "privileged": { "hashedPassword": [hash_password(settings)?] },
        "locked": true,
    });
    let skel = stage_skel(settings, cfg).await;
    let verb = crate::Verb::HomedCreate { user: user.to_owned(), fullname, skel };
    super::root_with_stdin(verb, identity.to_string().as_bytes())
        .await
        .wrap_msg("cannot create the user with homectl")
}

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

pub(crate) fn previous_uid(user: &str) -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    let meta = home_path(user).symlink_metadata().ok().filter(std::fs::Metadata::is_dir)?;
    let uid = meta.uid();
    (crate::passwd::REGULAR_UIDS.contains(&uid)
        && crate::passwd::removable(user, uid)
        && uzers::get_user_by_uid(uid).is_none())
    .then_some(uid)
}

fn home_exists(user: &str) -> bool {
    match home_path(user).symlink_metadata() {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => {
            tracing::warn!(?e, "cannot tell whether the home is already there, keeping it");
            true
        }
    }
}

async fn remove_previous() -> Res<()> {
    let Some(crate::passwd::Record { user: prev, keep_home }) = crate::passwd::record() else {
        return Ok(());
    };
    let logins = crate::passwd::login_state(&prev);
    let logged_in = logins == Some(true);
    let login_unknown = logins.is_none();
    let keep_home = keep_home || login_unknown;
    match uzers::get_user_by_name(&prev) {
        None => tracing::info!(prev, "user from an earlier attempt is already gone"),
        Some(_) if logged_in => {
            tracing::warn!(prev, "user from an earlier attempt has logged in, keeping it");
        }
        // homectl can't remove a user and keep its home
        Some(u) if login_unknown && crate::passwd::HOMED_UIDS.contains(&u.uid()) => {
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

// homectl doesn't leave a new home mounted, so settings can't be written there later
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
