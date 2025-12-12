//! utf8dok - Plain text, powerful docs
//!
//! A blazing-fast document processor for UTF-8 text formats.
//! Currently under active development.
//!
//! Coming soon:
//! - AsciiDoc support
//! - Multiple output formats
//! - WASM compilation
//!
//! Follow development: https://github.com/alanbld/utf8dok

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.0.1");
    }
}
