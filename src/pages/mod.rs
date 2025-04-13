//! Pages inside the application.
//!
//! You might think the naming of the inner modules is weird, but in fact it's
//! designed such that they are sorted correctly inside most code editors.

#![allow(clippy::semicolon_outside_block)] // bug from relm4 component macro
pub mod _00_welcome;
pub mod _01_keyboard;
// pub mod _02_whoareyou;
pub mod _01_whoareyou;
// pub mod _03_password;
pub mod _02_password;
pub mod _04_internet;
pub mod _05_analytics;
pub mod _06_crashreport;
pub mod _07_location;
pub mod _08_codecs;
pub mod _09_inputmethod;
// pub mod _10_nightlight;
pub mod _03_nightlight;
// pub mod _11_theme;
pub mod _04_theme;
pub mod _12_browser;
pub mod _13_categories;
// pub mod _14_installing;
pub mod _05_installing;
// pub mod _15_finish;
pub mod _06_finish;
// pub mod _16_error;
pub mod _07_error;

// pub use _14_installing::InstallingPageMsg;
pub use _05_installing::InstallingPageMsg;