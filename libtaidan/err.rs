use std::borrow::Cow;

#[derive(Debug, thiserror::Error)]
pub enum Err {
    #[error("fail to run program `{0}`: {1}")]
    FailToRunProgram(&'static str, std::io::Error),
    #[error("program `{name}` failed")]
    ProgramFail { name: &'static str, output: String, rc: Option<i32> },
    #[error("fail to read dir `{0}`: {1}")]
    ReadDir(&'static str, std::io::Error),
    #[error("fail to read file `{0}`: {1}")]
    ReadFile(Cow<'static, std::path::Path>, std::io::Error),
    #[error(
        "unknown repo `{0}` (this does not seem like a url, and this repo is not installed in /etc/yum.repos.d/)"
    )]
    UnknownRepo(String),
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

impl Err {
    pub fn _read_dir(dir: &'static str) -> impl FnOnce(std::io::Error) -> Self {
        move |e| Self::ReadDir(dir, e)
    }
    pub fn _read_file(file: Cow<'static, std::path::Path>) -> impl FnOnce(std::io::Error) -> Self {
        |e| Self::ReadFile(file, e)
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
