#[derive(Debug, thiserror::Error)]
pub enum Err {
    #[error("fail to run program `{0}`: {1}")]
    FailToRunProgram(&'static str, std::io::Error),
    #[error("program `{name}` failed{}", fail_detail(*rc, output))]
    ProgramFail { name: &'static str, output: String, rc: Option<i32> },
    #[error("request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("{0}: {1}")]
    WrapMsg(&'static str, Box<Self>),
    #[error("{0}: {1}")]
    WrapMessage(String, Box<Self>),
    #[error("{0}")]
    Msg(&'static str),
    #[error("{0}")]
    Message(String),
    #[error("cannot generate password: {0}")]
    XCrypt(#[from] xcrypt::Error),
    #[error("io error: {0}")]
    Io(std::io::Error),
    #[error("error working with json: {0}")]
    Serde(#[from] serde_json::Error),
}

fn fail_detail(rc: Option<i32>, output: &str) -> String {
    let mut detail = rc.map(|rc| format!(" with exit code {rc}")).unwrap_or_default();
    if let Some(line) = output.lines().map(str::trim).rfind(|l| !l.is_empty()) {
        detail.push_str(": ");
        detail.extend(line.chars().take(300));
    }
    detail
}

impl Err {
    /// Details for local troubleshooting, including command output omitted by `Display`.
    #[must_use]
    pub fn diagnostic(&self) -> String {
        match self {
            Self::WrapMsg(message, source) => format!("{message}: {}", source.diagnostic()),
            Self::WrapMessage(message, source) => format!("{message}: {}", source.diagnostic()),
            Self::ProgramFail { output, .. } if !output.trim().is_empty() => {
                format!("{self}\n\nCommand output:\n{output}")
            }
            _ => self.to_string(),
        }
    }

    #[must_use]
    pub fn program_fail(name: &'static str, out: &std::process::Output) -> Self {
        let mut output = String::from_utf8_lossy(&out.stdout).into_owned();
        output.push_str(&String::from_utf8_lossy(&out.stderr));
        crate::command::report_exit(name, out.status, &output);
        Self::ProgramFail { name, output, rc: out.status.code() }
    }

    pub fn _fail_to_run(program: &'static str) -> impl FnOnce(std::io::Error) -> Self {
        move |e| Self::FailToRunProgram(program, e)
    }
}

pub type Res<T> = Result<T, Err>;

pub trait TaidanErrHelper {
    fn wrap_msg(self, msg: &'static str) -> Self;
    fn wrap_message<F: FnOnce(&Err) -> String>(self, msg: F) -> Self;
}

impl<T> TaidanErrHelper for Result<T, Err> {
    fn wrap_msg(self, msg: &'static str) -> Self {
        self.map_err(|e| Err::WrapMsg(msg, Box::new(e)))
    }

    fn wrap_message<F: FnOnce(&Err) -> String>(self, msg: F) -> Self {
        self.map_err(|e| Err::WrapMessage(msg(&e), Box::new(e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_keep_wrapped_command_output() {
        let output = format!("{}important first line\nlast line\n", "x".repeat(400));
        let error = Err::WrapMsg(
            "cannot create the user",
            Box::new(Err::ProgramFail { name: "useradd", output: output.clone(), rc: Some(1) }),
        );
        assert!(!error.to_string().contains("important first line"));
        let detail = error.diagnostic();
        assert!(detail.starts_with("cannot create the user: program `useradd` failed"));
        assert!(detail.contains(&output));
    }

    #[test]
    fn ordinary_diagnostics_keep_the_error_message() {
        let error = Err::Msg("refusing to create a user without a password");
        assert_eq!(error.diagnostic(), error.to_string());
    }
}
