//! Relationships parsing for OOXML documents
//!
//! OOXML uses relationship files (_rels/*.rels) to map IDs to targets.
//! This is used for hyperlinks, images, and other external references.

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::{OoxmlError, Result};

/// Parsed relationships from a .rels file
#[derive(Debug, Clone, Default)]
pub struct Relationships {
    /// Map of relationship ID to target URL/path
    map: HashMap<String, RelationshipTarget>,
}

/// A relationship target with its type
#[derive(Debug, Clone)]
pub struct RelationshipTarget {
    /// The target URL or path
    pub target: String,
    /// The relationship type (e.g., hyperlink, image, styles)
    pub rel_type: String,
}

impl Relationships {
    /// Create an empty relationships map
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse relationships from XML bytes
    pub fn parse(xml: &[u8]) -> Result<Self> {
        let mut reader = Reader::from_reader(xml);
        reader.config_mut().trim_text(true);

        let mut map = HashMap::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                    if e.local_name().as_ref() == b"Relationship" {
                        let mut id = None;
                        let mut target = None;
                        let mut rel_type = None;

                        for attr in e.attributes().filter_map(|a| a.ok()) {
                            match attr.key.as_ref() {
                                b"Id" => {
                                    id = String::from_utf8(attr.value.to_vec()).ok();
                                }
                                b"Target" => {
                                    target = String::from_utf8(attr.value.to_vec()).ok();
                                }
                                b"Type" => {
                                    rel_type = String::from_utf8(attr.value.to_vec()).ok();
                                }
                                _ => {}
                            }
                        }

                        if let (Some(id), Some(target)) = (id, target) {
                            map.insert(
                                id,
                                RelationshipTarget {
                                    target,
                                    rel_type: rel_type.unwrap_or_default(),
                                },
                            );
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(OoxmlError::Xml(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(Self { map })
    }

    /// Get the target for a relationship ID
    pub fn get(&self, id: &str) -> Option<&str> {
        self.map.get(id).map(|r| r.target.as_str())
    }

    /// Get the full relationship target for an ID
    pub fn get_target(&self, id: &str) -> Option<&RelationshipTarget> {
        self.map.get(id)
    }

    /// Check if a relationship ID exists
    pub fn contains(&self, id: &str) -> bool {
        self.map.contains_key(id)
    }

    /// Check if a relationship is a hyperlink
    pub fn is_hyperlink(&self, id: &str) -> bool {
        self.map
            .get(id)
            .map(|r| r.rel_type.contains("hyperlink"))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_relationships() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
            <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
        </Relationships>"#;

        let rels = Relationships::parse(xml).unwrap();

        assert_eq!(rels.get("rId1"), Some("https://example.com"));
        assert_eq!(rels.get("rId2"), Some("styles.xml"));
        assert!(rels.is_hyperlink("rId1"));
        assert!(!rels.is_hyperlink("rId2"));
    }

    #[test]
    fn test_empty_relationships() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
        </Relationships>"#;

        let rels = Relationships::parse(xml).unwrap();
        assert!(rels.get("rId1").is_none());
    }
}
