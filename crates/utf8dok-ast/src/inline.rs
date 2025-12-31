//! Inline elements for document content
//!
//! This module defines inline-level elements that appear within blocks,
//! such as text, formatting, links, and images.

use serde::{Deserialize, Serialize};

/// Inline-level content element
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Inline {
    /// Plain text content
    Text(String),
    /// Formatted content (bold, italic, etc.)
    Format(FormatType, Box<Inline>),
    /// A span containing multiple inline elements
    Span(Vec<Inline>),
    /// A hyperlink
    Link(Link),
    /// An inline image
    Image(Image),
    /// A line break
    Break,
    /// An anchor/bookmark (for internal cross-references)
    Anchor(String),
}

/// Text formatting types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FormatType {
    /// Bold text
    Bold,
    /// Italic text
    Italic,
    /// Monospace/code text
    Monospace,
    /// Highlighted text
    Highlight,
    /// Superscript text
    Superscript,
    /// Subscript text
    Subscript,
}

/// A hyperlink element
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    /// The URL target
    pub url: String,
    /// The link text (can contain nested inline elements)
    pub text: Vec<Inline>,
}

/// An image element
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    /// Image source path or URL
    pub src: String,
    /// Alternative text for accessibility
    pub alt: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_inline() {
        let inline = Inline::Text("Hello".to_string());
        assert_eq!(inline, Inline::Text("Hello".to_string()));
    }

    #[test]
    fn test_formatted_text() {
        let bold = Inline::Format(
            FormatType::Bold,
            Box::new(Inline::Text("important".to_string())),
        );
        if let Inline::Format(FormatType::Bold, inner) = bold {
            assert_eq!(*inner, Inline::Text("important".to_string()));
        } else {
            panic!("Expected Bold format");
        }
    }

    #[test]
    fn test_link() {
        let link = Link {
            url: "https://example.com".to_string(),
            text: vec![Inline::Text("Example".to_string())],
        };
        assert_eq!(link.url, "https://example.com");
    }
}
