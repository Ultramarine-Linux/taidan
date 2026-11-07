use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemSetup;
impl super::Step for SystemSetup {
    /// adds the language pack for the user's language to the apps to install
    #[tracing::instrument(skip(settings))]
    async fn pre<C: crate::Callback>(
        &self,
        settings: &mut crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        if settings.nointernet || settings.locale.is_empty() || !super::can_use_dnf(cfg).await {
            return Ok(());
        }
        let candidates = langpack_candidates(&settings.locale);
        match available(&candidates).await {
            Ok(found) => {
                // the first one that exists, the most specific of the distro's own
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

    #[tracing::instrument(skip(settings))]
    async fn run<C: crate::Callback>(
        &self,
        settings: &crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        if let Some(ids) = settings.tweaks.ids() {
            let sett = serde_json::to_vec(settings)?;
            // the ones an earlier attempt turned on, to turn off again if they aren't anymore.
            // Recorded as it goes, so an attempt cut short still says what it turned on
            let mut recorded = crate::tweaks::recorded_on();
            let record = |ids: &[String]| {
                let ids = ids.to_vec();
                async move {
                    if let Err(e) = crate::tweaks::record_on(&ids).await {
                        tracing::warn!(?e, "cannot record the tweaks turned on");
                    }
                }
            };
            for tweak in &*crate::tweaks::TWEAKS {
                let id = tweak.id().to_string_lossy().into_owned();
                let started = format!("{}{id}", crate::tweaks::STARTED);
                let on = ids.contains(&id);
                // one that may be only partly applied runs again whichever way it's wanted
                if !recorded.contains(&started) && on == recorded.contains(&id) {
                    continue;
                }
                if on && settings.nointernet && tweak.needs_internet {
                    tracing::info!(id, "skipping tweak that needs the internet");
                    // one an earlier attempt only got partway through turning on is turned off
                    // again instead, nothing would after this install
                    if recorded.contains(&started) && tweak.run(&sett, false).await {
                        recorded.retain(|r| *r != started);
                        record(&recorded).await;
                    }
                    continue;
                }
                recorded.retain(|r| *r != id && *r != started);
                // before turning it on, so a retry still knows to turn it off
                if on {
                    recorded.push(started.clone());
                    record(&recorded).await;
                    recorded.pop();
                }
                let ok = tweak.run(&sett, on).await;
                if !ok {
                    recorded.push(started);
                } else if on {
                    recorded.push(id);
                }
                record(&recorded).await;
            }
        } else {
            tracing::warn!(
                "the settings don't say which tweaks to turn on, leaving them as they are"
            );
        }
        // GeoClue is D-Bus activated, so only a mask keeps apps from starting it. Unmasked when
        // it's on, for a retry after an attempt with it off. Left alone when the page was skipped,
        // the distro may have masked it on purpose
        if let Some(on) = settings.location {
            let (verb, what) = if on { ("unmask", "on") } else { ("mask", "off") };
            if let Err(e) = crate::root("systemctl", [verb, "geoclue.service"]).await {
                tracing::warn!(?e, "cannot turn {what} location services");
            }
        }
        if let Err(e) = crate::i18n::apply_keymap_to_user(settings, cfg).await {
            tracing::warn!(?e, "cannot carry over the keyboard layout");
        }
        if let Err(e) = crate::a11y::apply_to_user(settings, cfg).await {
            tracing::warn!(?e, "cannot carry over the accessibility settings");
        }
        // for cameras and brightness controls on systems where these aren't given to the seat
        let has_video = tokio::fs::read_to_string("/etc/group")
            .await
            .is_ok_and(|groups| groups.lines().any(|l| l.starts_with("video:")));
        // homed users get it when they're created
        if has_video
            && !cfg.taidan0.systemd_homed
            && let Err(e) = crate::root("usermod", ["-aG", "video", &settings.username]).await
        {
            tracing::warn!(?e, "cannot add the user to the video group");
        }
        Ok(())
    }
}

/// language pack packages for a POSIX locale like `pt_BR.UTF-8`, best first: Ultramarine's, which
/// pull in Fedora's, then Fedora's, each for the language in that country and then on its own
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
    ["ultramarine-langpacks-", "langpacks-"]
        .iter()
        .flat_map(|prefix| names.iter().map(move |n| format!("{prefix}{n}")))
        .collect()
}

/// which of `pkgs` the repositories have
async fn available(pkgs: &[String]) -> Res<Vec<String>> {
    // as root, to use the system's metadata cache rather than downloading one for this user
    let out = tokio::process::Command::new("pkexec")
        .args(["--user", "root", "env"])
        .args(crate::proxy::wrapper())
        .args(["dnf5", "-q", "repoquery", "--latest-limit", "1", "--qf", "%{name}\\n"])
        .args(pkgs)
        .output()
        .await
        .map_err(|e| Err::FailToRunProgram("dnf5", e))?;
    if !out.status.success() {
        return Err(Err::ProgramFail {
            name: "dnf5",
            output: String::from_utf8_lossy(&out.stderr).into_owned(),
            rc: out.status.code(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).lines().map(str::to_owned).collect())
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
                "langpacks-pt"
            ]
        );
        assert_eq!(langpack_candidates("eo.UTF-8"), ["ultramarine-langpacks-eo", "langpacks-eo"]);
        assert!(langpack_candidates("C.UTF-8").is_empty());
        assert!(langpack_candidates("").is_empty());
    }
}
