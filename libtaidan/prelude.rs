pub use crate::err::{Err, Res, TaidanErrHelper};
pub use itertools::{Either, Itertools};
pub use std::sync::LazyLock;

pub(crate) static REQWEST_CLIENT: std::sync::LazyLock<reqwest::Client> =
    std::sync::LazyLock::new(reqwest::Client::new);
