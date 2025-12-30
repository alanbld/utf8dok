//! Parser for dual-nature annotations in AsciiDoc content

use regex::Regex;
use std::sync::OnceLock;

use super::types::*;

/// Parser for dual-nature content annotations
pub struct DualNatureParser;

impl DualNatureParser {
    /// Parse content with dual-nature annotations
    pub fn parse(content: &str) -> DualNatureDocument {
        let mut doc = DualNatureDocument::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        // Parse document title and attributes
        while i < lines.len() {
            let line = lines[i].trim();

            // Document title
            if line.starts_with("= ") && doc.title.is_none() {
                doc.title = Some(line[2..].to_string());
                i += 1;
                continue;
            }

            // Document attributes
            if line.starts_with(':') && line.contains(':') {
                if let Some((name, value)) = Self::parse_attribute(line) {
                    Self::apply_attribute(&mut doc.attributes, &name, &value);
                }
                i += 1;
                continue;
            }

            // Empty line or content starts
            if line.is_empty() {
                i += 1;
                continue;
            }

            break;
        }

        // Parse content blocks
        while i < lines.len() {
            let (block, consumed) = Self::parse_block(&lines, i);
            if let Some(b) = block {
                doc.blocks.push(b);
            }
            i += consumed.max(1);
        }

        doc
    }

    /// Parse a document attribute line
    fn parse_attribute(line: &str) -> Option<(String, String)> {
        static ATTR_RE: OnceLock<Regex> = OnceLock::new();
        let re = ATTR_RE.get_or_init(|| Regex::new(r"^:([\w\-]+):\s*(.*)$").unwrap());

        re.captures(line).map(|cap| {
            (
                cap.get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
                cap.get(2)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default(),
            )
        })
    }

    /// Apply an attribute to the document attributes
    fn apply_attribute(attrs: &mut DocumentAttributes, name: &str, value: &str) {
        match name {
            "author" => attrs.author = Some(value.to_string()),
            "date" => attrs.date = Some(value.to_string()),
            "template" => attrs.slide.template = Some(value.to_string()),
            "slide-master" => attrs.slide.slide_master = Some(value.to_string()),
            "slide-layout" => attrs.slide.default_layout = Some(value.to_string()),
            "slide-bullets" => {
                attrs.slide.default_bullets = value.parse().ok();
            }
            "document-template" => attrs.document.template = Some(value.to_string()),
            "document-style" => attrs.document.default_style = Some(value.to_string()),
            _ => {
                attrs.generic.insert(name.to_string(), value.to_string());
            }
        }
    }

    /// Parse a content block starting at the given line
    fn parse_block(lines: &[&str], start: usize) -> (Option<DualNatureBlock>, usize) {
        if start >= lines.len() {
            return (None, 0);
        }

        let line = lines[start].trim();

        // Empty line
        if line.is_empty() {
            return (None, 1);
        }

        // Annotation block [.selector]
        if line.starts_with("[.") && line.ends_with(']') {
            return Self::parse_annotated_block(lines, start);
        }

        // Section heading
        if line.starts_with("==") || line.starts_with("= ") {
            return Self::parse_section(lines, start, ContentSelector::Both);
        }

        // Bullet list
        if line.starts_with("* ") || line.starts_with("- ") {
            return Self::parse_bullet_list(lines, start, ContentSelector::Both);
        }

        // Numbered list
        if line.starts_with(". ")
            || line
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            return Self::parse_numbered_list(lines, start, ContentSelector::Both);
        }

        // Code block
        if line.starts_with("----") || line.starts_with("```") {
            return Self::parse_code_block(lines, start, ContentSelector::Both);
        }

        // Image
        if line.starts_with("image::") {
            return Self::parse_image(lines, start, ContentSelector::Both);
        }

        // Include
        if line.starts_with("include::") {
            return Self::parse_include(lines, start, ContentSelector::Both);
        }

        // Regular paragraph
        Self::parse_paragraph(lines, start, ContentSelector::Both)
    }

    /// Parse an annotated block [.selector]
    fn parse_annotated_block(lines: &[&str], start: usize) -> (Option<DualNatureBlock>, usize) {
        let annotation_line = lines[start].trim();

        // Extract selector from [.selector]
        let selector_str = &annotation_line[1..annotation_line.len() - 1];
        let selector = ContentSelector::from_annotation(selector_str);

        // Parse the content after the annotation
        let content_start = start + 1;
        if content_start >= lines.len() {
            return (None, 1);
        }

        let content_line = lines[content_start].trim();

        // Section heading after annotation
        if content_line.starts_with("==") || content_line.starts_with("= ") {
            let (block, consumed) = Self::parse_section(lines, content_start, selector);
            return (block, consumed + 1);
        }

        // Bullet list after annotation
        if content_line.starts_with("* ") || content_line.starts_with("- ") {
            let (block, consumed) = Self::parse_bullet_list(lines, content_start, selector);
            return (block, consumed + 1);
        }

        // Paragraph after annotation
        let (block, consumed) = Self::parse_paragraph(lines, content_start, selector);
        (block, consumed + 1)
    }

    /// Parse a section heading
    fn parse_section(
        lines: &[&str],
        start: usize,
        selector: ContentSelector,
    ) -> (Option<DualNatureBlock>, usize) {
        let line = lines[start].trim();

        // Count the '=' signs for level
        let level = line.chars().take_while(|&c| c == '=').count();
        let title = line[level..].trim().to_string();

        // Check for block-level overrides on following lines
        let mut overrides = BlockOverrides::default();
        let mut consumed = 1;

        while start + consumed < lines.len() {
            let next_line = lines[start + consumed].trim();
            if next_line.starts_with(':') {
                if let Some((name, value)) = Self::parse_attribute(next_line) {
                    match name.as_str() {
                        "slide-layout" => overrides.slide_layout = Some(value),
                        "slide-bullets" => overrides.slide_bullets = value.parse().ok(),
                        "slide-style" => overrides.slide_style = Some(value),
                        "document-style" => overrides.document_style = Some(value),
                        _ => {}
                    }
                    consumed += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let block = DualNatureBlock {
            selector,
            content: BlockContent::Section(SectionContent {
                level,
                title,
                id: None,
                children: Vec::new(),
            }),
            overrides,
            source_line: start + 1,
        };

        (Some(block), consumed)
    }

    /// Parse a bullet list
    fn parse_bullet_list(
        lines: &[&str],
        start: usize,
        selector: ContentSelector,
    ) -> (Option<DualNatureBlock>, usize) {
        let mut items = Vec::new();
        let mut i = start;

        while i < lines.len() {
            let line = lines[i].trim();
            if let Some(stripped) = line.strip_prefix("* ").or_else(|| line.strip_prefix("- ")) {
                items.push(stripped.to_string());
                i += 1;
            } else if line.is_empty() {
                // Allow one empty line within list
                if i + 1 < lines.len() {
                    let next = lines[i + 1].trim();
                    if next.starts_with("* ") || next.starts_with("- ") {
                        i += 1;
                        continue;
                    }
                }
                break;
            } else {
                break;
            }
        }

        if items.is_empty() {
            return (None, 1);
        }

        let block = DualNatureBlock {
            selector,
            content: BlockContent::BulletList(items),
            overrides: BlockOverrides::default(),
            source_line: start + 1,
        };

        (Some(block), i - start)
    }

    /// Parse a numbered list
    fn parse_numbered_list(
        lines: &[&str],
        start: usize,
        selector: ContentSelector,
    ) -> (Option<DualNatureBlock>, usize) {
        let mut items = Vec::new();
        let mut i = start;

        while i < lines.len() {
            let line = lines[i].trim();
            if let Some(stripped) = line.strip_prefix(". ") {
                items.push(stripped.to_string());
                i += 1;
            } else if line.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                // Handle "1. item" format
                if let Some(pos) = line.find(". ") {
                    items.push(line[pos + 2..].to_string());
                    i += 1;
                } else {
                    break;
                }
            } else {
                // Empty or non-list line
                break;
            }
        }

        if items.is_empty() {
            return (None, 1);
        }

        let block = DualNatureBlock {
            selector,
            content: BlockContent::NumberedList(items),
            overrides: BlockOverrides::default(),
            source_line: start + 1,
        };

        (Some(block), i - start)
    }

    /// Parse a code block
    fn parse_code_block(
        lines: &[&str],
        start: usize,
        selector: ContentSelector,
    ) -> (Option<DualNatureBlock>, usize) {
        let delimiter = lines[start].trim();
        let is_fenced = delimiter.starts_with("```");
        let language = if is_fenced && delimiter.len() > 3 {
            Some(delimiter[3..].trim().to_string())
        } else {
            None
        };

        let mut code_lines = Vec::new();
        let mut i = start + 1;

        while i < lines.len() {
            let line = lines[i];
            if line.trim() == delimiter || (is_fenced && line.trim() == "```") {
                i += 1;
                break;
            }
            code_lines.push(line);
            i += 1;
        }

        let block = DualNatureBlock {
            selector,
            content: BlockContent::Code(CodeContent {
                language,
                code: code_lines.join("\n"),
                caption: None,
            }),
            overrides: BlockOverrides::default(),
            source_line: start + 1,
        };

        (Some(block), i - start)
    }

    /// Parse an image
    fn parse_image(
        lines: &[&str],
        start: usize,
        selector: ContentSelector,
    ) -> (Option<DualNatureBlock>, usize) {
        static IMG_RE: OnceLock<Regex> = OnceLock::new();
        let re = IMG_RE.get_or_init(|| Regex::new(r"image::([^\[]+)\[([^\]]*)\]").unwrap());

        let line = lines[start].trim();
        if let Some(cap) = re.captures(line) {
            let path = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let attrs = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Parse attributes like width=70%, alt="description"
            let mut width = None;
            let mut alt = None;

            for attr in attrs.split(',') {
                let attr = attr.trim();
                if let Some(w) = attr.strip_prefix("width=") {
                    width = Some(w.to_string());
                } else if let Some(a) = attr.strip_prefix("alt=") {
                    alt = Some(a.trim_matches('"').to_string());
                } else if !attr.contains('=') && alt.is_none() {
                    alt = Some(attr.to_string());
                }
            }

            let block = DualNatureBlock {
                selector,
                content: BlockContent::Image(ImageContent {
                    path,
                    alt,
                    width,
                    height: None,
                    caption: None,
                    slide_path: None,
                }),
                overrides: BlockOverrides::default(),
                source_line: start + 1,
            };

            return (Some(block), 1);
        }

        (None, 1)
    }

    /// Parse an include directive
    fn parse_include(
        lines: &[&str],
        start: usize,
        selector: ContentSelector,
    ) -> (Option<DualNatureBlock>, usize) {
        static INC_RE: OnceLock<Regex> = OnceLock::new();
        let re = INC_RE.get_or_init(|| Regex::new(r"include::([^\[]+)\[([^\]]*)\]").unwrap());

        let line = lines[start].trim();
        if let Some(cap) = re.captures(line) {
            let path = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            let block = DualNatureBlock {
                selector,
                content: BlockContent::Include(IncludeContent {
                    path,
                    lines: None,
                    tag: None,
                }),
                overrides: BlockOverrides::default(),
                source_line: start + 1,
            };

            return (Some(block), 1);
        }

        (None, 1)
    }

    /// Parse a paragraph
    fn parse_paragraph(
        lines: &[&str],
        start: usize,
        selector: ContentSelector,
    ) -> (Option<DualNatureBlock>, usize) {
        let mut para_lines = Vec::new();
        let mut i = start;

        while i < lines.len() {
            let line = lines[i].trim();

            // End paragraph on empty line or special content
            if line.is_empty()
                || line.starts_with("==")
                || line.starts_with("* ")
                || line.starts_with("- ")
                || line.starts_with(". ")
                || line.starts_with("[.")
                || line.starts_with("----")
                || line.starts_with("```")
                || line.starts_with("image::")
                || line.starts_with("include::")
            {
                break;
            }

            para_lines.push(line);
            i += 1;
        }

        if para_lines.is_empty() {
            return (None, 1);
        }

        let block = DualNatureBlock {
            selector,
            content: BlockContent::Paragraph(para_lines.join(" ")),
            overrides: BlockOverrides::default(),
            source_line: start + 1,
        };

        (Some(block), i - start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_document_title() {
        let content = "= My Document\n:author: Jane Doe";
        let doc = DualNatureParser::parse(content);
        assert_eq!(doc.title, Some("My Document".to_string()));
        assert_eq!(doc.attributes.author, Some("Jane Doe".to_string()));
    }

    #[test]
    fn test_parse_slide_attributes() {
        let content = r#"= Presentation
:slide-master: Executive-Deck
:slide-layout: Title-And-Content
:slide-bullets: 3
"#;
        let doc = DualNatureParser::parse(content);
        assert_eq!(
            doc.attributes.slide.slide_master,
            Some("Executive-Deck".to_string())
        );
        assert_eq!(
            doc.attributes.slide.default_layout,
            Some("Title-And-Content".to_string())
        );
        assert_eq!(doc.attributes.slide.default_bullets, Some(3));
    }

    #[test]
    fn test_parse_annotated_section() {
        let content = r#"= Title

[.slide]
== Slide Section
:slide-layout: Two-Column

Some content here.
"#;
        let doc = DualNatureParser::parse(content);

        let slide_blocks: Vec<_> = doc
            .blocks
            .iter()
            .filter(|b| matches!(b.selector, ContentSelector::Slide))
            .collect();

        assert!(!slide_blocks.is_empty());
        if let BlockContent::Section(section) = &slide_blocks[0].content {
            assert_eq!(section.title, "Slide Section");
        }
        assert_eq!(
            slide_blocks[0].overrides.slide_layout,
            Some("Two-Column".to_string())
        );
    }

    #[test]
    fn test_parse_bullet_list() {
        let content = r#"= Title

* Item 1
* Item 2
* Item 3
"#;
        let doc = DualNatureParser::parse(content);

        let list_block = doc
            .blocks
            .iter()
            .find(|b| matches!(b.content, BlockContent::BulletList(_)));

        assert!(list_block.is_some());
        if let BlockContent::BulletList(items) = &list_block.unwrap().content {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], "Item 1");
        }
    }

    #[test]
    fn test_parse_document_only_block() {
        let content = r#"= Title

[.document-only]
== Detailed Analysis

This is detailed content.
"#;
        let doc = DualNatureParser::parse(content);

        let doc_only: Vec<_> = doc
            .blocks
            .iter()
            .filter(|b| matches!(b.selector, ContentSelector::DocumentOnly))
            .collect();

        assert!(!doc_only.is_empty());
    }

    #[test]
    fn test_parse_code_block() {
        let content = r#"= Title

```rust
fn main() {
    println!("Hello");
}
```
"#;
        let doc = DualNatureParser::parse(content);

        let code_block = doc
            .blocks
            .iter()
            .find(|b| matches!(b.content, BlockContent::Code(_)));

        assert!(code_block.is_some());
        if let BlockContent::Code(code) = &code_block.unwrap().content {
            assert_eq!(code.language, Some("rust".to_string()));
            assert!(code.code.contains("println!"));
        }
    }

    #[test]
    fn test_parse_image() {
        let content = r#"= Title

image::diagram.png[Architecture Overview, width=70%]
"#;
        let doc = DualNatureParser::parse(content);

        let img_block = doc
            .blocks
            .iter()
            .find(|b| matches!(b.content, BlockContent::Image(_)));

        assert!(img_block.is_some());
        if let BlockContent::Image(img) = &img_block.unwrap().content {
            assert_eq!(img.path, "diagram.png");
            assert_eq!(img.width, Some("70%".to_string()));
        }
    }
}
