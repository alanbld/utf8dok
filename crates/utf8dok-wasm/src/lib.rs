//! utf8dok-wasm - WebAssembly bindings for utf8dok
//!
//! This crate provides WASM bindings to use utf8dok in web browsers
//! and other WASM-compatible environments.

use wasm_bindgen::prelude::*;

/// Returns the current version of utf8dok
#[wasm_bindgen]
pub fn version() -> String {
    utf8dok_core::VERSION.to_string()
}

/// Placeholder for future document parsing functionality
#[wasm_bindgen]
pub fn parse(_input: &str) -> String {
    "Parsing not yet implemented".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.1.0");
    }
}
