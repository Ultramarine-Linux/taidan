use crate::{kconfig::KConfigEntry, prelude::*};
use secrecy::{ExposeSecret as _, SecretString};
use std::path::{Path, PathBuf};

pub enum Home {
    Skel(PathBuf),
    User { name: String, home: PathBuf, homed: Option<SecretString> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomeCmd {
    argv: Vec<String>,
    input: Option<String>,
    best_effort: bool,
}

impl HomeCmd {
    pub fn new(argv: impl IntoIterator<Item: AsRef<str>>) -> Self {
        let argv = argv.into_iter().map(|a| a.as_ref().to_owned()).collect();
        Self { argv, input: None, best_effort: false }
    }

    #[must_use]
    pub fn best_effort(self) -> Self {
        Self { best_effort: true, ..self }
    }
}

const WRITE_STDIN_TO_FILE: &str = r#"mkdir -p "${1%/*}" && exec cat > "$1""#;

impl Home {
    pub async fn user(settings: &crate::settings::Settings, cfg: &crate::cfg::Config) -> Res<Self> {
        Ok(Self::User {
            name: settings.username.clone(),
            home: home_of(&settings.username).await?,
            homed: cfg.taidan0.systemd_homed.then(|| settings.passwd.clone()),
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Skel(home) | Self::User { home, .. } => home,
        }
    }

    #[must_use]
    pub fn config(&self) -> PathBuf {
        self.path().join(".config")
    }

    #[must_use]
    pub fn session(&self, cmds: Vec<Vec<String>>) -> Option<HomeCmd> {
        let cmd = one_session(cmds)?;
        let home = self.path();
        let env = [
            "env".to_owned(),
            "-u".to_owned(),
            "DBUS_SESSION_BUS_ADDRESS".to_owned(),
            format!("HOME={}", home.display()),
            format!("XDG_CONFIG_HOME={}", self.config().display()),
        ];
        Some(HomeCmd::new(env.into_iter().chain(cmd)))
    }

    pub fn kconfig(&self, entries: &[KConfigEntry]) -> impl Iterator<Item = HomeCmd> {
        let config = self.config();
        entries.iter().map(move |entry| HomeCmd::new(entry.command(Some(&config))))
    }

    #[must_use]
    pub fn write_file(&self, relative: &str, content: impl Into<String>) -> HomeCmd {
        let path = self.path().join(relative).to_string_lossy().into_owned();
        let cmd = HomeCmd::new(["sh", "-c", WRITE_STDIN_TO_FILE, "sh", &path]);
        HomeCmd { input: Some(content.into()), ..cmd }
    }

    #[must_use]
    pub fn remove_file(&self, relative: &str) -> HomeCmd {
        HomeCmd::new(["rm", "-f", &self.path().join(relative).to_string_lossy()])
    }

    pub async fn run(&self, cmds: Vec<HomeCmd>) -> Res<()> {
        if cmds.is_empty() {
            return Ok(());
        }
        match self {
            Self::User { name, homed: Some(password), .. } => {
                let argv = ["sh", "-c", &homed_script(&cmds)].map(str::to_owned).to_vec();
                let password_line = format!("{}\n", password.expose_secret());
                let verb = crate::Verb::AsUserHomed { user: name.clone(), argv };
                return crate::root_with_stdin(verb, password_line.as_bytes())
                    .await
                    .wrap_msg("cannot write into the systemd-homed home");
            }
            Self::Skel(skel) => {
                tokio::fs::create_dir_all(skel.join(".config")).await.map_err(Err::Io)?;
            }
            Self::User { .. } => {}
        }
        let mut first_failure = Ok(());
        for cmd in cmds {
            let input = cmd.input.as_deref().map(str::as_bytes);
            let res = match self {
                Self::User { name, .. } => crate::as_user(name, &cmd.argv, input).await,
                Self::Skel(_) => crate::run_here(&cmd.argv, input).await,
            };
            match res {
                Err(e) if cmd.best_effort => {
                    tracing::warn!(?e, ?cmd.argv, "cannot write into the home");
                }
                Err(e) if first_failure.is_ok() => first_failure = Err(e),
                _ => {}
            }
        }
        first_failure
    }
}

pub(crate) fn gsettings(schema: &str, key: &str, value: Option<&str>) -> Vec<String> {
    let mut cmd = vec!["dbus-run-session", "--", "gsettings"];
    match value {
        Some(value) => cmd.extend(["set", schema, key, value]),
        None => cmd.extend(["reset", schema, key]),
    }
    cmd.into_iter().map(str::to_owned).collect()
}

const XFCONF_CREATE_IF_MISSING: &str = "-n";

pub(crate) fn xfconf(channel: &str, property: &str, ty: &str, value: Option<&str>) -> Vec<String> {
    let mut cmd = vec!["dbus-run-session", "--", "xfconf-query", "-c", channel, "-p", property];
    match value {
        Some(value) => cmd.extend([XFCONF_CREATE_IF_MISSING, "-t", ty, "-s", value]),
        None => cmd.push("-r"),
    }
    cmd.into_iter().map(str::to_owned).collect()
}

const RUN_COUNTED_COMMANDS: &str = r#"
status=0
while [ $# -gt 0 ]; do
    n=$1
    shift
    c=
    i=1
    while [ "$i" -le "$n" ]; do
        c="$c \"\${$i}\""
        i=$((i + 1))
    done
    eval "$c" || status=1
    shift "$n"
done
if [ -n "$TAIDAN_XFCONF_WAIT" ]; then sleep "$TAIDAN_XFCONF_WAIT"; fi
exit "$status"
"#;

// dbus-run-session doesn't wait for xfconfd to save as its bus goes away
const XFCONFD_SAVE_DELAY: &str = "6";

fn one_session(cmds: Vec<Vec<String>>) -> Option<Vec<String>> {
    if cmds.is_empty() {
        return None;
    }
    let mut any_xfconf = false;
    let mut any_wants_dbus = false;
    let mut args = vec![];
    for cmd in cmds {
        let cmd = match cmd.as_slice() {
            [run, dashes, rest @ ..] if run == "dbus-run-session" && dashes == "--" => {
                any_wants_dbus = true;
                rest.to_vec()
            }
            _ => cmd,
        };
        any_xfconf |= cmd.first().is_some_and(|c| c == "xfconf-query");
        let env_cmd = std::iter::once("env".to_owned()).chain(cmd).collect_vec();
        args.push(env_cmd.len().to_string());
        args.extend(env_cmd);
    }
    let mut out =
        if any_wants_dbus { vec!["dbus-run-session".to_owned(), "--".to_owned()] } else { vec![] };
    out.push("env".to_owned());
    out.push(format!("TAIDAN_XFCONF_WAIT={}", if any_xfconf { XFCONFD_SAVE_DELAY } else { "" }));
    out.extend(["sh", "-c", RUN_COUNTED_COMMANDS, "sh"].map(str::to_owned));
    out.extend(args);
    Some(out)
}

fn homed_script(cmds: &[HomeCmd]) -> String {
    use crate::proxy::sh_quote;
    let mut script = "rc=0\n".to_owned();
    for HomeCmd { argv, input, .. } in cmds {
        let cmd = argv.iter().map(|a| sh_quote(a)).join(" ");
        match input {
            Some(input) => script += &format!("printf %s {} | {cmd} || rc=1\n", sh_quote(input)),
            None => script += &format!("{cmd} || rc=1\n"),
        }
    }
    script + "exit $rc\n"
}

const PASSWD_HOME_FIELD: usize = 5;

async fn home_of(user: &str) -> Res<PathBuf> {
    let out = tokio::process::Command::new("getent")
        .args(["passwd", user])
        .output()
        .await
        .map_err(|e| Err::FailToRunProgram("getent", e))?;
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .split(':')
        .nth(PASSWD_HOME_FIELD)
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| Err::ProgramFail {
            name: "getent",
            output: format!("no home directory for {user}"),
            rc: out.status.code(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homed_script_quotes_and_feeds_stdin() {
        let cat = HomeCmd { input: Some("a'b\n".to_owned()), ..HomeCmd::new(["cat"]) };
        let cmds = [HomeCmd::new(["echo", "it's; rm -rf /"]), cat, HomeCmd::new(["false"])];
        let out =
            std::process::Command::new("sh").args(["-c", &homed_script(&cmds)]).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout), "it's; rm -rf /\na'b\n");
        assert!(!out.status.success());
    }

    #[test]
    fn one_session_runs_them_all() {
        let cmd = |args: &[&str]| args.iter().map(|&s| s.to_owned()).collect::<Vec<_>>();
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out");
        let out_s = out.to_string_lossy().into_owned();
        let all = one_session(vec![
            cmd(&[
                "dbus-run-session",
                "--",
                "sh",
                "-c",
                "printf '%s|' \"$@\" >> \"$0\"",
                &out_s,
                "a b",
                "$(c)",
                "'d'",
            ]),
            cmd(&["X=1", "sh", "-c", "printf '%s' \"$X\" >> \"$0\"", &out_s]),
        ])
        .unwrap();
        assert_eq!(all[..3], ["dbus-run-session", "--", "env"]);
        let without_dbus = &all[2..];
        let status =
            std::process::Command::new(&without_dbus[0]).args(&without_dbus[1..]).status().unwrap();
        assert!(status.success());
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "a b|$(c)|'d'|1");
        assert!(one_session(vec![]).is_none());
    }

    #[test]
    fn skel_writes_and_removes_files() {
        let dir = tempfile::tempdir().unwrap();
        let skel = Home::Skel(dir.path().to_path_buf());
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let file = ".config/autostart/x.desktop";
        rt.block_on(skel.run(vec![skel.write_file(file, "hi\n")])).unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join(file)).unwrap(), "hi\n");
        rt.block_on(skel.run(vec![skel.remove_file(file)])).unwrap();
        assert!(!dir.path().join(file).exists());
        let failing = vec![HomeCmd::new(["false"]).best_effort(), HomeCmd::new(["false"])];
        assert!(rt.block_on(skel.run(failing)).is_err());
        assert!(rt.block_on(skel.run(vec![HomeCmd::new(["false"]).best_effort()])).is_ok());
    }
}
