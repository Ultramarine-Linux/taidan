use libtaidan::passwd::UsernameProblem;

const fn username_error_id(problem: Option<UsernameProblem>) -> &'static str {
    match problem {
        None => "",
        Some(UsernameProblem::Taken) => "page-whoareyou-error-taken",
        Some(UsernameProblem::TooLong) => "page-whoareyou-error-long",
        Some(UsernameProblem::Invalid) => "page-whoareyou-error",
    }
}

fn valid_fullname(s: &str) -> bool {
    !s.chars().any(libtaidan::passwd::gecos_separator)
}

fn spawn_username_checker(
    ui: slint::Weak<crate::ui::AppWindow>,
) -> std::sync::mpsc::Sender<String> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        while let Ok(oldest) = rx.recv() {
            let name = rx.try_iter().last().unwrap_or(oldest);
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

// Han reads differently in Chinese and Japanese, so romanizing it would be a guess
fn mask_han(s: &str) -> String {
    const UNUSABLE_IN_USERNAME: char = '?';
    let script = icu::properties::CodePointMapData::<icu::properties::props::Script>::new();
    s.chars()
        .map(|c| {
            if script.get(c) == icu::properties::props::Script::Han {
                UNUSABLE_IN_USERNAME
            } else {
                c
            }
        })
        .collect()
}

fn derive_username(fullname: &str) -> String {
    fn username_from_word(part: &str) -> Option<String> {
        (part.chars().all(|c: char| c.is_ascii_alphanumeric())
            && part.chars().next().is_some_and(|c| c.is_ascii_alphabetic()))
        .then(|| part.to_ascii_lowercase())
    }

    let fullname = deunicode::deunicode(&mask_han(fullname));
    let mut words = fullname.split_whitespace();
    words
        .next()
        .and_then(username_from_word)
        .or_else(|| words.next_back().and_then(username_from_word))
        .unwrap_or_default()
}

pub fn initialize_ui(ui: slint::Weak<crate::ui::AppWindow>, account: &crate::ui::Account<'_>) {
    use slint::ComponentHandle;
    account.on_username_syntax_error(|s| {
        username_error_id(libtaidan::passwd::username_syntax_problem(&s)).into()
    });
    let checker = spawn_username_checker(ui.clone());
    account.on_check_username(move |s| {
        if libtaidan::passwd::username_syntax_problem(&s).is_none() {
            _ = checker.send(s.into());
        }
    });
    account.on_valid_fullname(|s| valid_fullname(&s));
    account.on_long_password(|s| s.len() > libtaidan::passwd::PASSWORD_MAX_LEN);
    account.on_suggest_username(|fullname, _current| derive_username(&fullname).into());
    account.on_first_name(|fullname, username| {
        fullname.split_whitespace().next().map_or(username, Into::into)
    });
    account.on_save(move || {
        let Some(ui) = ui.upgrade() else { return };
        let account = ui.global::<crate::ui::Account<'_>>();
        let mut settings = crate::SETTINGS.write();
        settings.username = account.get_username().into();
        let fullname = libtaidan::passwd::sanitize_fullname(&account.get_fullname());
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

    #[test]
    fn usernames_derived() {
        for (fullname, username) in [
            ("", ""),
            ("!!!", ""),
            ("Alice Smith", "alice"),
            ("Dr. Smith", "smith"),
            ("José García", "jose"),
            ("Jörg Müller", "jorg"),
            ("Иван Петров", "ivan"),
            ("山田 太郎", ""),
            ("Jos 山田", "jos"),
            ("山田 Taro", "taro"),
        ] {
            assert_eq!(derive_username(fullname), username, "{fullname}");
        }
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
        assert!(libtaidan::passwd::username_problem(&me.to_string_lossy()).is_some());
    }
}
