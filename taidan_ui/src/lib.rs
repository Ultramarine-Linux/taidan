//! the code Slint generates from `ui/`, used through `crate::ui` in Taidan
//!
//! It's ~20 MB of Rust, so it lives in its own crate. Changing Taidan's Rust then only relinks
//! this instead of compiling it again.

slint::include_modules!();
