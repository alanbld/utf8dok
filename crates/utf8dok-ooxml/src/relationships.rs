//! Relationships parsing and modification for OOXML documents
//!
//! OOXML uses relationship files (_rels/*.rels) to map IDs to targets.
//! This is used for hyperlinks, images, and other external references.
//!
//! # Example
//!
//! ```ignore
//! use utf8dok_ooxml::relationships::Relationships;
//!
//! // Parse existing relationships
//! let mut rels = Relationships::parse(xml_bytes)?;
//!
//! // Add a new image relationship
//! let id = rels.add(
//!     "media/image1.png".to_string(),
//!     Relationships::TYPE_IMAGE.to_string(),
//! );
//!
//! // Serialize back to XML
//! let xml = rels.to_xml();
//! ```

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::{OoxmlError, Result};

/// OOXML namespace for relationships
pub const RELATIONSHIPS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// Common relationship type URIs
impl Relationships {
    /// Hyperlink relationship type
    pub const TYPE_HYPERLINK: &'static str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
    /// Image relationship type
    pub const TYPE_IMAGE: &'static str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
    /// Styles relationship type
    pub const TYPE_STYLES: &'static str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
    /// Numbering relationship type
    pub const TYPE_NUMBERING: &'static str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering";
    /// Font table relationship type
    pub const TYPE_FONT_TABLE: &'static str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/fontTable";
    /// Settings relationship type
    pub const TYPE_SETTINGS: &'static str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings";
}

/// Parsed relationships from a .rels file
///
/// Maintains insertion order for deterministic XML serialization.
#[derive(Debug, Clone)]
pub struct Relationships {
    /// Ordered list of relationship IDs (maintains insertion order)
    order: Vec<String>,
    /// Map of relationship ID to target (for fast lookups)
    map: HashMap<String, RelationshipTarget>,
    /// Counter for generating unique IDs (starts at 1)
    next_id_counter: u32,
}

impl Default for Relationships {
    fn default() -> Self {
        Self {
            order: Vec::new(),
            map: HashMap::new(),
            next_id_counter: 1, // IDs start at rId1
        }
    }
}

/// A relationship target with its type and mode
#[derive(Debug, Clone)]
pub struct RelationshipTarget {
    /// The target URL or path
    pub target: String,
    /// The relationship type URI (e.g., hyperlink, image, styles)
    pub rel_type: String,
    /// Target mode: "External" for URLs, None for internal paths
    pub target_mode: Option<String>,
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

        let mut order = Vec::new();
        let mut map = HashMap::new();
        let mut max_id: u32 = 0;
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                    if e.local_name().as_ref() == b"Relationship" {
                        let mut id = None;
                        let mut target = None;
                        let mut rel_type = None;
                        let mut target_mode = None;

                        for attr in e.attributes().filter_map(|a| a.ok()) {
                            match attr.key.as_ref() {
                                b"Id" => {
                                    id = attr.unescape_value().ok().map(|s| s.to_string());
                                }
                                b"Target" => {
                                    target = attr.unescape_value().ok().map(|s| s.to_string());
                                }
                                b"Type" => {
                                    rel_type = attr.unescape_value().ok().map(|s| s.to_string());
                                }
                                b"TargetMode" => {
                                    target_mode = attr.unescape_value().ok().map(|s| s.to_string());
                                }
                                _ => {}
                            }
                        }

                        if let (Some(id), Some(target)) = (id, target) {
                            // Track the maximum numeric ID for generating new IDs
                            if let Some(num) = extract_id_number(&id) {
                                max_id = max_id.max(num);
                            }

                            order.push(id.clone());
                            map.insert(
                                id,
                                RelationshipTarget {
                                    target,
                                    rel_type: rel_type.unwrap_or_default(),
                                    target_mode,
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

        Ok(Self {
            order,
            map,
            next_id_counter: max_id + 1,
        })
    }

    /// Add a new relationship and return the generated ID
    ///
    /// # Arguments
    ///
    /// * `target` - The target path or URL
    /// * `rel_type` - The relationship type URI (use TYPE_* constants)
    ///
    /// # Returns
    ///
    /// The generated relationship ID (e.g., "rId3")
    ///
    /// # Example
    ///
    /// ```ignore
    /// let id = rels.add(
    ///     "media/image1.png".to_string(),
    ///     Relationships::TYPE_IMAGE.to_string(),
    /// );
    /// assert!(id.starts_with("rId"));
    /// ```
    pub fn add(&mut self, target: String, rel_type: String) -> String {
        let id = format!("rId{}", self.next_id_counter);
        self.next_id_counter += 1;

        // Determine target mode based on type or target
        let target_mode = if rel_type.contains("hyperlink") && target.starts_with("http") {
            Some("External".to_string())
        } else {
            None
        };

        self.order.push(id.clone());
        self.map.insert(
            id.clone(),
            RelationshipTarget {
                target,
                rel_type,
                target_mode,
            },
        );

        id
    }

    /// Add a new relationship with explicit target mode
    pub fn add_with_mode(
        &mut self,
        target: String,
        rel_type: String,
        target_mode: Option<String>,
    ) -> String {
        let id = format!("rId{}", self.next_id_counter);
        self.next_id_counter += 1;

        self.order.push(id.clone());
        self.map.insert(
            id.clone(),
            RelationshipTarget {
                target,
                rel_type,
                target_mode,
            },
        );

        id
    }

    /// Serialize relationships to OOXML format
    ///
    /// Returns valid XML that can be written to a .rels file.
    pub fn to_xml(&self) -> String {
        let mut xml = String::new();
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
        xml.push('\n');
        xml.push_str(&format!(r#"<Relationships xmlns="{}">"#, RELATIONSHIPS_NS));
        xml.push('\n');

        // Iterate in insertion order for deterministic output
        for id in &self.order {
            if let Some(rel) = self.map.get(id) {
                xml.push_str("  <Relationship");
                xml.push_str(&format!(r#" Id="{}""#, escape_xml(id)));
                xml.push_str(&format!(r#" Type="{}""#, escape_xml(&rel.rel_type)));
                xml.push_str(&format!(r#" Target="{}""#, escape_xml(&rel.target)));
                if let Some(mode) = &rel.target_mode {
                    xml.push_str(&format!(r#" TargetMode="{}""#, escape_xml(mode)));
                }
                xml.push_str("/>\n");
            }
        }

        xml.push_str("</Relationships>");
        xml
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

    /// Check if a relationship is an image
    pub fn is_image(&self, id: &str) -> bool {
        self.map
            .get(id)
            .map(|r| r.rel_type.contains("image"))
            .unwrap_or(false)
    }

    /// Get the number of relationships
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if there are no relationships
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Iterate over relationships in insertion order
    pub fn iter(&self) -> impl Iterator<Item = (&str, &RelationshipTarget)> {
        self.order
            .iter()
            .filter_map(|id| self.map.get(id).map(|rel| (id.as_str(), rel)))
    }

    /// Get the next ID that would be generated (without incrementing)
    pub fn peek_next_id(&self) -> String {
        format!("rId{}", self.next_id_counter)
    }

    /// Add an image relationship (convenience method)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let id = rels.add_image("media/image1.png");
    /// assert!(id.starts_with("rId"));
    /// ```
    pub fn add_image(&mut self, target: &str) -> String {
        self.add(target.to_string(), Self::TYPE_IMAGE.to_string())
    }

    /// Add a hyperlink relationship (convenience method)
    pub fn add_hyperlink(&mut self, url: &str) -> String {
        self.add_with_mode(
            url.to_string(),
            Self::TYPE_HYPERLINK.to_string(),
            Some("External".to_string()),
        )
    }
}

/// Extract the numeric portion from a relationship ID (e.g., "rId5" -> 5)
fn extract_id_number(id: &str) -> Option<u32> {
    id.strip_prefix("rId")
        .or_else(|| id.strip_prefix("RId"))
        .or_else(|| id.strip_prefix("rid"))
        .and_then(|num_str| num_str.parse().ok())
}

/// Escape special XML characters in attribute values
fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
        assert_eq!(rels.len(), 2);
    }

    #[test]
    fn test_empty_relationships() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
        </Relationships>"#;

        let rels = Relationships::parse(xml).unwrap();
        assert!(rels.get("rId1").is_none());
        assert!(rels.is_empty());
    }

    #[test]
    fn test_add_relationship() {
        let mut rels = Relationships::new();

        let id1 = rels.add(
            "media/image1.png".to_string(),
            Relationships::TYPE_IMAGE.to_string(),
        );
        assert_eq!(id1, "rId1");
        assert_eq!(rels.get("rId1"), Some("media/image1.png"));
        assert!(rels.is_image("rId1"));

        let id2 = rels.add(
            "https://example.com".to_string(),
            Relationships::TYPE_HYPERLINK.to_string(),
        );
        assert_eq!(id2, "rId2");
        assert!(rels.is_hyperlink("rId2"));

        assert_eq!(rels.len(), 2);
    }

    #[test]
    fn test_add_continues_from_existing() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
            <Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>
        </Relationships>"#;

        let mut rels = Relationships::parse(xml).unwrap();
        assert_eq!(rels.len(), 2);

        // Next ID should be rId6 (max existing is 5)
        let new_id = rels.add(
            "media/image1.png".to_string(),
            Relationships::TYPE_IMAGE.to_string(),
        );
        assert_eq!(new_id, "rId6");
    }

    #[test]
    fn test_to_xml() {
        let mut rels = Relationships::new();
        rels.add(
            "styles.xml".to_string(),
            Relationships::TYPE_STYLES.to_string(),
        );
        rels.add_with_mode(
            "https://example.com".to_string(),
            Relationships::TYPE_HYPERLINK.to_string(),
            Some("External".to_string()),
        );

        let xml = rels.to_xml();

        assert!(xml.contains(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#));
        assert!(xml.contains(&format!(r#"xmlns="{}""#, RELATIONSHIPS_NS)));
        assert!(xml.contains(r#"Id="rId1""#));
        assert!(xml.contains(r#"Target="styles.xml""#));
        assert!(xml.contains(r#"Id="rId2""#));
        assert!(xml.contains(r#"Target="https://example.com""#));
        assert!(xml.contains(r#"TargetMode="External""#));
    }

    #[test]
    fn test_add_relationship_and_serialize() {
        // Parse existing relationships
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
        </Relationships>"#;

        let mut rels = Relationships::parse(xml).unwrap();
        assert_eq!(rels.len(), 1);

        // Add new relationships
        let img_id = rels.add(
            "media/diagram1.png".to_string(),
            Relationships::TYPE_IMAGE.to_string(),
        );
        assert_eq!(img_id, "rId2");

        let link_id = rels.add_with_mode(
            "https://kroki.io".to_string(),
            Relationships::TYPE_HYPERLINK.to_string(),
            Some("External".to_string()),
        );
        assert_eq!(link_id, "rId3");

        assert_eq!(rels.len(), 3);

        // Serialize to XML
        let output_xml = rels.to_xml();

        // Verify structure
        assert!(output_xml.contains(r#"<?xml version="1.0""#));
        assert!(output_xml.contains("<Relationships"));
        assert!(output_xml.contains("</Relationships>"));

        // Verify all relationships present
        assert!(output_xml.contains(r#"Id="rId1""#));
        assert!(output_xml.contains(r#"Target="styles.xml""#));
        assert!(output_xml.contains(r#"Id="rId2""#));
        assert!(output_xml.contains(r#"Target="media/diagram1.png""#));
        assert!(output_xml.contains(r#"Id="rId3""#));
        assert!(output_xml.contains(r#"Target="https://kroki.io""#));
        assert!(output_xml.contains(r#"TargetMode="External""#));

        // Verify it can be re-parsed
        let reparsed = Relationships::parse(output_xml.as_bytes()).unwrap();
        assert_eq!(reparsed.len(), 3);
        assert_eq!(reparsed.get("rId1"), Some("styles.xml"));
        assert_eq!(reparsed.get("rId2"), Some("media/diagram1.png"));
        assert_eq!(reparsed.get("rId3"), Some("https://kroki.io"));
    }

    #[test]
    fn test_xml_escaping_in_serialization() {
        let mut rels = Relationships::new();
        rels.add(
            "file with <special> & \"chars\".xml".to_string(),
            Relationships::TYPE_STYLES.to_string(),
        );

        let xml = rels.to_xml();

        // Should be escaped
        assert!(xml.contains("&lt;special&gt;"));
        assert!(xml.contains("&amp;"));
        assert!(xml.contains("&quot;chars&quot;"));

        // Should be re-parseable
        let reparsed = Relationships::parse(xml.as_bytes()).unwrap();
        assert_eq!(
            reparsed.get("rId1"),
            Some("file with <special> & \"chars\".xml")
        );
    }

    #[test]
    fn test_iteration_order() {
        let mut rels = Relationships::new();
        rels.add("first.xml".to_string(), "type1".to_string());
        rels.add("second.xml".to_string(), "type2".to_string());
        rels.add("third.xml".to_string(), "type3".to_string());

        let targets: Vec<&str> = rels.iter().map(|(_, rel)| rel.target.as_str()).collect();
        assert_eq!(targets, vec!["first.xml", "second.xml", "third.xml"]);
    }

    #[test]
    fn test_peek_next_id() {
        let mut rels = Relationships::new();
        assert_eq!(rels.peek_next_id(), "rId1");

        rels.add("test.xml".to_string(), "type".to_string());
        assert_eq!(rels.peek_next_id(), "rId2");
    }

    #[test]
    fn test_extract_id_number() {
        assert_eq!(extract_id_number("rId1"), Some(1));
        assert_eq!(extract_id_number("rId123"), Some(123));
        assert_eq!(extract_id_number("RId5"), Some(5));
        assert_eq!(extract_id_number("rid10"), Some(10));
        assert_eq!(extract_id_number("invalid"), None);
        assert_eq!(extract_id_number("rIdabc"), None);
    }

    // ==================== Sprint 6: Edge Case Tests ====================

    #[test]
    fn test_parse_large_id_numbers() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId999" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
            <Relationship Id="rId1000" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="image.png"/>
        </Relationships>"#;

        let mut rels = Relationships::parse(xml).unwrap();
        assert_eq!(rels.get("rId999"), Some("styles.xml"));
        assert_eq!(rels.get("rId1000"), Some("image.png"));

        // Next ID should be 1001
        let new_id = rels.add("new.xml".to_string(), "type".to_string());
        assert_eq!(new_id, "rId1001");
    }

    #[test]
    fn test_parse_non_consecutive_ids() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="type" Target="first.xml"/>
            <Relationship Id="rId10" Type="type" Target="tenth.xml"/>
            <Relationship Id="rId5" Type="type" Target="fifth.xml"/>
        </Relationships>"#;

        let mut rels = Relationships::parse(xml).unwrap();
        assert_eq!(rels.len(), 3);

        // Verify all relationships are accessible
        assert_eq!(rels.get("rId1"), Some("first.xml"));
        assert_eq!(rels.get("rId5"), Some("fifth.xml"));
        assert_eq!(rels.get("rId10"), Some("tenth.xml"));

        // Next ID should be max + 1 = 11
        let new_id = rels.add("new.xml".to_string(), "type".to_string());
        assert_eq!(new_id, "rId11");
    }

    #[test]
    fn test_add_duplicate_target_different_type() {
        let mut rels = Relationships::new();

        // Add same URL as both image and hyperlink
        let img_id = rels.add(
            "https://example.com/file.png".to_string(),
            Relationships::TYPE_IMAGE.to_string(),
        );
        let link_id = rels.add(
            "https://example.com/file.png".to_string(),
            Relationships::TYPE_HYPERLINK.to_string(),
        );

        // Both should be added with different IDs
        assert_eq!(img_id, "rId1");
        assert_eq!(link_id, "rId2");
        assert_eq!(rels.len(), 2);

        // Both should be retrievable
        assert!(rels.is_image("rId1"));
        assert!(rels.is_hyperlink("rId2"));
    }

    #[test]
    fn test_extract_id_number_edge_cases() {
        // Leading zeros
        assert_eq!(extract_id_number("rId007"), Some(7));
        assert_eq!(extract_id_number("rId0"), Some(0));

        // Empty prefix
        assert_eq!(extract_id_number("rId"), None);
        assert_eq!(extract_id_number(""), None);

        // Numeric only (no prefix)
        assert_eq!(extract_id_number("123"), None);

        // Mixed case variations
        assert_eq!(extract_id_number("RId99"), Some(99));
        assert_eq!(extract_id_number("rid1"), Some(1));

        // Large numbers
        assert_eq!(extract_id_number("rId4294967295"), Some(4294967295)); // u32 max
    }

    #[test]
    fn test_convenience_methods() {
        let mut rels = Relationships::new();

        let img_id = rels.add_image("media/diagram.png");
        assert!(rels.is_image(&img_id));
        assert_eq!(rels.get(&img_id), Some("media/diagram.png"));

        let link_id = rels.add_hyperlink("https://example.com");
        assert!(rels.is_hyperlink(&link_id));
        assert_eq!(rels.get(&link_id), Some("https://example.com"));
    }

    #[test]
    fn test_is_empty_and_len() {
        let mut rels = Relationships::new();
        assert!(rels.is_empty());
        assert_eq!(rels.len(), 0);

        rels.add("test.xml".to_string(), "type".to_string());
        assert!(!rels.is_empty());
        assert_eq!(rels.len(), 1);
    }

    #[test]
    fn test_get_missing_relationship() {
        let rels = Relationships::new();
        assert_eq!(rels.get("rId1"), None);
        assert_eq!(rels.get("nonexistent"), None);
        assert!(!rels.is_hyperlink("rId1"));
        assert!(!rels.is_image("rId1"));
    }

    #[test]
    fn test_escape_xml_function() {
        assert_eq!(escape_xml("normal text"), "normal text");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(escape_xml("it's"), "it&apos;s");
        assert_eq!(
            escape_xml("a < b & c > d \"e\" 'f'"),
            "a &lt; b &amp; c &gt; d &quot;e&quot; &apos;f&apos;"
        );
    }

    #[test]
    fn test_parse_with_target_mode_internal() {
        // Internal links don't have TargetMode attribute
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
            <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
        </Relationships>"#;

        let rels = Relationships::parse(xml).unwrap();
        // Internal targets should still be retrievable
        assert_eq!(rels.get("rId1"), Some("styles.xml"));
    }

    #[test]
    fn test_roundtrip_with_special_characters() {
        let mut rels = Relationships::new();

        // Add relationships with special characters in target
        rels.add("path/with spaces/file.xml".to_string(), "type".to_string());
        rels.add(
            "unicode/日本語/ファイル.xml".to_string(),
            "type".to_string(),
        );

        let xml = rels.to_xml();
        let reparsed = Relationships::parse(xml.as_bytes()).unwrap();

        assert_eq!(reparsed.get("rId1"), Some("path/with spaces/file.xml"));
        assert_eq!(reparsed.get("rId2"), Some("unicode/日本語/ファイル.xml"));
    }

    // ==================== Sprint 9: Additional Edge Cases ====================

    #[test]
    fn test_add_hyperlink_various_protocols() {
        let mut rels = Relationships::new();

        // Standard HTTP
        let http_id = rels.add_hyperlink("https://example.com/page");
        assert!(rels.is_hyperlink(&http_id));

        // FTP protocol
        let ftp_id = rels.add_hyperlink("ftp://files.example.com/download.zip");
        assert!(rels.is_hyperlink(&ftp_id));

        // Mailto link
        let mail_id = rels.add_hyperlink("mailto:user@example.com");
        assert!(rels.is_hyperlink(&mail_id));

        // Verify all are retrievable
        assert_eq!(rels.get(&http_id), Some("https://example.com/page"));
        assert_eq!(
            rels.get(&ftp_id),
            Some("ftp://files.example.com/download.zip")
        );
        assert_eq!(rels.get(&mail_id), Some("mailto:user@example.com"));
    }

    #[test]
    fn test_add_image_various_paths() {
        let mut rels = Relationships::new();

        // Standard media path
        let id1 = rels.add_image("media/image1.png");
        assert_eq!(rels.get(&id1), Some("media/image1.png"));

        // Relative path with ..
        let id2 = rels.add_image("../media/image2.jpg");
        assert_eq!(rels.get(&id2), Some("../media/image2.jpg"));

        // Path with spaces
        let id3 = rels.add_image("media/my image.png");
        assert_eq!(rels.get(&id3), Some("media/my image.png"));

        // All should be images
        assert!(rels.is_image(&id1));
        assert!(rels.is_image(&id2));
        assert!(rels.is_image(&id3));
    }

    #[test]
    fn test_hyperlink_with_query_params() {
        let mut rels = Relationships::new();

        // URL with query parameters containing special chars
        let url = "https://example.com/search?q=test&page=1&filter=a<b";
        let id = rels.add_hyperlink(url);

        // Should store the URL as-is
        assert_eq!(rels.get(&id), Some(url));

        // Should serialize and parse correctly
        let xml = rels.to_xml();
        let reparsed = Relationships::parse(xml.as_bytes()).unwrap();
        assert_eq!(reparsed.get(&id), Some(url));
    }

    #[test]
    fn test_relationship_type_constants() {
        // Verify all type constants are valid URIs
        assert!(Relationships::TYPE_STYLES.starts_with("http://"));
        assert!(Relationships::TYPE_NUMBERING.starts_with("http://"));
        assert!(Relationships::TYPE_SETTINGS.starts_with("http://"));
        assert!(Relationships::TYPE_FONT_TABLE.starts_with("http://"));
        assert!(Relationships::TYPE_IMAGE.starts_with("http://"));
        assert!(Relationships::TYPE_HYPERLINK.starts_with("http://"));

        // Verify constants are unique
        let types = [
            Relationships::TYPE_STYLES,
            Relationships::TYPE_NUMBERING,
            Relationships::TYPE_SETTINGS,
            Relationships::TYPE_FONT_TABLE,
            Relationships::TYPE_IMAGE,
            Relationships::TYPE_HYPERLINK,
        ];
        let unique: std::collections::HashSet<_> = types.iter().collect();
        assert_eq!(
            unique.len(),
            types.len(),
            "All type constants should be unique"
        );
    }

    #[test]
    fn test_iter_with_type_filter() {
        let mut rels = Relationships::new();

        rels.add_image("media/img1.png");
        rels.add_image("media/img2.jpg");
        rels.add_hyperlink("https://example.com");
        rels.add(
            "styles.xml".to_string(),
            Relationships::TYPE_STYLES.to_string(),
        );

        // Count by type
        let image_count = rels
            .iter()
            .filter(|(_, rel)| rel.rel_type == Relationships::TYPE_IMAGE)
            .count();
        let link_count = rels
            .iter()
            .filter(|(_, rel)| rel.rel_type == Relationships::TYPE_HYPERLINK)
            .count();

        assert_eq!(image_count, 2);
        assert_eq!(link_count, 1);
    }

    #[test]
    fn test_empty_target_handling() {
        let mut rels = Relationships::new();

        // Add relationship with empty target (edge case)
        let id = rels.add(String::new(), "type".to_string());
        assert_eq!(rels.get(&id), Some(""));

        // Should serialize and parse correctly
        let xml = rels.to_xml();
        let reparsed = Relationships::parse(xml.as_bytes()).unwrap();
        assert_eq!(reparsed.get(&id), Some(""));
    }
}
