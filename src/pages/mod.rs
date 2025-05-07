//! Pages inside the application.
//!
//! You might think the naming of the inner modules is weird, but in fact it's
//! designed such that they are sorted correctly inside most code editors.

#![allow(clippy::semicolon_outside_block)] // bug from relm4 component macro
pub mod _00_language;
pub mod _01_welcome;
pub mod _02_keyboard;
pub mod _03_whoareyou;
pub mod _04_password;
pub mod _05_internet;
pub mod _06_analytics;
pub mod _07_crashreport;
pub mod _08_tweaks;
pub mod _09_codecs;
pub mod _10_inputmethod;
pub mod _11_nightlight;
pub mod _12_theme;
pub mod _13_browser;
pub mod _14_categories;
pub mod _15_installing;
pub mod _16_finish;
pub mod _17_error;

pub(crate) use _13_browser::BROWSER_CATEGORY;
pub use _15_installing::InstallingPageMsg;
pub use _17_error::ErrorPageMsg;
