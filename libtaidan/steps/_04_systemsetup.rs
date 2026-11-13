use crate::prelude::*;

pub(super) async fn run(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<()> {
    if let Some(ids) = settings.tweaks.ids() {
        apply_tweaks(settings, cfg, ids).await?;
    } else {
        tracing::warn!("the settings don't say which tweaks to turn on, leaving them as they are");
    }
    apply_location(settings).await;
    if let Err(e) = crate::i18n::apply_keymap_to_user(settings, cfg).await {
        tracing::warn!(?e, "cannot carry over the keyboard layout");
    }
    if let Err(e) = crate::a11y::apply_to_user(settings, cfg).await {
        tracing::warn!(?e, "cannot carry over the accessibility settings");
    }
    if !cfg.taidan0.systemd_homed
        && video_group_exists().await
        && let Err(e) =
            crate::root(crate::Verb::AddToVideo { user: settings.username.clone() }).await
    {
        tracing::warn!(?e, "cannot add the user to the video group");
    }
    Ok(())
}

pub(super) async fn pre(
    settings: &mut crate::settings::Settings,
    cfg: &crate::cfg::Config,
) -> Res<()> {
    if settings.nointernet || settings.locale.is_empty() || !super::can_use_dnf(cfg).await {
        return Ok(());
    }
    let candidates = langpack_candidates(&settings.locale);
    match packages_in_repos(&candidates).await {
        Ok(found) => {
            if let Some(pkg) = candidates.into_iter().find(|c| found.contains(c)) {
                tracing::info!(pkg, "Installing the language pack");
                settings.actions[1].push(pkg);
            } else {
                tracing::info!(locale = settings.locale, "No language pack for this locale");
            }
        }
        Err(e) => tracing::warn!(?e, "cannot look up language packs"),
    }
    Ok(())
}

async fn apply_tweaks(
    settings: &crate::settings::Settings,
    cfg: &crate::cfg::Config,
    wanted_on: &[String],
) -> Res<()> {
    let sett = crate::tweaks::settings_json(settings)?;
    let mut recorded = crate::tweaks::recorded_on();
    let tweaks = crate::tweaks::available(super::can_use_dnf(cfg).await).await;
    for tweak in tweaks {
        let id = tweak.id().to_string_lossy().into_owned();
        let started = format!("{}{id}", crate::tweaks::STARTED);
        let on = wanted_on.contains(&id);
        let partly_applied = recorded.contains(&started);
        if !partly_applied && on == recorded.contains(&id) {
            continue;
        }
        if on && settings.nointernet && tweak.needs_internet {
            tracing::info!(id, "skipping tweak that needs the internet");
            if partly_applied && tweak.run(&sett, false).await {
                recorded.retain(|r| *r != started);
                persist_record(&recorded).await;
            }
            continue;
        }
        recorded.retain(|r| *r != id && *r != started);
        if on {
            persist_record(&[recorded.as_slice(), std::slice::from_ref(&started)].concat()).await;
        }
        if !tweak.run(&sett, on).await {
            recorded.push(started);
        } else if on {
            recorded.push(id);
        }
        persist_record(&recorded).await;
    }
    Ok(())
}

async fn persist_record(ids: &[String]) {
    if let Err(e) = crate::tweaks::record_on(ids).await {
        tracing::warn!(?e, "cannot record the tweaks turned on");
    }
}

async fn apply_location(settings: &crate::settings::Settings) {
    if let Some(on) = settings.location
        && let Err(e) = crate::root(crate::Verb::Location { on }).await
    {
        tracing::warn!(?e, on, "cannot switch location services");
    }
}

async fn video_group_exists() -> bool {
    tokio::fs::read_to_string("/etc/group")
        .await
        .is_ok_and(|groups| groups.lines().any(|l| l.starts_with("video:")))
}

pub(crate) const LANGPACK_PREFIXES: [&str; 3] =
    ["ultramarine-langpacks-", "langpacks-", "glibc-langpack-"];

fn langpack_candidates(locale: &str) -> Vec<String> {
    let name = locale.split(['.', '@']).next().unwrap_or_default();
    let lang = name.split('_').next().unwrap_or_default();
    if lang.is_empty() || lang == "C" || lang == "POSIX" {
        return vec![];
    }
    let mut names = vec![name];
    if lang != name {
        names.push(lang);
    }
    let (glibc, per_name) = LANGPACK_PREFIXES.split_last().expect("langpack prefixes");
    let per_name =
        per_name.iter().flat_map(|prefix| names.iter().map(move |n| prefix.to_string() + n));
    per_name.chain([format!("{glibc}{lang}")]).collect()
}

async fn packages_in_repos(pkgs: &[String]) -> Res<Vec<String>> {
    let out = crate::root_output(crate::Verb::RepoQuery { pkgs: pkgs.to_vec() }).await?;
    Ok(out.lines().map(str::to_owned).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn langpacks() {
        assert_eq!(
            langpack_candidates("pt_BR.UTF-8"),
            [
                "ultramarine-langpacks-pt_BR",
                "ultramarine-langpacks-pt",
                "langpacks-pt_BR",
                "langpacks-pt",
                "glibc-langpack-pt"
            ]
        );
        assert_eq!(
            langpack_candidates("eo.UTF-8"),
            ["ultramarine-langpacks-eo", "langpacks-eo", "glibc-langpack-eo"]
        );
        assert!(langpack_candidates("C.UTF-8").is_empty());
        assert!(langpack_candidates("").is_empty());
    }
}
