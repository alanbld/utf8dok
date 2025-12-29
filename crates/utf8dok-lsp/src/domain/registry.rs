//! Domain Registry
//!
//! Manages domain plugins and selects the appropriate domain for each document.

use crate::domain::plugins::{BridgePlugin, GenericPlugin, RfcPlugin};
use crate::domain::traits::DocumentDomain;
use std::sync::Arc;

/// The domain registry manages all available domain plugins
/// and selects the most appropriate one for each document.
pub struct DomainRegistry {
    /// Registered domain plugins (in priority order)
    domains: Vec<Arc<dyn DocumentDomain>>,
    /// Fallback domain for unrecognized documents
    fallback: Arc<dyn DocumentDomain>,
}

impl DomainRegistry {
    /// Create a new registry with default plugins
    pub fn new() -> Self {
        // Register built-in plugins (order matters for tie-breaking)
        let domains: Vec<Arc<dyn DocumentDomain>> = vec![
            Arc::new(BridgePlugin::new()),
            Arc::new(RfcPlugin::new()),
        ];

        Self {
            domains,
            fallback: Arc::new(GenericPlugin::new()),
        }
    }

    /// Detect the most appropriate domain for a document.
    ///
    /// Returns the domain and its confidence score, or the fallback domain
    /// if no specific domain matches.
    pub fn detect_domain(&self, text: &str) -> Option<(Arc<dyn DocumentDomain>, f32)> {
        // Score all domains
        let mut scored: Vec<_> = self
            .domains
            .iter()
            .map(|domain| {
                let score = domain.score_document(text);
                (domain.clone(), score)
            })
            .filter(|(_, score)| *score > 0.2) // Threshold for considering
            .collect();

        // Sort by score (highest first)
        scored.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        // Return best match or fallback
        if let Some((domain, score)) = scored.into_iter().next() {
            Some((domain, score))
        } else {
            // Return fallback with its score
            let fallback_score = self.fallback.score_document(text);
            Some((self.fallback.clone(), fallback_score))
        }
    }

    /// Get a domain by name (for testing and direct access)
    #[allow(dead_code)]
    pub fn get_domain(&self, name: &str) -> Option<Arc<dyn DocumentDomain>> {
        // Check registered domains
        if let Some(domain) = self.domains.iter().find(|d| d.name() == name) {
            return Some(domain.clone());
        }

        // Check fallback
        if self.fallback.name() == name {
            return Some(self.fallback.clone());
        }

        None
    }

    /// Get all registered domains
    #[allow(dead_code)]
    pub fn domains(&self) -> &[Arc<dyn DocumentDomain>] {
        &self.domains
    }

    /// Get the fallback domain
    pub fn fallback(&self) -> Arc<dyn DocumentDomain> {
        self.fallback.clone()
    }

    /// Register a new domain plugin
    #[allow(dead_code)]
    pub fn register(&mut self, domain: Arc<dyn DocumentDomain>) {
        self.domains.push(domain);
    }
}

impl Default for DomainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DomainRegistry {
    fn clone(&self) -> Self {
        Self {
            domains: self.domains.clone(),
            fallback: self.fallback.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = DomainRegistry::new();

        assert_eq!(registry.domains.len(), 2); // Bridge + RFC
        assert_eq!(registry.fallback.name(), "generic");
    }

    #[test]
    fn test_domain_detection() {
        let registry = DomainRegistry::new();

        // Bridge document
        let bridge_doc = "= ADR 001: Test\n:status: Draft";
        let (domain, score) = registry.detect_domain(bridge_doc).unwrap();
        assert_eq!(domain.name(), "bridge");
        assert!(score > 0.5);

        // RFC document
        let rfc_doc = "= RFC 1234: Test\n:category: standards-track";
        let (domain, score) = registry.detect_domain(rfc_doc).unwrap();
        assert_eq!(domain.name(), "rfc");
        assert!(score > 0.5);

        // Plain document -> fallback
        let plain_doc = "Just some text.";
        let (domain, _) = registry.detect_domain(plain_doc).unwrap();
        assert_eq!(domain.name(), "generic");
    }

    #[test]
    fn test_get_domain_by_name() {
        let registry = DomainRegistry::new();

        assert!(registry.get_domain("bridge").is_some());
        assert!(registry.get_domain("rfc").is_some());
        assert!(registry.get_domain("generic").is_some());
        assert!(registry.get_domain("unknown").is_none());
    }
}
