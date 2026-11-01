use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemSetup;
impl super::Step for SystemSetup {
    /// adds the language pack for the user's language to the apps to install
    #[tracing::instrument]
    async fn pre<C: crate::Callback>(
        &self,
        settings: &mut crate::settings::Settings,
        cfg: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        if settings.nointernet || cfg.taidan0.skip_dnf || settings.locale.is_empty() {
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
        _: &crate::cfg::Config,
        _: &C,
    ) -> Res<()> {
        let sett = serde_json::to_vec(settings)?;
        for tweak in &*crate::tweaks::TWEAKS {
            let on = settings.tweaks.iter().any(|id| tweak.id() == id.as_str());
            tweak.run(&sett, on).await;
        }
        // services as the distro's presets want them, the image may have been built otherwise
        if let Err(e) = crate::root("systemctl", ["preset-all"]).await {
            tracing::warn!(?e, "cannot apply systemd presets");
        }
        // after the presets, which could bring it back. GeoClue is D-Bus activated, so only a
        // mask keeps apps from starting it
        if !settings.location
            && let Err(e) = crate::root("systemctl", ["mask", "geoclue.service"]).await
        {
            tracing::warn!(?e, "cannot turn off location services");
        }
        // for cameras and brightness controls on systems where these aren't given to the seat
        let has_video = tokio::fs::read_to_string("/etc/group")
            .await
            .is_ok_and(|groups| groups.lines().any(|l| l.starts_with("video:")));
        if has_video
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
    // as root, to use the metadata the system update just downloaded
    let out = tokio::process::Command::new("pkexec")
        .args(["--user", "root", "env"])
        .args(crate::proxy::env())
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
