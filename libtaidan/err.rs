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

/// the exit code and the last line the program printed, which is usually why it failed, for the
/// error page
fn fail_detail(rc: Option<i32>, output: &str) -> String {
    let mut detail = rc.map(|rc| format!(" with exit code {rc}")).unwrap_or_default();
    if let Some(line) = output.lines().map(str::trim).rfind(|l| !l.is_empty()) {
        detail.push_str(": ");
        detail.extend(line.chars().take(300));
    }
    detail
}

impl Err {
    /// a [`Err::ProgramFail`] from what `name` printed, stdout then stderr
    #[must_use]
    pub fn program_fail(name: &'static str, out: &std::process::Output) -> Self {
        let mut output = String::from_utf8_lossy(&out.stdout).into_owned();
        output.push_str(&String::from_utf8_lossy(&out.stderr));
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
