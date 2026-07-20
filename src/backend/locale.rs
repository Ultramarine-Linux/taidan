use color_eyre::eyre::eyre;
use i18n_embed::unic_langid::LanguageIdentifier;

pub const UTF8_SUFFIX: &str = ".UTF-8";

#[must_use]
pub fn locale_base(input: &str) -> Option<&str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let base = trimmed.split_once('.').map_or(trimmed, |(base, _)| base);
    if base.is_empty() || !base.contains('_') {
        return None;
    }
    Some(base)
}

#[must_use]
pub fn normalize_system_locale(input: &str) -> Option<String> {
    Some(format!("{}{UTF8_SUFFIX}", locale_base(input)?))
}

#[must_use]
pub fn is_canonical_system_locale(input: &str) -> bool {
    input
        .strip_suffix(UTF8_SUFFIX)
        .is_some_and(|base| locale_base(base).is_some())
}

/// # Errors
/// Returns an error if the locale base cannot be parsed as a `LanguageIdentifier`.
pub fn locale_to_langid(input: &str) -> color_eyre::Result<LanguageIdentifier> {
    let base = locale_base(input).ok_or_else(|| eyre!("invalid locale `{input}`"))?;
    base.replace('_', "-")
        .parse()
        .map_err(|e| eyre!("cannot parse locale `{input}` as language identifier: {e}"))
}

/// # Errors
/// Returns an error if the locale is not already in canonical `.UTF-8` form.
pub fn require_canonical_system_locale(input: &str) -> color_eyre::Result<()> {
    if is_canonical_system_locale(input) {
        Ok(())
    } else {
        Err(eyre!("locale `{input}` is not in canonical UTF-8 form"))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        UTF8_SUFFIX, is_canonical_system_locale, locale_base, locale_to_langid,
        normalize_system_locale, require_canonical_system_locale,
    };

    #[test]
    fn locale_base_strips_encoding() {
        assert_eq!(locale_base("en_US.UTF-8"), Some("en_US"));
        assert_eq!(locale_base("ar_AE.iso88596"), Some("ar_AE"));
    }

    #[test]
    fn locale_base_rejects_invalid_values() {
        assert_eq!(locale_base(""), None);
        assert_eq!(locale_base("C"), None);
    }

    #[test]
    fn normalize_system_locale_uses_utf8_suffix() {
        assert_eq!(
            normalize_system_locale("en_US"),
            Some(format!("en_US{UTF8_SUFFIX}"))
        );
        assert_eq!(
            normalize_system_locale("en_US.utf8"),
            Some(format!("en_US{UTF8_SUFFIX}"))
        );
        assert_eq!(
            normalize_system_locale("ar_AE.iso88596"),
            Some(format!("ar_AE{UTF8_SUFFIX}"))
        );
    }

    #[test]
    fn canonical_system_locale_validation_matches_suffix() {
        assert!(is_canonical_system_locale("en_US.UTF-8"));
        assert!(!is_canonical_system_locale("en_US.utf8"));
        assert!(require_canonical_system_locale("en_US.UTF-8").is_ok());
        assert!(require_canonical_system_locale("en_US").is_err());
    }

    #[test]
    fn locale_to_langid_parses_base_locale() {
        let langid = locale_to_langid("pt_BR.UTF-8").unwrap();
        assert_eq!(langid.to_string(), "pt-BR");
    }
}
