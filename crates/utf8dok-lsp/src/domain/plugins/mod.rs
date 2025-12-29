//! Domain Plugins
//!
//! This module contains the domain-specific plugin implementations.
//! Each plugin implements the `DocumentDomain` trait.
//!
//! Content plugins (Phase 17) provide additional analysis:
//! - `QualityPlugin`: Writing quality checks (passive voice, weasel words, readability)
//! - `DiagramPlugin`: Diagram syntax validation (Mermaid, PlantUML)

mod bridge;
mod diagrams;
mod generic;
mod quality;
mod rfc;

pub use bridge::BridgePlugin;
pub use diagrams::DiagramPlugin;
pub use generic::GenericPlugin;
pub use quality::{QualityPlugin, QualitySummary};
pub use rfc::RfcPlugin;
