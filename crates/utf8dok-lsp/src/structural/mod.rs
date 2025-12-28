//! Structural intelligence for utf8dok LSP
//!
//! This module provides document structure analysis for:
//! - Folding ranges (attribute groups, headers, blocks)
//! - Document symbols (outline view)
//!
//! The implementation follows TDD principles with comprehensive tests.

pub mod folding;
pub mod scanner;
pub mod symbols;

pub use folding::FoldingAnalyzer;

#[cfg(test)]
mod tests;
