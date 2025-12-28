//! Intelligence module for utf8dok LSP
//!
//! This module provides smart editing features:
//! - Selection ranges (hierarchy-aware selection expansion)
//! - Rename refactoring (section IDs, attributes, cross-references)

pub mod rename;
pub mod selection;

#[cfg(test)]
mod tests;

pub use rename::RenameAnalyzer;
pub use selection::SelectionAnalyzer;
