//! Parse /etc/passwd

use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::prelude::*;

/// whether there's a regular user, apart from one an unfinished install made that nobody has
/// logged in as (see [`CREATED_RECORD`]), which the next install removes
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

/// uids `useradd` gives regular users, `UID_MIN` to `UID_MAX` in Fedora's `/etc/login.defs`
pub const REGULAR_UIDS: std::ops::RangeInclusive<u32> = 1000..=60000;

/// uids systemd-homed gives its users
pub const HOMED_UIDS: std::ops::RangeInclusive<u32> = 60001..=60513;

/// whether `uid` belongs to a regular user, made by `useradd` or systemd-homed
#[must_use]
pub fn regular_uid(uid: u32) -> bool {
    REGULAR_UIDS.contains(&uid) || HOMED_UIDS.contains(&uid)
}

/// [`user_already_exists`] for the passwd database `reader`, with `recorded` the user from
/// [`CREATED_RECORD`]
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

/// the user an unfinished install created, so the next attempt (a retry, or after a reboot) can
/// remove it and start over instead of failing on it or leaving it behind. removed when the
/// install finishes, so a later Taidan run never touches the user.
pub const CREATED_RECORD: &str = "/var/lib/taidan/created-user";

/// what [`CREATED_RECORD`] says about the user an unfinished install made
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub user: String,
    /// the user's home was there before the user was made (a reinstall that kept `/home`), so
    /// removing the user mustn't take it along
    pub keep_home: bool,
}

/// the line after the username in [`CREATED_RECORD`] that sets [`Record::keep_home`]
const KEEP_HOME: &str = "keep-home";

/// what [`CREATED_RECORD`] says, if there's one
#[must_use]
pub fn record() -> Option<Record> {
    parse_record(&std::fs::read_to_string(CREATED_RECORD).ok()?)
}

/// [`record`] from the contents of [`CREATED_RECORD`]
fn parse_record(s: &str) -> Option<Record> {
    let mut lines = s.lines().map(str::trim);
    let user = lines.next().filter(|u| !u.is_empty())?.to_owned();
    Some(Record { user, keep_home: lines.any(|l| l == KEEP_HOME) })
}

/// the user named in [`CREATED_RECORD`], if there's one
#[must_use]
pub fn recorded_user() -> Option<String> {
    record().map(|r| r.user)
}

/// the user named in [`CREATED_RECORD`], if there's one and nobody has logged in as it. A user
/// someone has logged in as is their account now, whatever the record says. A user whose logins
/// can't be checked counts as unused, since the record says it's from an install that never
/// finished. treating it as a real user would mark the system configured with an account that was
/// never set up, and [`login_state`] already says to keep its home when it's removed.
#[must_use]
pub fn unused_recorded_user() -> Option<String> {
    recorded_user().filter(|u| match login_state(u) {
        Some(logged_in) => !logged_in,
        // except a homed user, which can't be removed without its home, so it stays (see
        // `remove_previous`)
        None => uzers::get_user_by_name(u).is_none_or(|user| !HOMED_UIDS.contains(&user.uid())),
    })
}

/// whether an unfinished install created `username` and nobody has logged in as it since, see
/// [`CREATED_RECORD`]
#[must_use]
pub fn created_by_us(username: &str) -> bool {
    unused_recorded_user().is_some_and(|u| u == username)
}

/// whether `username` has ever logged in, going by lastlog2 (PAM updates it for display manager,
/// console and ssh logins, but not `runuser` or `pkexec`). [`None`] if that can't be checked, and
/// then removing the user mustn't take its home along, in case it was used after all.
#[must_use]
pub fn login_state(username: &str) -> Option<bool> {
    // `-a` leaves out users who never logged in. A missing database prints nothing
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

/// whether `lastlog2 -a -u username` printed a record for `username`, past its header
fn logged_in_from(output: &[u8], username: &str) -> bool {
    String::from_utf8_lossy(output).lines().any(|l| l.split_whitespace().next() == Some(username))
}

/// writes `username` to [`CREATED_RECORD`], world-readable since the OOBE checks it as its own
/// user. pass `keep_home` if its home already exists (see [`Record::keep_home`]).
///
/// # Errors
/// cannot write the file
pub async fn record_created(username: &str, keep_home: bool) -> Res<()> {
    crate::root(crate::Verb::RecordCreated { user: username.to_owned(), keep_home })
        .await
        .wrap_msg("cannot record the created user")
}

/// what [`record_created`] does, run by the helper as root. the file is replaced whole so a full
/// disk can't leave it empty, and is on disk before the user it records is created so a power cut
/// can't lose it.
pub(crate) fn write_record(username: &str, keep_home: bool) -> std::io::Result<()> {
    let mut content = format!("{username}\n");
    if keep_home {
        content.push_str(KEEP_HOME);
        content.push('\n');
    }
    crate::helper::write_atomic(std::path::Path::new(CREATED_RECORD), content.as_bytes(), 0o644)
}

/// removes [`CREATED_RECORD`], if it's there
///
/// # Errors
/// cannot remove the file
pub async fn clear_record() -> Res<()> {
    if !tokio::fs::try_exists(CREATED_RECORD).await.unwrap_or(true) {
        return Ok(());
    }
    crate::root(crate::Verb::ClearRecord).await.wrap_msg("cannot remove the created user record")
}

/// whether `username` with `uid` is safe for Taidan to remove: a regular user with a name the OOBE
/// would accept
#[must_use]
pub fn removable(username: &str, uid: u32) -> bool {
    regular_uid(uid) && username_syntax_problem(username).is_none()
}

/// the longest name `homectl create` takes (it must fit utmp's `ut_user`), one less than `useradd`
pub const USERNAME_MAX_LEN: usize = 31;

/// the longest password libxcrypt hashes, in bytes. `crypt` refuses `CRYPT_MAX_PASSPHRASE_SIZE`
/// (512) and up.
pub const PASSWORD_MAX_LEN: usize = 511;

/// names that are taken even if the system doesn't have them right now
const RESERVED_USERNAMES: &[&str] = &["root", "taidan", "nobody"];

static USERNAME_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new("^[a-z][-a-z0-9_]*$").expect("invalid username regex"));

/// why a username can't be used
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsernameProblem {
    /// not lowercase letters, digits, `_` and `-`, starting with a letter
    Invalid,
    /// longer than [`USERNAME_MAX_LEN`]
    TooLong,
    /// a system account, or a user or group that already exists
    Taken,
}

/// [`username_problem`] without looking up users and groups, so it's cheap enough to run on
/// every keystroke. only reserved names come out as [`UsernameProblem::Taken`].
#[must_use]
pub fn username_syntax_problem(username: &str) -> Option<UsernameProblem> {
    username_problem_with(username, |_| false, None)
}

/// what's wrong with `username` for the new user, if anything. this looks up NSS and may run
/// `lastlog2`, so keep it off the UI thread.
#[must_use]
pub fn username_problem(username: &str) -> Option<UsernameProblem> {
    username_problem_with(
        username,
        |name| uzers::get_user_by_name(name).is_some() || uzers::get_group_by_name(name).is_some(),
        unused_recorded_user().as_deref(),
    )
}

/// [`username_problem`] with `exists` checking whether a user or group has the name, and
/// `recorded` the user from [`CREATED_RECORD`]
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
    // useradd also makes a group with the user's name, so that can't be taken either
    if RESERVED_USERNAMES.contains(&username)
        || username.starts_with("systemd-")
        || (exists(username) && recorded != Some(username))
    {
        return Some(UsernameProblem::Taken);
    }
    None
}

/// whether `c` can't go in a GECOS field as is: `:` separates passwd fields, `,` separates the
/// GECOS subfields (KDE and AccountsService cut the name at it) and control characters (newlines)
/// would break the line
#[must_use]
pub fn gecos_separator(c: char) -> bool {
    matches!(c, ':' | ',') || c.is_control()
}

/// `fullname` as a GECOS field, with each [`gecos_separator`] turned into a space and runs of
/// whitespace squeezed into one
#[must_use]
pub fn sanitize_fullname(fullname: &str) -> String {
    fullname
        .chars()
        .map(|c| if gecos_separator(c) { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
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
        // a record can't make a reserved name usable
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
        // DynamicUser= services
        assert!(!removable("earlyoom", 61876));
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
