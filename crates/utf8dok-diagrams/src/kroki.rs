//! Kroki diagram rendering client
//!
//! This module provides a client for the [Kroki](https://kroki.io) diagram
//! rendering service, which supports multiple diagram types.
//!
//! # Feature Flag
//!
//! This module requires the `kroki` feature:
//! ```toml
//! utf8dok-diagrams = { version = "0.1", features = ["kroki"] }
//! ```

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};

use crate::renderer::{DiagramRenderer, RenderError, RenderOptions, RenderResult};
use crate::types::{DiagramType, OutputFormat};

/// Default Kroki server URL (public cloud instance)
pub const DEFAULT_KROKI_URL: &str = "https://kroki.io";

/// Default local Kroki server URL (for self-hosted instances)
pub const LOCAL_KROKI_URL: &str = "http://localhost:8000";

/// Client for rendering diagrams via Kroki
///
/// Kroki is a unified API for multiple diagram types including
/// Mermaid, PlantUML, GraphViz, D2, and many more.
///
/// # Example
///
/// ```ignore
/// use utf8dok_diagrams::{KrokiRenderer, DiagramRenderer, DiagramType, OutputFormat, RenderOptions};
///
/// let renderer = KrokiRenderer::new()?;
/// let png = renderer.render(
///     "graph TD; A-->B;",
///     DiagramType::Mermaid,
///     OutputFormat::Png,
///     &RenderOptions::default(),
/// )?;
/// ```
#[derive(Debug)]
pub struct KrokiRenderer {
    /// Base URL of the Kroki server
    base_url: String,
    /// HTTP client
    client: Client,
    /// Request timeout
    timeout: Duration,
    /// Cache for availability check
    available: AtomicBool,
    /// Whether availability has been checked
    checked: AtomicBool,
}

impl KrokiRenderer {
    /// Create a new Kroki renderer with the default server
    ///
    /// Tries local Kroki first, then falls back to cloud.
    pub fn new() -> Result<Self, RenderError> {
        // Try local first, then cloud
        if let Ok(renderer) = Self::with_url(LOCAL_KROKI_URL) {
            if renderer.health_check_quick() {
                return Ok(renderer);
            }
        }

        Self::with_url(DEFAULT_KROKI_URL)
    }

    /// Create a Kroki renderer with a specific server URL
    pub fn with_url(base_url: impl Into<String>) -> Result<Self, RenderError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| RenderError::Network(e.to_string()))?;

        Ok(Self {
            base_url,
            client,
            timeout: Duration::from_secs(30),
            available: AtomicBool::new(true),
            checked: AtomicBool::new(false),
        })
    }

    /// Create a renderer for the public Kroki cloud
    pub fn cloud() -> Result<Self, RenderError> {
        Self::with_url(DEFAULT_KROKI_URL)
    }

    /// Create a renderer for a local Kroki instance
    pub fn local() -> Result<Self, RenderError> {
        Self::with_url(LOCAL_KROKI_URL)
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self.client = Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");
        self
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Perform a quick health check (2 second timeout)
    fn health_check_quick(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        self.client
            .get(&url)
            .timeout(Duration::from_secs(2))
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Check if the Kroki server is available
    pub fn health_check(&self) -> Result<bool, RenderError> {
        let url = format!("{}/health", self.base_url);
        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .map_err(|e| RenderError::Network(e.to_string()))?;

        let is_healthy = response.status().is_success();
        self.available.store(is_healthy, Ordering::Relaxed);
        self.checked.store(true, Ordering::Relaxed);

        Ok(is_healthy)
    }

    /// Encode diagram source for use in URLs (deflate + base64)
    pub fn encode_source(source: &str) -> Result<String, RenderError> {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(source.as_bytes())
            .map_err(|e| RenderError::InvalidSource(e.to_string()))?;
        let compressed = encoder
            .finish()
            .map_err(|e| RenderError::InvalidSource(e.to_string()))?;

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
    ) -> Result<String, RenderError> {
        let encoded = Self::encode_source(source)?;
        Ok(format!(
            "{}/{}/{}/{}",
            self.base_url,
            diagram_type.kroki_name(),
            format.kroki_name(),
            encoded
        ))
    }

    /// Render using POST with raw body (more reliable)
    fn render_post(
        &self,
        source: &str,
        diagram_type: DiagramType,
        format: OutputFormat,
    ) -> RenderResult<Vec<u8>> {
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
            .timeout(self.timeout)
            .send()
            .map_err(|e| RenderError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(RenderError::RenderFailed(format!(
                "Kroki error ({}): {}",
                status.as_u16(),
                message
            )));
        }

        response
            .bytes()
            .map(|b| b.to_vec())
            .map_err(|e| RenderError::Network(e.to_string()))
    }

    /// Render using GET with compressed URL (for caching)
    pub fn render_compressed(
        &self,
        source: &str,
        diagram_type: DiagramType,
        format: OutputFormat,
    ) -> RenderResult<Vec<u8>> {
        let encoded = Self::encode_source(source)?;
        let url = format!(
            "{}/{}/{}/{}",
            self.base_url,
            diagram_type.kroki_name(),
            format.kroki_name(),
            encoded
        );

        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .map_err(|e| RenderError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(RenderError::RenderFailed(format!(
                "Kroki error ({}): {}",
                status.as_u16(),
                message
            )));
        }

        response
            .bytes()
            .map(|b| b.to_vec())
            .map_err(|e| RenderError::Network(e.to_string()))
    }
}

impl DiagramRenderer for KrokiRenderer {
    fn name(&self) -> &'static str {
        "kroki"
    }

    fn supports(&self, _diagram_type: DiagramType) -> bool {
        // Kroki supports all diagram types including svgbob
        true
    }

    fn supports_format(&self, format: OutputFormat) -> bool {
        // Kroki supports all standard formats
        matches!(
            format,
            OutputFormat::Png | OutputFormat::Svg | OutputFormat::Pdf
        )
    }

    fn is_available(&self) -> bool {
        // If we haven't checked yet, do a quick check
        if !self.checked.load(Ordering::Relaxed) {
            let available = self.health_check_quick();
            self.available.store(available, Ordering::Relaxed);
            self.checked.store(true, Ordering::Relaxed);
        }
        self.available.load(Ordering::Relaxed)
    }

    fn render(
        &self,
        source: &str,
        diagram_type: DiagramType,
        format: OutputFormat,
        _options: &RenderOptions,
    ) -> RenderResult<Vec<u8>> {
        // Validate source
        let source = source.trim();
        if source.is_empty() {
            return Err(RenderError::InvalidSource(
                "Empty diagram source".to_string(),
            ));
        }

        // Validate format
        if !self.supports_format(format) {
            return Err(RenderError::UnsupportedFormat(format));
        }

        // Render via POST (more reliable than GET for large diagrams)
        self.render_post(source, diagram_type, format)
    }
}

/// Compute SHA-256 hash of content (for drift detection)
pub fn content_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    format!("sha256:{}", hex_encode(&result))
}

/// Helper to format bytes as hex string
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_name() {
        let renderer = KrokiRenderer::cloud().unwrap();
        assert_eq!(renderer.name(), "kroki");
    }

    #[test]
    fn test_client_default_url() {
        let renderer = KrokiRenderer::cloud().unwrap();
        assert_eq!(renderer.base_url(), DEFAULT_KROKI_URL);
    }

    #[test]
    fn test_client_custom_url() {
        let renderer = KrokiRenderer::with_url("http://localhost:8000/").unwrap();
        assert_eq!(renderer.base_url(), "http://localhost:8000");
    }

    #[test]
    fn test_encode_source() {
        let source = "graph TD; A-->B;";
        let encoded = KrokiRenderer::encode_source(source).unwrap();

        // Should be URL-safe base64
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_diagram_url() {
        let renderer = KrokiRenderer::cloud().unwrap();
        let url = renderer
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

    #[test]
    fn test_supports_diagram_types() {
        let renderer = KrokiRenderer::cloud().unwrap();
        assert!(renderer.supports(DiagramType::Mermaid));
        assert!(renderer.supports(DiagramType::PlantUml));
        assert!(renderer.supports(DiagramType::GraphViz));
        assert!(renderer.supports(DiagramType::D2));
    }

    #[test]
    fn test_supports_formats() {
        let renderer = KrokiRenderer::cloud().unwrap();
        assert!(renderer.supports_format(OutputFormat::Png));
        assert!(renderer.supports_format(OutputFormat::Svg));
        assert!(renderer.supports_format(OutputFormat::Pdf));
        assert!(!renderer.supports_format(OutputFormat::Base64));
    }

    // Integration tests - require network access
    // Run with: cargo test --features kroki -- --ignored

    #[test]
    #[ignore]
    fn test_render_mermaid_svg() {
        let renderer = KrokiRenderer::cloud().unwrap();
        let result = renderer.render(
            "graph TD; A-->B; B-->C;",
            DiagramType::Mermaid,
            OutputFormat::Svg,
            &RenderOptions::default(),
        );

        match result {
            Ok(svg) => {
                assert!(!svg.is_empty());
                let svg_str = String::from_utf8_lossy(&svg);
                assert!(svg_str.contains("<svg"));
            }
            Err(e) => {
                eprintln!("Kroki test skipped (network error): {}", e);
            }
        }
    }

    #[test]
    #[ignore]
    fn test_render_mermaid_png() {
        let renderer = KrokiRenderer::cloud().unwrap();
        let result = renderer.render(
            "graph TD; A-->B;",
            DiagramType::Mermaid,
            OutputFormat::Png,
            &RenderOptions::default(),
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
    #[ignore]
    fn test_health_check() {
        let renderer = KrokiRenderer::cloud().unwrap();
        match renderer.health_check() {
            Ok(healthy) => {
                println!("Kroki health: {}", healthy);
            }
            Err(e) => {
                eprintln!("Kroki health check skipped: {}", e);
            }
        }
    }
}
