//! user account details (full name, username, password)

use libtaidan::passwd::UsernameProblem;

/// fluent id of the message for `problem`
const fn username_error(problem: Option<UsernameProblem>) -> &'static str {
    match problem {
        None => "",
        Some(UsernameProblem::Taken) => "page-whoareyou-error-taken",
        Some(UsernameProblem::TooLong) => "page-whoareyou-error-long",
        Some(UsernameProblem::Invalid) => "page-whoareyou-error",
    }
}

/// whether the full name has none of the characters that separate GECOS fields.
/// the install would replace them, but then the login screen wouldn't show what the user typed
fn valid_fullname(s: &str) -> bool {
    !s.chars().any(libtaidan::passwd::gecos_separator)
}

/// starts the thread that looks up whether usernames are taken, which goes through NSS and maybe
/// `lastlog2`, too slow for the UI thread. it only checks the latest username sent and drops a
/// result once the username has changed again
fn spawn_username_checker(
    ui: slint::Weak<crate::ui::AppWindow>,
) -> std::sync::mpsc::Sender<String> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        while let Ok(mut name) = rx.recv() {
            // skip the ones typed while the last check ran
            while let Ok(newer) = rx.try_recv() {
                name = newer;
            }
            let taken = libtaidan::passwd::username_problem(&name).is_some();
            let posted = ui.upgrade_in_event_loop(move |ui| {
                use slint::ComponentHandle;
                let account = ui.global::<crate::ui::Account<'_>>();
                if account.get_username() == name.as_str() {
                    account.set_checked_username(name.into());
                    account.set_checked_taken(taken);
                }
            });
            if posted.is_err() {
                break;
            }
        }
    });
    tx
}

/// suggest a username from the full name: the first word if usable, otherwise the last, romanized
/// first so e.g. `José` gives `jose` and `Иван` gives `ivan`
///
/// returns an empty string if neither word can be turned into a valid username, so the field
/// doesn't keep a suggestion from what was typed before
fn derive_username(fullname: &str) -> String {
    fn derive_part(part: &str) -> Option<String> {
        (part.chars().all(|c: char| c.is_ascii_alphanumeric())
            && part.chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
        .then(|| part.to_ascii_lowercase())
    }

    // Han characters read differently in Chinese and Japanese, so a guess would likely be wrong
    // for one of them. mapping them to `?` makes their word unusable
    let script = icu::properties::CodePointMapData::<icu::properties::props::Script>::new();
    let fullname = fullname
        .chars()
        .map(|c| if script.get(c) == icu::properties::props::Script::Han { '?' } else { c })
        .collect::<String>();
    let fullname = deunicode::deunicode(&fullname);
    let fullname = fullname.trim();
    if fullname.is_empty() {
        return String::new();
    }

    let first = fullname.split_once(char::is_whitespace).map_or(fullname, |(a, _)| a);
    derive_part(first)
        .or_else(|| {
            let last = fullname.rsplit_once(char::is_whitespace).map_or(fullname, |(_, b)| b);
            derive_part(last)
        })
        .unwrap_or_default()
}

pub fn initialize_ui(ui: slint::Weak<crate::ui::AppWindow>, account: &crate::ui::Account<'_>) {
    use slint::ComponentHandle;
    account.on_username_syntax_error(|s| {
        username_error(libtaidan::passwd::username_syntax_problem(&s)).into()
    });
    let checker = spawn_username_checker(ui.clone());
    account.on_check_username(move |s| {
        // a name that's wrong anyway needs no lookup, `username-error` already says why
        if libtaidan::passwd::username_syntax_problem(&s).is_none() {
            _ = checker.send(s.into());
        }
    });
    account.on_valid_fullname(|s| valid_fullname(&s));
    account.on_long_password(|s| s.len() > libtaidan::passwd::PASSWORD_MAX_LEN);
    // only asked while the username is the suggestion, so one that can't be derived clears it
    account.on_suggest_username(|fullname, _current| derive_username(&fullname).into());
    account.on_first_name(|fullname, username| {
        fullname.split_whitespace().next().map_or(username, Into::into)
    });
    account.on_save(move || {
        let Some(ui) = ui.upgrade() else { return };
        let account = ui.global::<crate::ui::Account<'_>>();
        let mut settings = crate::SETTINGS.write();
        settings.username = account.get_username().into();
        // `:`, `,` and newlines would break the passwd entry
        let fullname = libtaidan::passwd::sanitize_fullname(&account.get_fullname());
        // so the summary shows it as saved, not e.g. a blank line for one that's only spaces
        if fullname != account.get_fullname().as_str() {
            account.set_fullname(fullname.as_str().into());
        }
        settings.fullname = if fullname.is_empty() { settings.username.clone() } else { fullname };
        settings.passwd = account.get_password().as_str().into();
        let avatar = account.get_avatar_id();
        settings.avatar = (!avatar.is_empty()).then(|| avatar.as_str().into());
    });
}

#[cfg(test)]
mod tests {
    use super::{derive_username, valid_fullname};

    fn valid_username(s: &str) -> bool {
        libtaidan::passwd::username_problem(s).is_none()
    }

    #[test]
    fn empty_input_clears_username() {
        assert_eq!(derive_username(""), "");
    }

    #[test]
    fn valid_input_derives_lowercase_username() {
        assert_eq!(derive_username("Alice Smith"), "alice");
    }

    #[test]
    fn falls_back_to_last_word() {
        assert_eq!(derive_username("Dr. Smith"), "smith");
    }

    #[test]
    fn non_convertible_input_clears_username() {
        assert_eq!(derive_username("!!!"), "");
    }

    #[test]
    fn non_ascii_names_are_romanized() {
        assert_eq!(derive_username("José García"), "jose");
        assert_eq!(derive_username("Jörg Müller"), "jorg");
        assert_eq!(derive_username("Иван Петров"), "ivan");
        // no guessing how to read them, and nothing left from a name typed before
        assert_eq!(derive_username("山田 太郎"), "");
        assert_eq!(derive_username("Jos 山田"), "jos");
        assert_eq!(derive_username("山田 Taro"), "taro");
    }

    #[test]
    fn username_validation() {
        assert!(valid_username("alice"));
        assert!(valid_username("a-b_c1"));
        assert!(!valid_username(""));
        assert!(!valid_username("1alice"));
        assert!(!valid_username("Alice"));
        assert!(valid_username(&"a".repeat(31)));
        assert!(!valid_username(&"a".repeat(32)));
        assert!(!valid_username("root"));
        assert!(!valid_username("taidan"));
    }

    #[test]
    fn fullname_validation() {
        assert!(valid_fullname("John Smith Jr."));
        assert!(valid_fullname("Zoë 山田"));
        assert!(!valid_fullname("John Smith, Jr."));
        assert!(!valid_fullname("a:b"));
        assert!(!valid_fullname("a\nb"));
    }

    #[test]
    fn existing_user_is_taken() {
        let me = uzers::get_current_username().expect("no current user");
        assert!(!valid_username(&me.to_string_lossy()));
    }
}
