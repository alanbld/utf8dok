//! Type definitions for diagram rendering
//!
//! This module defines the supported diagram types and output formats.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::DiagramError;

/// Supported diagram types for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagramType {
    /// Mermaid diagrams (flowcharts, sequence, class, etc.)
    Mermaid,
    /// PlantUML diagrams
    PlantUml,
    /// GraphViz DOT language
    GraphViz,
    /// Ditaa ASCII art diagrams
    Ditaa,
    /// BlockDiag block diagrams
    BlockDiag,
    /// SeqDiag sequence diagrams
    SeqDiag,
    /// ActDiag activity diagrams
    ActDiag,
    /// NwDiag network diagrams
    NwDiag,
    /// PacketDiag packet diagrams
    PacketDiag,
    /// RackDiag rack diagrams
    RackDiag,
    /// C4 architecture diagrams (via PlantUML)
    C4PlantUml,
    /// D2 diagrams
    D2,
    /// Structurizr DSL
    Structurizr,
    /// Excalidraw diagrams
    Excalidraw,
    /// Pikchr diagrams
    Pikchr,
    /// Vega visualizations
    Vega,
    /// Vega-Lite visualizations
    VegaLite,
    /// WaveDrom digital timing diagrams
    WaveDrom,
    /// BPMN diagrams
    Bpmn,
    /// Bytefield diagrams
    Bytefield,
    /// ERD entity-relationship diagrams
    Erd,
    /// Nomnoml UML diagrams
    Nomnoml,
    /// Svgbob ASCII art to SVG
    Svgbob,
    /// UMLet diagrams
    Umlet,
    /// WireViz cable/wiring diagrams
    WireViz,
}

impl DiagramType {
    /// Get the Kroki API endpoint name for this diagram type
    pub fn kroki_name(&self) -> &'static str {
        match self {
            Self::Mermaid => "mermaid",
            Self::PlantUml => "plantuml",
            Self::GraphViz => "graphviz",
            Self::Ditaa => "ditaa",
            Self::BlockDiag => "blockdiag",
            Self::SeqDiag => "seqdiag",
            Self::ActDiag => "actdiag",
            Self::NwDiag => "nwdiag",
            Self::PacketDiag => "packetdiag",
            Self::RackDiag => "rackdiag",
            Self::C4PlantUml => "c4plantuml",
            Self::D2 => "d2",
            Self::Structurizr => "structurizr",
            Self::Excalidraw => "excalidraw",
            Self::Pikchr => "pikchr",
            Self::Vega => "vega",
            Self::VegaLite => "vegalite",
            Self::WaveDrom => "wavedrom",
            Self::Bpmn => "bpmn",
            Self::Bytefield => "bytefield",
            Self::Erd => "erd",
            Self::Nomnoml => "nomnoml",
            Self::Svgbob => "svgbob",
            Self::Umlet => "umlet",
            Self::WireViz => "wireviz",
        }
    }

    /// Get common file extensions for this diagram type
    pub fn file_extensions(&self) -> &'static [&'static str] {
        match self {
            Self::Mermaid => &["mmd", "mermaid"],
            Self::PlantUml => &["puml", "plantuml", "pu"],
            Self::GraphViz => &["dot", "gv"],
            Self::Ditaa => &["ditaa"],
            Self::BlockDiag => &["blockdiag"],
            Self::SeqDiag => &["seqdiag"],
            Self::ActDiag => &["actdiag"],
            Self::NwDiag => &["nwdiag"],
            Self::PacketDiag => &["packetdiag"],
            Self::RackDiag => &["rackdiag"],
            Self::C4PlantUml => &["c4puml", "c4"],
            Self::D2 => &["d2"],
            Self::Structurizr => &["dsl"],
            Self::Excalidraw => &["excalidraw"],
            Self::Pikchr => &["pikchr"],
            Self::Vega => &["vg", "vega"],
            Self::VegaLite => &["vl", "vegalite"],
            Self::WaveDrom => &["wavedrom"],
            Self::Bpmn => &["bpmn"],
            Self::Bytefield => &["bytefield"],
            Self::Erd => &["erd"],
            Self::Nomnoml => &["nomnoml"],
            Self::Svgbob => &["svgbob", "bob"],
            Self::Umlet => &["uxf"],
            Self::WireViz => &["wireviz", "yml", "yaml"],
        }
    }

    /// Try to detect diagram type from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        let ext = ext.to_lowercase();
        let ext = ext.trim_start_matches('.');

        for variant in Self::all() {
            if variant.file_extensions().contains(&ext) {
                return Some(*variant);
            }
        }
        None
    }

    /// Get all diagram types
    pub fn all() -> &'static [DiagramType] {
        &[
            Self::Mermaid,
            Self::PlantUml,
            Self::GraphViz,
            Self::Ditaa,
            Self::BlockDiag,
            Self::SeqDiag,
            Self::ActDiag,
            Self::NwDiag,
            Self::PacketDiag,
            Self::RackDiag,
            Self::C4PlantUml,
            Self::D2,
            Self::Structurizr,
            Self::Excalidraw,
            Self::Pikchr,
            Self::Vega,
            Self::VegaLite,
            Self::WaveDrom,
            Self::Bpmn,
            Self::Bytefield,
            Self::Erd,
            Self::Nomnoml,
            Self::Svgbob,
            Self::Umlet,
            Self::WireViz,
        ]
    }
}

impl fmt::Display for DiagramType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kroki_name())
    }
}

impl FromStr for DiagramType {
    type Err = DiagramError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mermaid" | "mmd" => Ok(Self::Mermaid),
            "plantuml" | "puml" => Ok(Self::PlantUml),
            "graphviz" | "dot" => Ok(Self::GraphViz),
            "ditaa" => Ok(Self::Ditaa),
            "blockdiag" => Ok(Self::BlockDiag),
            "seqdiag" => Ok(Self::SeqDiag),
            "actdiag" => Ok(Self::ActDiag),
            "nwdiag" => Ok(Self::NwDiag),
            "packetdiag" => Ok(Self::PacketDiag),
            "rackdiag" => Ok(Self::RackDiag),
            "c4plantuml" | "c4" => Ok(Self::C4PlantUml),
            "d2" => Ok(Self::D2),
            "structurizr" => Ok(Self::Structurizr),
            "excalidraw" => Ok(Self::Excalidraw),
            "pikchr" => Ok(Self::Pikchr),
            "vega" => Ok(Self::Vega),
            "vegalite" => Ok(Self::VegaLite),
            "wavedrom" => Ok(Self::WaveDrom),
            "bpmn" => Ok(Self::Bpmn),
            "bytefield" => Ok(Self::Bytefield),
            "erd" => Ok(Self::Erd),
            "nomnoml" => Ok(Self::Nomnoml),
            "svgbob" => Ok(Self::Svgbob),
            "umlet" => Ok(Self::Umlet),
            "wireviz" => Ok(Self::WireViz),
            _ => Err(DiagramError::UnsupportedType(s.to_string())),
        }
    }
}

/// Output format for rendered diagrams
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// PNG raster image
    #[default]
    Png,
    /// SVG vector image
    Svg,
    /// PDF document
    Pdf,
    /// Base64-encoded PNG (for embedding)
    Base64,
}

impl OutputFormat {
    /// Get the Kroki API format name
    pub fn kroki_name(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Pdf => "pdf",
            Self::Base64 => "base64",
        }
    }

    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Pdf => "pdf",
            Self::Base64 => "b64",
        }
    }

    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Svg => "image/svg+xml",
            Self::Pdf => "application/pdf",
            Self::Base64 => "text/plain",
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kroki_name())
    }
}

impl FromStr for OutputFormat {
    type Err = DiagramError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "png" => Ok(Self::Png),
            "svg" => Ok(Self::Svg),
            "pdf" => Ok(Self::Pdf),
            "base64" | "b64" => Ok(Self::Base64),
            _ => Err(DiagramError::UnsupportedFormat(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagram_type_from_str() {
        assert_eq!("mermaid".parse::<DiagramType>().unwrap(), DiagramType::Mermaid);
        assert_eq!("plantuml".parse::<DiagramType>().unwrap(), DiagramType::PlantUml);
        assert_eq!("dot".parse::<DiagramType>().unwrap(), DiagramType::GraphViz);
        assert!("unknown".parse::<DiagramType>().is_err());
    }

    #[test]
    fn test_diagram_type_display() {
        assert_eq!(DiagramType::Mermaid.to_string(), "mermaid");
        assert_eq!(DiagramType::PlantUml.to_string(), "plantuml");
    }

    #[test]
    fn test_diagram_type_from_extension() {
        assert_eq!(DiagramType::from_extension("mmd"), Some(DiagramType::Mermaid));
        assert_eq!(DiagramType::from_extension(".puml"), Some(DiagramType::PlantUml));
        assert_eq!(DiagramType::from_extension("dot"), Some(DiagramType::GraphViz));
        assert_eq!(DiagramType::from_extension("unknown"), None);
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!("png".parse::<OutputFormat>().unwrap(), OutputFormat::Png);
        assert_eq!("svg".parse::<OutputFormat>().unwrap(), OutputFormat::Svg);
        assert!("unknown".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_output_format_properties() {
        assert_eq!(OutputFormat::Png.extension(), "png");
        assert_eq!(OutputFormat::Svg.mime_type(), "image/svg+xml");
    }
}
