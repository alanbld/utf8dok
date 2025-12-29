//! Bridge Framework Compliance Rules
//!
//! Implements validation rules specific to the Bridge documentation framework,
//! including ADR (Architecture Decision Record) status validation and orphan detection.

mod orphan;
mod status;

use crate::workspace::graph::WorkspaceGraph;

use super::{ComplianceRule, Violation};

pub use orphan::OrphanRule;
pub use status::StatusRule;

/// Collection of all Bridge Framework compliance rules
#[allow(dead_code)]
pub struct BridgeRules {
    rules: Vec<Box<dyn ComplianceRule>>,
}

#[allow(dead_code)]
impl BridgeRules {
    /// Create a new set of Bridge rules
    pub fn new() -> Self {
        Self {
            rules: vec![
                Box::new(StatusRule::new()),
                Box::new(OrphanRule::new()),
            ],
        }
    }

    /// Validate the workspace graph against all Bridge rules
    pub fn validate(&self, graph: &WorkspaceGraph) -> Vec<Violation> {
        let mut all_violations = Vec::new();

        for rule in &self.rules {
            let violations = rule.check(graph);
            all_violations.extend(violations);
        }

        all_violations
    }
}

impl Default for BridgeRules {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_rules_creation() {
        let rules = BridgeRules::new();
        assert_eq!(rules.rules.len(), 2);
    }

    #[test]
    fn test_bridge_rules_empty_graph() {
        let graph = WorkspaceGraph::new();
        let rules = BridgeRules::new();
        let violations = rules.validate(&graph);
        assert!(violations.is_empty());
    }
}
