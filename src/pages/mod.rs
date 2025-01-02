//! Pages inside the application.
//!
//! You might think the naming of the inner modules is weird, but in fact it's
//! designed such that they are sorted correctly inside most code editors.

#![allow(clippy::semicolon_outside_block)] // bug from relm4 component macro
pub mod _00_welcome;
pub mod _01_keyboard;
pub mod _02_whoareyou;
pub mod _03_password;
pub mod _04_internet;
pub mod _05_analytics;
pub mod _06_crashreport;
pub mod _07_location;
pub mod _08_nightlight;
pub mod _09_theme;
pub mod _10_browser;
pub mod _11_categories;
pub mod _12_installing;
pub mod _13_finish;
pub mod _14_error;
pub use _12_installing::InstallingPageMsg;
