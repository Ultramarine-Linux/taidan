//! Pages inside the application.
//!
//! You might think the naming of the inner modules is weird, but in fact it's
//! designed such that they are sorted correctly inside most code editors.

#![allow(clippy::semicolon_outside_block)] // bug from relm4 component macro
pub mod _00_welcome;
pub mod _01_keyboard;
pub mod _02_inputmethod;
pub mod _03_whoareyou;
pub mod _04_password;
pub mod _05_internet;
pub mod _06_analytics;
pub mod _07_crashreport;
pub mod _08_location;
pub mod _09_nightlight;
pub mod _10_theme;
pub mod _11_browser;
pub mod _12_categories;
pub mod _13_installing;
pub mod _14_finish;
pub mod _15_error;
pub use _13_installing::InstallingPageMsg;
