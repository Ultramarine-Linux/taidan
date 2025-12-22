//! Pages inside the application.
//!
//! You might think the naming of the inner modules is weird, but in fact it's
//! designed such that they are sorted correctly inside most code editors.

#![allow(clippy::semicolon_outside_block)] // bug from relm4 component macro
pub mod _00_language;
pub mod _01_welcome;
pub mod _02_keyboard;
pub mod _03_devicename;
pub mod _04_whoareyou;
pub mod _05_password;
pub mod _06_internet;
pub mod _07_analytics;
pub mod _08_crashreport;
pub mod _09_tweaks;
pub mod _10_codecs;
pub mod _11_inputmethod;
pub mod _12_nightlight;
pub mod _13_theme;
pub mod _14_browser;
pub mod _15_categories;
pub mod _16_installing;
pub mod _17_finish;
pub mod _18_error;

pub(crate) use _14_browser::BROWSER_CATEGORY;
pub use _16_installing::InstallingPageMsg;
pub use _18_error::ErrorPageMsg;
