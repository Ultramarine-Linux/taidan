//! the code Slint generates from `ui/`, used through `crate::ui` in Taidan
//!
//! it's ~20 MB of Rust, so it lives in its own crate. changing Taidan's Rust then only relinks
//! it instead of compiling it again.

slint::include_modules!();
