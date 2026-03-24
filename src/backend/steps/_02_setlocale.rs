use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct SetLocale;

impl super::Step for SetLocale {
    #[tracing::instrument]
    async fn run(
        &self,
        settings: &crate::backend::settings::Settings,
        _: relm4::Sender<crate::pages::InstallingPageMsg>,
    ) -> color_eyre::Result<()> {
        set_system_locale(settings.langlocale).await
    }
}

fn localectl_args(locale: &str) -> color_eyre::Result<[String; 2]> {
    crate::backend::locale::require_canonical_system_locale(locale)?;
    Ok(["set-locale".to_owned(), format!("LANG={locale}")])
}

async fn set_system_locale(locale: &str) -> color_eyre::Result<()> {
    let args = localectl_args(locale)?;
    let arg_refs = args.iter().map(String::as_str).collect_vec();
    super::root("localectl", &arg_refs).await
}

#[cfg(test)]
mod tests {
    use super::localectl_args;

    #[test]
    fn localectl_args_use_canonical_utf8_locale() {
        assert_eq!(
            localectl_args("en_US.UTF-8").unwrap(),
            ["set-locale".to_owned(), "LANG=en_US.UTF-8".to_owned()]
        );
    }

    #[test]
    fn localectl_args_reject_non_canonical_locale() {
        assert!(localectl_args("en_US").is_err());
        assert!(localectl_args("en_US.utf8").is_err());
    }
}
