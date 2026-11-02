use secrecy::ExposeSecret;

use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct UserAdd;
impl super::Step for UserAdd {
    #[tracing::instrument]
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
        remove_previous(cfg).await?;
        let created =
            if cfg.taidan0.systemd_homed { homed(settings).await } else { useradd(settings).await };
        // recorded even if the command failed, it can leave the user behind (useradd exits with 12
        // when only the home couldn't be made), so the next attempt removes it
        if uzers::get_user_by_name(&settings.username).is_some() {
            let recorded = crate::passwd::record_created(&settings.username).await;
            created?;
            recorded?;
        } else {
            created?;
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
    super::super::root(
        "install",
        [std::ffi::OsStr::new("-Dm644"), avatar.as_os_str(), icon.as_ref()],
    )
    .await?;
    // a new user has no settings there yet, so this doesn't overwrite anything of theirs
    let user = crate::TEMP_DIR.join("accountsservice-user");
    tokio::fs::write(&user, format!("[User]\nIcon={icon}\nSystemAccount=false\n"))
        .await
        .map_err(Err::Io)?;
    let dest = format!("/var/lib/AccountsService/users/{}", settings.username);
    super::super::root("install", [std::ffi::OsStr::new("-Dm600"), user.as_os_str(), dest.as_ref()])
        .await
}

fn hash_password(settings: &crate::settings::Settings) -> Res<String> {
    let crypt_setting = xcrypt::crypt_gensalt(None, 0, None)?;
    Ok(xcrypt::crypt(settings.passwd.expose_secret(), &crypt_setting)?)
}

async fn useradd(settings: &crate::settings::Settings) -> Res<()> {
    let pass = hash_password(settings)?;
    let fullname = crate::passwd::sanitize_fullname(&settings.fullname);
    let user = settings.username.as_str();
    super::super::root("useradd", ["-p", &pass, "-c", &fullname, "-m", "-G", "wheel", "--", user])
        .await
        .wrap_msg("cannot create the user")
}

async fn homed(settings: &crate::settings::Settings) -> Res<()> {
    let user = settings.username.as_str();
    let fullname = crate::passwd::sanitize_fullname(&settings.fullname);
    // the password goes through stdin, as the plain text one homed encrypts the home with and the
    // hash it checks logins against, so homectl doesn't need a terminal to ask for it
    let identity = serde_json::json!({
        "secret": { "password": [settings.passwd.expose_secret()] },
        "privileged": { "hashedPassword": [hash_password(settings)?] },
    });
    let skel = stage_skel(settings).await;
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

/// removes the user an earlier, unfinished attempt created (see
/// [`crate::passwd::CREATED_RECORD`]), so retrying, renaming the user or changing the password
/// all start over from no user. Only a regular user the record names is removed
async fn remove_previous(cfg: &crate::cfg::Config) -> Res<()> {
    let Some(prev) = crate::passwd::recorded_user() else { return Ok(()) };
    match uzers::get_user_by_name(&prev) {
        None => tracing::info!(prev, "user from an earlier attempt is already gone"),
        Some(u) if !crate::passwd::removable(&prev, u.uid()) => {
            tracing::warn!(prev, uid = u.uid(), "not removing a user that isn't a regular one");
        }
        Some(_) => {
            tracing::info!(prev, "removing the user from an earlier attempt");
            if cfg.taidan0.systemd_homed {
                super::super::root("homectl", ["remove", prev.as_str()]).await
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
async fn stage_skel(settings: &crate::settings::Settings) -> Option<std::path::PathBuf> {
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
        let imf = super::_08_setup_imf::write_fcitx5_to_skel(settings, &skel).await?;
        Ok::<_, Err>(a11y || imf)
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
