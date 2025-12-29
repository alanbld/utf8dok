//! Domain Plugins
//!
//! This module contains the domain-specific plugin implementations.
//! Each plugin implements the `DocumentDomain` trait.

mod bridge;
mod generic;
mod rfc;

pub use bridge::BridgePlugin;
pub use generic::GenericPlugin;
pub use rfc::RfcPlugin;
