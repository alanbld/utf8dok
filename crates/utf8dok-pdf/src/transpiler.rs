//! AST to Typst markup transpiler
//!
//! Converts utf8dok AST nodes to Typst markup strings.

use utf8dok_ast::{
    AdmonitionType, Block, Document, FormatType, Inline, List, ListItem, ListType, Table, TableRow,
};

/// Transpiler for converting AST to Typst markup
pub struct Transpiler;

impl Transpiler {
    /// Transpile a document to Typst markup
    pub fn transpile(doc: &Document) -> String {
        let mut output = String::new();

        // Add document metadata if present
        if let Some(ref title) = doc.metadata.title {
            output.push_str(&format!(
                "#set document(title: \"{}\")\n",
                escape_string(title)
            ));
        }

        // Process blocks
        for block in &doc.blocks {
            output.push_str(&Self::transpile_block(block));
            output.push('\n');
        }

        output
    }

    /// Transpile with a template import
    pub fn transpile_with_template(doc: &Document, template_path: &str) -> String {
        let mut output = String::new();

        // Import template
        output.push_str(&format!("#import \"{}\": template\n", template_path));

        // Apply template with metadata
        let title = doc.metadata.title.as_deref().unwrap_or("Untitled");
        let author = doc
            .metadata
            .attributes
            .get("author")
            .map(|s| s.as_str())
            .unwrap_or("");

        output.push_str(&format!(
            "#show: template.with(title: \"{}\", author: \"{}\")\n\n",
            escape_string(title),
            escape_string(author)
        ));

        // Process blocks
        for block in &doc.blocks {
            output.push_str(&Self::transpile_block(block));
            output.push('\n');
        }

        output
    }

    /// Transpile a single block
    fn transpile_block(block: &Block) -> String {
        match block {
            Block::Heading(h) => {
                let prefix = "=".repeat(h.level as usize + 1);
                let text = Self::transpile_inlines(&h.text);
                format!("{} {}\n", prefix, text)
            }

            Block::Paragraph(p) => {
                let text = Self::transpile_inlines(&p.inlines);
                format!("{}\n", text)
            }

            Block::List(list) => Self::transpile_list(list),

            Block::Literal(code) => {
                let lang = code.language.as_deref().unwrap_or("");
                if lang.is_empty() {
                    format!("```\n{}\n```\n", code.content)
                } else {
                    format!("```{}\n{}\n```\n", lang, code.content)
                }
            }

            Block::Table(table) => Self::transpile_table(table),

            Block::Quote(quote) => {
                let mut inner = String::new();
                for block in &quote.blocks {
                    inner.push_str(&Self::transpile_block(block));
                }
                let attribution = quote
                    .attribution
                    .as_ref()
                    .map(|a| format!(", attribution: [{}]", a))
                    .unwrap_or_default();
                format!("#quote(block: true{})[{}]\n", attribution, inner.trim())
            }

            Block::ThematicBreak => "#line(length: 100%)\n".to_string(),

            Block::Admonition(admon) => {
                let kind = match admon.admonition_type {
                    AdmonitionType::Note => "Note",
                    AdmonitionType::Tip => "Tip",
                    AdmonitionType::Important => "Important",
                    AdmonitionType::Warning => "Warning",
                    AdmonitionType::Caution => "Caution",
                };
                let mut content = String::new();
                for block in &admon.content {
                    content.push_str(&Self::transpile_block(block));
                }
                format!(
                    "#block(fill: luma(240), inset: 8pt, radius: 4pt)[\n  *{}:* {}\n]\n",
                    kind,
                    content.trim()
                )
            }

            Block::Break(_) => "#pagebreak()\n".to_string(),

            Block::Open(open) => {
                let mut inner = String::new();
                for block in &open.blocks {
                    inner.push_str(&Self::transpile_block(block));
                }
                inner
            }

            Block::Sidebar(sidebar) => {
                let mut inner = String::new();
                for block in &sidebar.blocks {
                    inner.push_str(&Self::transpile_block(block));
                }
                format!(
                    "#block(fill: luma(230), inset: 12pt, radius: 4pt)[\n{}\n]\n",
                    inner.trim()
                )
            }
        }
    }

    /// Transpile a list
    fn transpile_list(list: &List) -> String {
        let ordered = matches!(list.list_type, ListType::Ordered);
        Self::transpile_list_items(&list.items, ordered, 0)
    }

    /// Transpile list items with nesting support
    fn transpile_list_items(items: &[ListItem], ordered: bool, indent_level: usize) -> String {
        let mut output = String::new();
        let indent = "  ".repeat(indent_level);

        for (i, item) in items.iter().enumerate() {
            let marker = if ordered {
                format!("{}. ", i + 1)
            } else {
                "- ".to_string()
            };

            // Get first block content as text
            let content = Self::transpile_blocks(&item.content);
            output.push_str(&format!("{}{}{}\n", indent, marker, content.trim()));
        }

        output
    }

    /// Transpile multiple blocks
    fn transpile_blocks(blocks: &[Block]) -> String {
        let mut output = String::new();
        for block in blocks {
            output.push_str(&Self::transpile_block(block));
        }
        output
    }

    /// Transpile a table
    fn transpile_table(table: &Table) -> String {
        let mut output = String::new();

        let col_count = table
            .columns
            .len()
            .max(table.rows.first().map(|r| r.cells.len()).unwrap_or(0));

        // Start table
        output.push_str(&format!("#table(\n  columns: {},\n", col_count));

        // Add rows
        for row in &table.rows {
            output.push_str(&Self::transpile_table_row(row));
        }

        output.push_str(")\n");
        output
    }

    /// Transpile a table row
    fn transpile_table_row(row: &TableRow) -> String {
        let mut output = String::new();

        for cell in &row.cells {
            let content = Self::transpile_blocks(&cell.content);
            if row.is_header {
                output.push_str(&format!("  table.header[{}],\n", content.trim()));
            } else {
                output.push_str(&format!("  [{}],\n", content.trim()));
            }
        }

        output
    }

    /// Transpile inline elements
    fn transpile_inlines(inlines: &[Inline]) -> String {
        let mut output = String::new();

        for inline in inlines {
            output.push_str(&Self::transpile_inline(inline));
        }

        output
    }

    /// Transpile a single inline element
    fn transpile_inline(inline: &Inline) -> String {
        match inline {
            Inline::Text(text) => text.clone(),

            Inline::Format(format_type, content) => {
                let inner = Self::transpile_inline(content);
                match format_type {
                    FormatType::Bold => format!("*{}*", inner),
                    FormatType::Italic => format!("_{}_", inner),
                    FormatType::Monospace => format!("`{}`", inner),
                    FormatType::Highlight => format!("#highlight[{}]", inner),
                    FormatType::Superscript => format!("#super[{}]", inner),
                    FormatType::Subscript => format!("#sub[{}]", inner),
                }
            }

            Inline::Span(inlines) => Self::transpile_inlines(inlines),

            Inline::Link(link) => {
                let text = Self::transpile_inlines(&link.text);
                if link.url.starts_with('#') {
                    // Internal reference
                    let label = &link.url[1..];
                    format!("@{}", label)
                } else {
                    format!("#link(\"{}\")[{}]", link.url, text)
                }
            }

            Inline::Image(img) => {
                if let Some(ref alt) = img.alt {
                    format!("#figure(image(\"{}\"), caption: [{}])", img.src, alt)
                } else {
                    format!("#image(\"{}\")", img.src)
                }
            }

            Inline::Break => " \\\n".to_string(),

            Inline::Anchor(id) => format!("<{}>", id),
        }
    }
}

/// Escape special characters in strings for Typst
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('#', "\\#")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use utf8dok_ast::{Heading, Paragraph};

    #[test]
    fn test_transpile_heading() {
        let mut doc = Document::new();
        doc.push(Block::Heading(Heading {
            level: 1,
            text: vec![Inline::Text("Hello World".to_string())],
            style_id: None,
            anchor: None,
        }));

        let typst = Transpiler::transpile(&doc);
        assert!(typst.contains("== Hello World"));
    }

    #[test]
    fn test_transpile_paragraph() {
        let mut doc = Document::new();
        doc.push(Block::Paragraph(Paragraph {
            inlines: vec![Inline::Text("This is a paragraph.".to_string())],
            style_id: None,
            attributes: HashMap::new(),
        }));

        let typst = Transpiler::transpile(&doc);
        assert!(typst.contains("This is a paragraph."));
    }

    #[test]
    fn test_transpile_bold() {
        let inline = Inline::Format(
            FormatType::Bold,
            Box::new(Inline::Text("bold text".to_string())),
        );
        let result = Transpiler::transpile_inline(&inline);
        assert_eq!(result, "*bold text*");
    }

    #[test]
    fn test_transpile_italic() {
        let inline = Inline::Format(
            FormatType::Italic,
            Box::new(Inline::Text("italic".to_string())),
        );
        let result = Transpiler::transpile_inline(&inline);
        assert_eq!(result, "_italic_");
    }

    #[test]
    fn test_transpile_code_inline() {
        let inline = Inline::Format(
            FormatType::Monospace,
            Box::new(Inline::Text("println!()".to_string())),
        );
        let result = Transpiler::transpile_inline(&inline);
        assert_eq!(result, "`println!()`");
    }

    #[test]
    fn test_transpile_link() {
        let inline = Inline::Link(utf8dok_ast::Link {
            url: "https://example.com".to_string(),
            text: vec![Inline::Text("Example".to_string())],
        });
        let result = Transpiler::transpile_inline(&inline);
        assert!(result.contains("#link(\"https://example.com\")[Example]"));
    }

    #[test]
    fn test_transpile_internal_link() {
        let inline = Inline::Link(utf8dok_ast::Link {
            url: "#section1".to_string(),
            text: vec![Inline::Text("Section 1".to_string())],
        });
        let result = Transpiler::transpile_inline(&inline);
        assert_eq!(result, "@section1");
    }

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_string("#heading"), "\\#heading");
    }
}
