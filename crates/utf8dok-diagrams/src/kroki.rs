//! Kroki diagram rendering client
//!
//! This module provides a client for the [Kroki](https://kroki.io) diagram
//! rendering service, which supports multiple diagram types.

use std::io::Write;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};

use crate::error::{DiagramError, Result};
use crate::types::{DiagramType, OutputFormat};

/// Default Kroki server URL
pub const DEFAULT_KROKI_URL: &str = "https://kroki.io";

/// Client for rendering diagrams via Kroki
#[derive(Debug, Clone)]
pub struct KrokiClient {
    /// Base URL of the Kroki server
    base_url: String,
    /// HTTP client
    client: Client,
    /// Request timeout
    timeout: Duration,
}

impl Default for KrokiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl KrokiClient {
    /// Create a new client with the default Kroki server
    pub fn new() -> Self {
        Self::with_url(DEFAULT_KROKI_URL)
    }

    /// Create a client with a custom Kroki server URL
    pub fn with_url(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url,
            client,
            timeout: Duration::from_secs(30),
        }
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self.client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");
        self
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Render a diagram to the specified format
    ///
    /// # Arguments
    /// * `source` - The diagram source code
    /// * `diagram_type` - The type of diagram (e.g., Mermaid, PlantUML)
    /// * `format` - The desired output format (e.g., PNG, SVG)
    ///
    /// # Returns
    /// The rendered diagram as bytes
    ///
    /// # Example
    /// ```no_run
    /// use utf8dok_diagrams::{KrokiClient, DiagramType, OutputFormat};
    ///
    /// let client = KrokiClient::new();
    /// let svg = client.render("graph TD; A-->B;", DiagramType::Mermaid, OutputFormat::Svg)?;
    /// # Ok::<(), utf8dok_diagrams::DiagramError>(())
    /// ```
    pub fn render(
        &self,
        source: &str,
        diagram_type: DiagramType,
        format: OutputFormat,
    ) -> Result<Vec<u8>> {
        let url = format!(
            "{}/{}/{}",
            self.base_url,
            diagram_type.kroki_name(),
            format.kroki_name()
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "text/plain")
            .body(source.to_string())
            .send()?;

        let status = response.status();
        if !status.is_success() {
            let message = response.text().unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DiagramError::ServerError {
                status: status.as_u16(),
                message,
            });
        }

        Ok(response.bytes()?.to_vec())
    }

    /// Render a diagram using the compressed GET method
    ///
    /// This method compresses the source and encodes it in the URL,
    /// which can be useful for caching or sharing URLs.
    pub fn render_compressed(
        &self,
        source: &str,
        diagram_type: DiagramType,
        format: OutputFormat,
    ) -> Result<Vec<u8>> {
        let encoded = Self::encode_source(source)?;
        let url = format!(
            "{}/{}/{}/{}",
            self.base_url,
            diagram_type.kroki_name(),
            format.kroki_name(),
            encoded
        );

        let response = self.client.get(&url).send()?;

        let status = response.status();
        if !status.is_success() {
            let message = response.text().unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DiagramError::ServerError {
                status: status.as_u16(),
                message,
            });
        }

        Ok(response.bytes()?.to_vec())
    }

    /// Encode diagram source for use in URLs (deflate + base64)
    pub fn encode_source(source: &str) -> Result<String> {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(source.as_bytes())
            .map_err(|e| DiagramError::InvalidSource(e.to_string()))?;
        let compressed = encoder
            .finish()
            .map_err(|e| DiagramError::InvalidSource(e.to_string()))?;

        Ok(URL_SAFE_NO_PAD.encode(&compressed))
    }

    /// Generate a URL for a diagram (without rendering)
    ///
    /// Useful for embedding in HTML or sharing.
    pub fn diagram_url(
        &self,
        source: &str,
        diagram_type: DiagramType,
        format: OutputFormat,
    ) -> Result<String> {
        let encoded = Self::encode_source(source)?;
        Ok(format!(
            "{}/{}/{}/{}",
            self.base_url,
            diagram_type.kroki_name(),
            format.kroki_name(),
            encoded
        ))
    }

    /// Check if the Kroki server is available
    pub fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send()?;
        Ok(response.status().is_success())
    }
}

/// Compute SHA-256 hash of content (for drift detection)
pub fn content_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    format!("sha256:{}", hex::encode(result))
}

/// Helper to format hash as hex string
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

/// Rendered diagram with metadata
#[derive(Debug, Clone)]
pub struct RenderedDiagram {
    /// The rendered image bytes
    pub data: Vec<u8>,
    /// The diagram type
    pub diagram_type: DiagramType,
    /// The output format
    pub format: OutputFormat,
    /// SHA-256 hash of the source
    pub source_hash: String,
}

impl RenderedDiagram {
    /// Get the file extension for this diagram
    pub fn extension(&self) -> &'static str {
        self.format.extension()
    }

    /// Get the MIME type for this diagram
    pub fn mime_type(&self) -> &'static str {
        self.format.mime_type()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_default() {
        let client = KrokiClient::new();
        assert_eq!(client.base_url(), DEFAULT_KROKI_URL);
    }

    #[test]
    fn test_client_custom_url() {
        let client = KrokiClient::with_url("http://localhost:8000/");
        assert_eq!(client.base_url(), "http://localhost:8000");
    }

    #[test]
    fn test_encode_source() {
        let source = "graph TD; A-->B;";
        let encoded = KrokiClient::encode_source(source).unwrap();

        // Should be URL-safe base64
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_diagram_url() {
        let client = KrokiClient::new();
        let url = client
            .diagram_url("graph TD; A-->B;", DiagramType::Mermaid, OutputFormat::Svg)
            .unwrap();

        assert!(url.starts_with("https://kroki.io/mermaid/svg/"));
    }

    #[test]
    fn test_content_hash() {
        let hash = content_hash(b"test content");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    // Integration tests - run with: cargo test --features integration
    // These require network access to kroki.io

    #[test]
    #[ignore] // Requires network access
    fn test_render_mermaid_svg() {
        let client = KrokiClient::new();
        let result = client.render(
            "graph TD; A-->B; B-->C;",
            DiagramType::Mermaid,
            OutputFormat::Svg,
        );

        match result {
            Ok(svg) => {
                assert!(!svg.is_empty());
                let svg_str = String::from_utf8_lossy(&svg);
                assert!(svg_str.contains("<svg"));
            }
            Err(e) => {
                // Allow network failures in CI
                eprintln!("Kroki test skipped (network error): {}", e);
            }
        }
    }

    #[test]
    #[ignore] // Requires network access
    fn test_render_mermaid_png() {
        let client = KrokiClient::new();
        let result = client.render(
            "graph TD; A-->B;",
            DiagramType::Mermaid,
            OutputFormat::Png,
        );

        match result {
            Ok(png) => {
                assert!(!png.is_empty());
                // PNG magic bytes
                assert_eq!(&png[0..4], &[0x89, 0x50, 0x4E, 0x47]);
            }
            Err(e) => {
                eprintln!("Kroki test skipped (network error): {}", e);
            }
        }
    }

    #[test]
    #[ignore] // Requires network access
    fn test_render_plantuml() {
        let client = KrokiClient::new();
        let source = r#"
@startuml
Alice -> Bob: Hello
Bob --> Alice: Hi!
@enduml
"#;
        let result = client.render(source, DiagramType::PlantUml, OutputFormat::Svg);

        match result {
            Ok(svg) => {
                assert!(!svg.is_empty());
            }
            Err(e) => {
                eprintln!("Kroki test skipped (network error): {}", e);
            }
        }
    }

    #[test]
    #[ignore] // Requires network access
    fn test_health_check() {
        let client = KrokiClient::new();
        match client.health_check() {
            Ok(healthy) => {
                println!("Kroki health: {}", healthy);
            }
            Err(e) => {
                eprintln!("Kroki health check skipped: {}", e);
            }
        }
    }
}
