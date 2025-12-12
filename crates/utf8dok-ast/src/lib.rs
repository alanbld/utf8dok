//! utf8dok-ast - Abstract Syntax Tree definitions
//!
//! This crate provides the AST types used by utf8dok for representing
//! parsed document structures.

/// Re-export the version from core
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.0.1");
    }
}
