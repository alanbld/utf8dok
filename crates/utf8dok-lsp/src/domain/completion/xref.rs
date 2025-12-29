//! Cross-reference completion
//!
//! Provides completion for <<section-id>> references.

use regex::Regex;
use std::sync::OnceLock;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, MarkupContent, MarkupKind,
};

/// Section info extracted from document
#[derive(Debug, Clone)]
struct SectionInfo {
    id: String,
    title: String,
    level: usize,
    line: usize,
}

/// Cross-reference completer
pub struct XrefCompleter;

impl XrefCompleter {
    pub fn new() -> Self {
        Self
    }

    /// Complete xrefs based on document sections
    pub fn complete(&self, text: &str, prefix: &str) -> Vec<CompletionItem> {
        let sections = self.extract_sections(text);

        sections
            .into_iter()
            .filter(|s| prefix.is_empty() || s.id.starts_with(prefix))
            .map(|s| self.section_to_completion(&s))
            .collect()
    }

    /// Extract all sections with IDs from the document
    fn extract_sections(&self, text: &str) -> Vec<SectionInfo> {
        let mut sections = Vec::new();
        let lines: Vec<&str> = text.lines().collect();

        // Regex for [[id]] anchors
        static ANCHOR_RE: OnceLock<Regex> = OnceLock::new();
        let anchor_re = ANCHOR_RE.get_or_init(|| Regex::new(r"^\[\[([\w\-]+)\]\]").unwrap());

        // Regex for headers
        static HEADER_RE: OnceLock<Regex> = OnceLock::new();
        let header_re = HEADER_RE.get_or_init(|| Regex::new(r"^(=+)\s+(.+)$").unwrap());

        let mut pending_anchor: Option<(String, usize)> = None;

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check for anchor [[id]]
            if let Some(cap) = anchor_re.captures(trimmed) {
                let id = cap.get(1).unwrap().as_str().to_string();
                pending_anchor = Some((id, line_num));
                continue;
            }

            // Check for header
            if let Some(cap) = header_re.captures(trimmed) {
                let level = cap.get(1).unwrap().as_str().len();
                let title = cap.get(2).unwrap().as_str().to_string();

                // Use pending anchor or generate ID from title
                let (id, anchor_line) =
                    if let Some((anchor_id, anchor_line)) = pending_anchor.take() {
                        (anchor_id, anchor_line)
                    } else {
                        // Generate ID from title (simplified)
                        let id = self.title_to_id(&title);
                        (id, line_num)
                    };

                sections.push(SectionInfo {
                    id,
                    title,
                    level,
                    line: anchor_line,
                });
            } else {
                // Non-header line clears pending anchor
                if !trimmed.is_empty() && pending_anchor.is_some() {
                    // Standalone anchor without header - still add it
                    if let Some((id, line)) = pending_anchor.take() {
                        sections.push(SectionInfo {
                            id: id.clone(),
                            title: format!("[{}]", id),
                            level: 0,
                            line,
                        });
                    }
                }
            }
        }

        // Handle trailing anchor
        if let Some((id, line)) = pending_anchor {
            sections.push(SectionInfo {
                id: id.clone(),
                title: format!("[{}]", id),
                level: 0,
                line,
            });
        }

        sections
    }

    /// Generate an ID from a title
    fn title_to_id(&self, title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Convert section info to completion item
    fn section_to_completion(&self, section: &SectionInfo) -> CompletionItem {
        let level_indicator = "=".repeat(section.level.max(1));

        CompletionItem {
            label: section.id.clone(),
            kind: Some(CompletionItemKind::REFERENCE),
            detail: Some(format!("{} {}", level_indicator, section.title)),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "**Section:** {}\n\n*Line {}*",
                    section.title,
                    section.line + 1
                ),
            })),
            insert_text: Some(format!("{}>>", section.id)),
            ..Default::default()
        }
    }
}

impl Default for XrefCompleter {
    fn default() -> Self {
        Self::new()
    }
}
