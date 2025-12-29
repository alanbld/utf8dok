//! Workspace Intelligence Module
//!
//! Provides cross-file navigation, validation, and refactoring through
//! a knowledge graph that indexes all workspace documents.
//!
//! # Components
//!
//! - `graph`: The core knowledge graph storing definitions, references, and symbols
//! - `indexer`: Extracts structural information from document content
//! - `symbol_provider`: LSP workspace/symbol functionality

pub mod graph;
pub mod indexer;
pub mod symbol_provider;

#[cfg(test)]
mod tests;

pub use graph::WorkspaceGraph;
