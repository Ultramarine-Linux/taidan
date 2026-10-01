pub use color_eyre::{
    Section, SectionExt,
    eyre::{Context, eyre},
};
pub use itertools::{Either, Itertools};

pub use crate::{CFG, SETTINGS, t};

pub(crate) static REQWEST_CLIENT: std::sync::LazyLock<reqwest::Client> =
    std::sync::LazyLock::new(reqwest::Client::new);

pub use std::sync::LazyLock;
