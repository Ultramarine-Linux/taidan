//! Parse /etc/passwd

use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::prelude::*;

#[must_use]
pub fn user_already_exists() -> bool {
    if cfg!(debug_assertions) {
        tracing::info!("skipping user_already_exists() check since debug_assertions is on");
        return false;
    }
    let recorded = unused_recorded_user();
    // through NSS, since systemd-homed users aren't in /etc/passwd
    match std::process::Command::new("getent").arg("passwd").output() {
        Ok(out) if out.status.success() => {
            return regular_user_in(out.stdout.as_slice(), recorded.as_deref());
        }
        Ok(out) => tracing::error!(status = ?out.status, "getent passwd failed"),
        Err(e) => tracing::error!(?e, "cannot run getent passwd"),
    }
    let Ok(f) =
        File::open("/etc/passwd").inspect_err(|e| tracing::error!(?e, "cannot open /etc/passwd"))
    else {
        return false;
    };
    regular_user_in(BufReader::new(f), recorded.as_deref())
}

pub const REGULAR_UIDS: std::ops::RangeInclusive<u32> = 1000..=60000;

pub const HOMED_UIDS: std::ops::RangeInclusive<u32> = 60001..=60513;

#[must_use]
pub fn regular_uid(uid: u32) -> bool {
    REGULAR_UIDS.contains(&uid) || HOMED_UIDS.contains(&uid)
}

fn regular_user_in(reader: impl BufRead, recorded: Option<&str>) -> bool {
    reader.lines().any(|l| {
        l.is_ok_and(|l| {
            let Some([user, _, uid, _]) = l.splitn(4, ':').collect_array() else {
                tracing::error!("cannot parse a line in the passwd database");
                return false;
            };
            let Ok(uid) =
                uid.parse::<u32>().inspect_err(|e| tracing::error!(?e, ?uid, "cannot parse uid"))
            else {
                return false;
            };
            if !regular_uid(uid) {
                return false;
            }
            if recorded == Some(user) {
                tracing::info!(?user, ?uid, "found the user an unfinished install made");
                return false;
            }
            tracing::info!(?user, ?uid, "found normal user");
            true
        })
    })
}

pub const CREATED_RECORD: &str = "/var/lib/taidan/created-user";

#[derive(Debug, PartialEq, Eq)]
pub struct Record {
    pub user: String,
    pub keep_home: bool,
}

const KEEP_HOME_LINE: &str = "keep-home";

#[must_use]
pub fn record() -> Option<Record> {
    parse_record(&std::fs::read_to_string(CREATED_RECORD).ok()?)
}

fn parse_record(s: &str) -> Option<Record> {
    let mut lines = s.lines().map(str::trim);
    let user = lines.next().filter(|u| !u.is_empty())?.to_owned();
    Some(Record { user, keep_home: lines.any(|l| l == KEEP_HOME_LINE) })
}

#[must_use]
pub fn recorded_user() -> Option<String> {
    record().map(|r| r.user)
}

fn unused_recorded_user() -> Option<String> {
    recorded_user().filter(|u| match login_state(u) {
        Some(logged_in) => !logged_in,
        // a homed user can't be removed without its home
        None => uzers::get_user_by_name(u).is_none_or(|user| !HOMED_UIDS.contains(&user.uid())),
    })
}

#[must_use]
pub fn created_by_us(username: &str) -> bool {
    unused_recorded_user().is_some_and(|u| u == username)
}

#[must_use]
pub fn login_state(username: &str) -> Option<bool> {
    // PAM updates lastlog2 for display manager, console and ssh logins, but not `runuser` or
    // `pkexec`. `-a` leaves out users who never logged in
    match std::process::Command::new("lastlog2").args(["-a", "-u", username]).output() {
        Ok(out) if out.status.success() => Some(logged_in_from(&out.stdout, username)),
        Ok(out) => {
            tracing::error!(status = ?out.status, stderr = %String::from_utf8_lossy(&out.stderr), "lastlog2 failed");
            None
        }
        Err(e) => {
            tracing::error!(?e, "cannot run lastlog2");
            None
        }
    }
}

fn logged_in_from(output: &[u8], username: &str) -> bool {
    String::from_utf8_lossy(output).lines().any(|l| l.split_whitespace().next() == Some(username))
}

pub async fn record_created(username: &str, keep_home: bool) -> Res<()> {
    crate::root(crate::Verb::RecordCreated { user: username.to_owned(), keep_home })
        .await
        .wrap_msg("cannot record the created user")
}

pub(crate) fn write_record(username: &str, keep_home: bool) -> std::io::Result<()> {
    let mut content = format!("{username}\n");
    if keep_home {
        content.push_str(KEEP_HOME_LINE);
        content.push('\n');
    }
    crate::helper::write_atomic(std::path::Path::new(CREATED_RECORD), content.as_bytes(), 0o644)
}

pub async fn clear_record() -> Res<()> {
    let failure = "cannot remove the created user record";
    crate::root_if_any_exists(&[CREATED_RECORD], crate::Verb::ClearRecord, failure).await
}

#[must_use]
pub fn removable(username: &str, uid: u32) -> bool {
    regular_uid(uid) && username_syntax_problem(username).is_none()
}

// `homectl create` needs it to fit utmp's `ut_user`
const USERNAME_MAX_LEN: usize = 31;

// libxcrypt refuses `CRYPT_MAX_PASSPHRASE_SIZE` (512) and up
pub const PASSWORD_MAX_LEN: usize = 511;

const RESERVED_USERNAMES: &[&str] = &["root", "taidan", "nobody"];

static USERNAME_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new("^[a-z][-a-z0-9_]*$").expect("invalid username regex"));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsernameProblem {
    Invalid,
    TooLong,
    Taken,
}

#[must_use]
pub fn username_syntax_problem(username: &str) -> Option<UsernameProblem> {
    username_problem_with(username, |_| false, None)
}

#[must_use]
pub fn username_problem(username: &str) -> Option<UsernameProblem> {
    username_problem_with(username, user_or_group_exists, unused_recorded_user().as_deref())
}

fn user_or_group_exists(name: &str) -> bool {
    uzers::get_user_by_name(name).is_some() || uzers::get_group_by_name(name).is_some()
}

fn username_problem_with(
    username: &str,
    exists: impl Fn(&str) -> bool,
    recorded: Option<&str>,
) -> Option<UsernameProblem> {
    if !USERNAME_RE.is_match(username) {
        return Some(UsernameProblem::Invalid);
    }
    if username.len() > USERNAME_MAX_LEN {
        return Some(UsernameProblem::TooLong);
    }
    if RESERVED_USERNAMES.contains(&username)
        || username.starts_with("systemd-")
        || (exists(username) && recorded != Some(username))
    {
        return Some(UsernameProblem::Taken);
    }
    None
}

#[must_use]
// KDE and AccountsService also cut the name at `,`
pub fn gecos_separator(c: char) -> bool {
    matches!(c, ':' | ',') || c.is_control()
}

#[must_use]
pub fn sanitize_fullname(fullname: &str) -> String {
    fullname.replace(gecos_separator, " ").split_whitespace().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nobody_exists(_: &str) -> bool {
        false
    }

    #[test]
    fn username_format() {
        assert_eq!(username_problem_with("alice", nobody_exists, None), None);
        assert_eq!(username_problem_with("a-b_c1", nobody_exists, None), None);
        for bad in ["", "1alice", "Alice", "al ice", "alice;rm", "-alice"] {
            assert_eq!(
                username_problem_with(bad, nobody_exists, None),
                Some(UsernameProblem::Invalid)
            );
        }
        assert_eq!(username_problem_with(&"a".repeat(31), nobody_exists, None), None);
        assert_eq!(
            username_problem_with(&"a".repeat(32), nobody_exists, None),
            Some(UsernameProblem::TooLong)
        );
    }

    #[test]
    fn username_taken() {
        for taken in ["root", "taidan", "nobody", "systemd-network"] {
            assert_eq!(
                username_problem_with(taken, nobody_exists, None),
                Some(UsernameProblem::Taken)
            );
        }
        let exists = |name: &str| name == "bob";
        assert_eq!(username_problem_with("bob", exists, None), Some(UsernameProblem::Taken));
        assert_eq!(username_problem_with("alice", exists, None), None);
    }

    #[test]
    fn username_recorded_is_fine() {
        let exists = |name: &str| name == "carol";
        assert_eq!(username_problem_with("carol", exists, None), Some(UsernameProblem::Taken));
        assert_eq!(
            username_problem_with("carol", exists, Some("dave")),
            Some(UsernameProblem::Taken)
        );
        assert_eq!(username_problem_with("carol", exists, Some("carol")), None);
        assert_eq!(
            username_problem_with("root", |_| true, Some("root")),
            Some(UsernameProblem::Taken)
        );
    }

    #[test]
    fn only_regular_users_are_removable() {
        assert!(removable("alice", 1000));
        assert!(removable("alice", 59999));
        assert!(!removable("alice", 0));
        assert!(!removable("alice", 999));
        assert!(!removable("alice", 65534));
        assert!(removable("alice", 60000));
        assert!(removable("alice", 60001));
        assert!(removable("alice", 60513));
        assert!(!removable("alice", 60514));
        let dynamic_user_uid = 61876;
        assert!(!removable("earlyoom", dynamic_user_uid));
        assert!(!removable("root", 1000));
        assert!(!removable("taidan", 1001));
        assert!(!removable("systemd-network", 1001));
        assert!(!removable("Alice", 1000));
    }

    #[test]
    fn recorded_user_isnt_an_existing_one() {
        let passwd = "root:x:0:0::/root:/bin/bash\ntaidan:x:980:980::/usr/lib/taidan:/bin/sh\n";
        assert!(!regular_user_in(passwd.as_bytes(), None));
        let passwd = format!("{passwd}alice:x:1000:1000:Alice:/home/alice:/bin/bash\n");
        assert!(regular_user_in(passwd.as_bytes(), None));
        assert!(!regular_user_in(passwd.as_bytes(), Some("alice")));
        assert!(regular_user_in(passwd.as_bytes(), Some("bob")));
        let passwd = format!("{passwd}bob:x:1001:1001::/home/bob:/bin/bash\n");
        assert!(regular_user_in(passwd.as_bytes(), Some("alice")));
    }

    #[test]
    fn homed_user_is_an_existing_one() {
        let passwd = "root:x:0:0::/root:/bin/bash\nearlyoom:x:61876:61876::/:/usr/sbin/nologin\n";
        assert!(!regular_user_in(passwd.as_bytes(), None));
        let passwd = format!("{passwd}alice:x:60123:60123:Alice:/home/alice:/bin/bash\n");
        assert!(regular_user_in(passwd.as_bytes(), None));
        assert!(!regular_user_in(passwd.as_bytes(), Some("alice")));
    }

    #[test]
    fn lastlog2_output() {
        let header =
            "Username         Port     From                                       Latest \n";
        assert!(!logged_in_from(b"", "alice"));
        assert!(!logged_in_from(header.as_bytes(), "alice"));
        let used = format!(
            "{header}alice            tty1                    Tue Nov  3 13:38:37 -0700 2026\n"
        );
        assert!(logged_in_from(used.as_bytes(), "alice"));
        assert!(!logged_in_from(used.as_bytes(), "ali"));
        assert!(!logged_in_from(used.as_bytes(), "bob"));
    }

    #[test]
    fn record_parsed() {
        assert_eq!(parse_record(""), None);
        assert_eq!(parse_record("\n"), None);
        assert_eq!(
            parse_record("alice\n"),
            Some(Record { user: "alice".into(), keep_home: false })
        );
        assert_eq!(
            parse_record("alice\nkeep-home\n"),
            Some(Record { user: "alice".into(), keep_home: true })
        );
    }

    #[test]
    fn fullname_sanitized() {
        assert_eq!(sanitize_fullname("Alice Smith"), "Alice Smith");
        assert_eq!(sanitize_fullname("Alice:Smith\n"), "Alice Smith");
        assert_eq!(sanitize_fullname("a\nb\tc"), "a b c");
        assert_eq!(sanitize_fullname("John Smith, Jr."), "John Smith Jr.");
        assert_eq!(sanitize_fullname("Smith,John"), "Smith John");
    }
}
