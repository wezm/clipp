//! simple possibly cross platform clipboard crate
//!
//! ```
//! clipp::copy("wow such clipboard");
//! assert_eq!(clipp::paste(), "wow such clipboard");
//! ```
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
mod providers;

use std::{io, fmt::Display, sync::OnceLock};

static CLIP: OnceLock<io::Result<providers::Board>> = OnceLock::new();

/// Copy text to the clipboard.
pub fn copy(text: impl Display) {
    CLIP.get_or_init(providers::provide)
        .unwrap()
        .0(&format!("{text}")).unwrap()
}

/// Copy text to the clipboard.
pub fn copy2(text: &str) -> io::Result<()> {
    CLIP.get_or_init(providers::provide)?.0(&text)
}

/// Paste text from the clipboard.
pub fn paste() -> String {
    CLIP.get_or_init(providers::provide)
        .unwrap()
        .1().unwrap()
}

/// Paste text from the clipboard.
pub fn paste2() -> io::Result<String> {
    CLIP.get_or_init(providers::provide)?.1()
}
