//! user account details (full name, username, password)
use std::sync::LazyLock;

use regex::Regex;

static USERNAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[a-z][-a-z0-9_]*$").expect("invalid username regex"));

fn valid_username(s: &str) -> bool {
    USERNAME_RE.is_match(s)
}

/// suggest a username from the full name: the first word if usable, otherwise the last
///
/// returns [`None`] if neither word can be turned into a valid username
fn derive_username(fullname: &str) -> Option<String> {
    fn derive_part(part: &str) -> Option<String> {
        (part.chars().all(|c: char| c.is_ascii_alphanumeric())
            && part.chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
        .then(|| part.to_ascii_lowercase())
    }

    let fullname = fullname.trim();
    if fullname.is_empty() {
        return Some(String::new());
    }

    let first = fullname.split_once(char::is_whitespace).map_or(fullname, |(a, _)| a);
    derive_part(first).or_else(|| {
        let last = fullname.rsplit_once(char::is_whitespace).map_or(fullname, |(_, b)| b);
        derive_part(last)
    })
}

pub fn initialize_ui(ui: slint::Weak<crate::ui::AppWindow>, account: &crate::ui::Account<'_>) {
    use slint::ComponentHandle;
    account.on_valid_username(|s| valid_username(&s));
    account.on_suggest_username(|fullname, current| {
        derive_username(&fullname).map_or(current, Into::into)
    });
    account.on_first_name(|fullname, username| {
        fullname.split_whitespace().next().map_or(username, Into::into)
    });
    account.on_save(move || {
        let Some(ui) = ui.upgrade() else { return };
        let account = ui.global::<crate::ui::Account<'_>>();
        let mut settings = crate::SETTINGS.write();
        settings.username = account.get_username().into();
        let fullname = account.get_fullname();
        settings.fullname =
            if fullname.trim().is_empty() { settings.username.clone() } else { fullname.into() };
        settings.passwd = account.get_password().into();
        let avatar = account.get_avatar_id();
        settings.avatar = (!avatar.is_empty()).then(|| avatar.as_str().into());
    });
}

#[cfg(test)]
mod tests {
    use super::{derive_username, valid_username};

    #[test]
    fn empty_input_clears_username() {
        assert_eq!(derive_username(""), Some(String::new()));
    }

    #[test]
    fn valid_input_derives_lowercase_username() {
        assert_eq!(derive_username("Alice Smith"), Some(String::from("alice")));
    }

    #[test]
    fn falls_back_to_last_word() {
        assert_eq!(derive_username("Dr. Smith"), Some(String::from("smith")));
    }

    #[test]
    fn non_convertible_input_does_not_overwrite_username() {
        assert_eq!(derive_username("!!!"), None);
    }

    #[test]
    fn username_validation() {
        assert!(valid_username("alice"));
        assert!(valid_username("a-b_c1"));
        assert!(!valid_username(""));
        assert!(!valid_username("1alice"));
        assert!(!valid_username("Alice"));
    }
}
