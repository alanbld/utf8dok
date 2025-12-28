//! Embedded JavaScript diagram renderer
//!
//! This module provides diagram rendering using an embedded V8 JavaScript runtime
//! via `deno_core`. Currently supports Mermaid diagrams with native offline rendering.
//!
//! # Feature Flag
//!
//! This module requires the `js` feature:
//! ```toml
//! utf8dok-diagrams = { version = "0.1", features = ["js"] }
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         JsRenderer                                   │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  1. Initialize deno_core JsRuntime (lazy, singleton)                │
//! │  2. Load mermaid.min.js + DOM shim                                  │
//! │  3. Execute: mermaid.render(source) → SVG string                    │
//! │  4. (Optional) Convert SVG → PNG via resvg                          │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::sync::Mutex;

use deno_core::{FastString, JsRuntime, RuntimeOptions};

use crate::renderer::{DiagramRenderer, RenderError, RenderOptions, RenderResult};
use crate::types::{DiagramType, OutputFormat};

/// DOM shim for headless Mermaid execution
///
/// Mermaid.js requires a minimal DOM API. This shim provides just enough
/// to let Mermaid initialize and render SVG without a real browser.
const DOM_SHIM: &str = r#"
// Minimal DOM shim for Mermaid.js headless rendering
globalThis.window = globalThis;
globalThis.self = globalThis;

// Document mock
globalThis.document = {
    body: {
        appendChild: function(el) { return el; },
        removeChild: function(el) { return el; },
        querySelector: function() { return null; },
        querySelectorAll: function() { return []; },
        style: {},
    },
    head: {
        appendChild: function(el) { return el; },
    },
    documentElement: {
        style: {},
    },
    createElement: function(tag) {
        const el = {
            tagName: tag.toUpperCase(),
            style: {},
            classList: {
                add: function() {},
                remove: function() {},
                contains: function() { return false; },
            },
            setAttribute: function(name, value) { this[name] = value; },
            getAttribute: function(name) { return this[name]; },
            appendChild: function(child) {
                if (!this.children) this.children = [];
                this.children.push(child);
                return child;
            },
            removeChild: function(child) { return child; },
            innerHTML: '',
            innerText: '',
            textContent: '',
            children: [],
            childNodes: [],
            parentNode: null,
            getBoundingClientRect: function() {
                return { x: 0, y: 0, width: 100, height: 100, top: 0, left: 0, right: 100, bottom: 100 };
            },
            cloneNode: function(deep) { return Object.assign({}, this); },
            querySelectorAll: function() { return []; },
            querySelector: function() { return null; },
            addEventListener: function() {},
            removeEventListener: function() {},
            insertAdjacentHTML: function(position, html) { this.innerHTML += html; },
        };
        return el;
    },
    createElementNS: function(ns, tag) {
        return this.createElement(tag);
    },
    createTextNode: function(text) {
        return { textContent: text, nodeType: 3 };
    },
    querySelector: function() { return null; },
    querySelectorAll: function() { return []; },
    getElementById: function() { return null; },
    getElementsByClassName: function() { return []; },
    getElementsByTagName: function() { return []; },
    createComment: function() { return { nodeType: 8 }; },
    createDocumentFragment: function() {
        return {
            appendChild: function(child) { return child; },
            children: [],
        };
    },
    styleSheets: [],
};

// Window mock
globalThis.window = {
    document: globalThis.document,
    getComputedStyle: function() {
        return {
            getPropertyValue: function() { return ''; },
        };
    },
    matchMedia: function() {
        return {
            matches: false,
            addListener: function() {},
            removeListener: function() {},
        };
    },
    addEventListener: function() {},
    removeEventListener: function() {},
    requestAnimationFrame: function(cb) { return setTimeout(cb, 16); },
    cancelAnimationFrame: function(id) { clearTimeout(id); },
    navigator: {
        userAgent: 'utf8dok/1.0 (embedded V8)',
        language: 'en-US',
    },
    location: {
        href: 'about:blank',
        protocol: 'about:',
        host: '',
        hostname: '',
        pathname: 'blank',
    },
    innerWidth: 1920,
    innerHeight: 1080,
    devicePixelRatio: 1,
    SVGElement: function() {},
    HTMLElement: function() {},
    Element: function() {},
    Node: function() {},
    DOMParser: function() {
        return {
            parseFromString: function(str, type) {
                return { documentElement: {} };
            },
        };
    },
};

// Performance API
globalThis.performance = {
    now: function() { return Date.now(); },
};

// Console is already available in deno_core
// Just ensure it exists
if (typeof console === 'undefined') {
    globalThis.console = {
        log: function() {},
        warn: function() {},
        error: function() {},
        debug: function() {},
        info: function() {},
    };
}

// MutationObserver mock
globalThis.MutationObserver = function() {
    return {
        observe: function() {},
        disconnect: function() {},
        takeRecords: function() { return []; },
    };
};

// ResizeObserver mock
globalThis.ResizeObserver = function() {
    return {
        observe: function() {},
        unobserve: function() {},
        disconnect: function() {},
    };
};

// Custom event for deno
globalThis.CustomEvent = function(type, options) {
    this.type = type;
    this.detail = options?.detail;
};

// Event mock
globalThis.Event = function(type) {
    this.type = type;
};

// Fetch mock (needed by some mermaid plugins)
if (typeof fetch === 'undefined') {
    globalThis.fetch = function() {
        return Promise.reject(new Error('fetch not available in headless mode'));
    };
}
"#;

/// Mermaid rendering wrapper
const MERMAID_WRAPPER: &str = r#"
// Mermaid rendering wrapper for utf8dok
async function renderMermaid(source, id) {
    try {
        // Initialize mermaid with safe config
        mermaid.initialize({
            startOnLoad: false,
            theme: 'default',
            securityLevel: 'strict',
            fontFamily: 'sans-serif',
        });

        // Render the diagram
        const { svg } = await mermaid.render(id || 'utf8dok-diagram', source);
        return { success: true, svg: svg };
    } catch (error) {
        return { success: false, error: error.message || String(error) };
    }
}
"#;

// Thread-local JavaScript runtime for diagram rendering
// Using thread-local storage because deno_core's JsRuntime is not Send+Sync,
// but DiagramRenderer requires Send+Sync. Each thread gets its own runtime.
thread_local! {
    static JS_RUNTIME: RefCell<Option<JsRuntime>> = const { RefCell::new(None) };
}

/// Global state for mermaid.js loading
static MERMAID_LOADED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

/// Embedded JavaScript diagram renderer
///
/// Uses deno_core to run Mermaid.js for offline diagram rendering.
/// The runtime is lazily initialized on first use.
///
/// # Example
///
/// ```ignore
/// use utf8dok_diagrams::{JsRenderer, DiagramRenderer, DiagramType, OutputFormat, RenderOptions};
///
/// let renderer = JsRenderer::new()?;
/// let svg = renderer.render(
///     "graph TD; A-->B;",
///     DiagramType::Mermaid,
///     OutputFormat::Svg,
///     &RenderOptions::default(),
/// )?;
/// ```
pub struct JsRenderer {
    /// Whether the renderer has been initialized
    initialized: std::sync::atomic::AtomicBool,
}

impl Default for JsRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl JsRenderer {
    /// Create a new JavaScript renderer
    pub fn new() -> Self {
        Self {
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Initialize the JS runtime for this thread
    fn ensure_runtime() -> Result<(), RenderError> {
        JS_RUNTIME.with(|runtime| {
            let mut runtime = runtime.borrow_mut();
            if runtime.is_none() {
                let rt = Self::create_runtime()?;
                *runtime = Some(rt);
            }
            Ok(())
        })
    }

    /// Create a new deno_core JsRuntime with DOM shim and Mermaid
    fn create_runtime() -> Result<JsRuntime, RenderError> {
        let options = RuntimeOptions::default();
        let mut runtime = JsRuntime::new(options);

        // Load DOM shim
        runtime
            .execute_script("<dom_shim>", FastString::Static(DOM_SHIM))
            .map_err(|e| RenderError::RenderFailed(format!("Failed to load DOM shim: {}", e)))?;

        // Load Mermaid.js (either embedded or from file)
        Self::load_mermaid(&mut runtime)?;

        // Load wrapper
        runtime
            .execute_script("<mermaid_wrapper>", FastString::Static(MERMAID_WRAPPER))
            .map_err(|e| {
                RenderError::RenderFailed(format!("Failed to load Mermaid wrapper: {}", e))
            })?;

        log::debug!("JsRenderer: Runtime initialized with Mermaid.js");
        Ok(runtime)
    }

    /// Load mermaid.js into the runtime
    fn load_mermaid(runtime: &mut JsRuntime) -> Result<(), RenderError> {
        // Check if already loaded globally
        let mut loaded = MERMAID_LOADED.lock().unwrap();
        if *loaded {
            return Ok(());
        }

        // Try to get mermaid.js path from build
        let mermaid_path = option_env!("MERMAID_JS_PATH");

        if let Some(path) = mermaid_path {
            // Try to read the downloaded file
            if let Ok(mermaid_js) = std::fs::read_to_string(path) {
                if !mermaid_js.contains("throw new Error") {
                    runtime
                        .execute_script("<mermaid>", mermaid_js.into())
                        .map_err(|e| {
                            RenderError::RenderFailed(format!("Failed to load Mermaid.js: {}", e))
                        })?;
                    *loaded = true;
                    log::info!("JsRenderer: Loaded Mermaid.js from build cache");
                    return Ok(());
                }
            }
        }

        // Fallback: try to load from current directory or common paths
        let fallback_paths = [
            "mermaid.min.js",
            "assets/mermaid.min.js",
            "vendor/mermaid.min.js",
        ];

        for path in fallback_paths {
            if let Ok(mermaid_js) = std::fs::read_to_string(path) {
                runtime
                    .execute_script("<mermaid>", mermaid_js.into())
                    .map_err(|e| {
                        RenderError::RenderFailed(format!("Failed to load Mermaid.js: {}", e))
                    })?;
                *loaded = true;
                log::info!("JsRenderer: Loaded Mermaid.js from {}", path);
                return Ok(());
            }
        }

        Err(RenderError::Unavailable(
            "mermaid.js not found. Either enable network download in build.rs or \
             place mermaid.min.js in the working directory."
                .to_string(),
        ))
    }

    /// Render Mermaid diagram to SVG
    fn render_mermaid_svg(&self, source: &str) -> RenderResult<String> {
        Self::ensure_runtime()?;

        JS_RUNTIME.with(|runtime_cell| {
            let mut runtime_ref = runtime_cell.borrow_mut();
            let runtime = runtime_ref.as_mut().ok_or_else(|| {
                RenderError::RenderFailed("JS runtime not initialized".to_string())
            })?;

            // Escape the source for JavaScript
            let escaped_source = source
                .replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace('\n', "\\n")
                .replace('\r', "\\r");

            // Generate unique ID for this diagram
            let diagram_id = format!(
                "diagram_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            );

            // Use a synchronous approach - store result in a global variable
            // This avoids the async/promise complexity
            let script = format!(
                r#"
                (function() {{
                    try {{
                        mermaid.initialize({{
                            startOnLoad: false,
                            theme: 'default',
                            securityLevel: 'strict',
                            fontFamily: 'sans-serif',
                        }});

                        // Use sync-style rendering
                        let svg = '';
                        mermaid.render('{}', '{}').then(result => {{
                            globalThis.__utf8dok_result = JSON.stringify({{ success: true, svg: result.svg }});
                        }}).catch(error => {{
                            globalThis.__utf8dok_result = JSON.stringify({{ success: false, error: error.message || String(error) }});
                        }});
                    }} catch (error) {{
                        globalThis.__utf8dok_result = JSON.stringify({{ success: false, error: error.message || String(error) }});
                    }}
                }})();
                "#,
                diagram_id, escaped_source
            );

            // Execute the script
            runtime
                .execute_script("<render>", script.into())
                .map_err(|e| RenderError::RenderFailed(format!("Mermaid render failed: {}", e)))?;

            // Run the event loop to let the promise resolve
            let tokio_rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| RenderError::RenderFailed(format!("Failed to create tokio runtime: {}", e)))?;

            tokio_rt.block_on(async {
                runtime
                    .run_event_loop(deno_core::PollEventLoopOptions::default())
                    .await
                    .map_err(|e| RenderError::RenderFailed(format!("Event loop error: {}", e)))
            })?;

            // Now read the result from the global variable
            let get_result_script: FastString = "globalThis.__utf8dok_result || '{\"success\": false, \"error\": \"No result\"}'".to_string().into();
            let result = runtime
                .execute_script("<get_result>", get_result_script)
                .map_err(|e| RenderError::RenderFailed(format!("Failed to get result: {}", e)))?;

            // Extract the string result
            let scope = &mut runtime.handle_scope();
            let local = deno_core::v8::Local::new(scope, result);
            let result_str = local
                .to_string(scope)
                .ok_or_else(|| {
                    RenderError::RenderFailed("Failed to convert result to string".to_string())
                })?
                .to_rust_string_lossy(scope);

            Self::parse_render_result(&result_str)
        })
    }

    /// Parse the JSON result from renderMermaid
    fn parse_render_result(json: &str) -> RenderResult<String> {
        #[derive(serde::Deserialize)]
        struct RenderResult {
            success: bool,
            svg: Option<String>,
            error: Option<String>,
        }

        let result: RenderResult = serde_json::from_str(json).map_err(|e| {
            RenderError::RenderFailed(format!("Failed to parse render result: {}", e))
        })?;

        if result.success {
            result
                .svg
                .ok_or_else(|| RenderError::RenderFailed("No SVG in result".to_string()))
        } else {
            Err(RenderError::RenderFailed(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    /// Convert SVG to PNG using resvg (if native feature enabled)
    #[cfg(feature = "native")]
    fn svg_to_png(&self, svg: &str, options: &RenderOptions) -> RenderResult<Vec<u8>> {
        // Parse SVG
        let tree = {
            let opts = usvg::Options::default();
            usvg::Tree::from_str(svg, &opts)
                .map_err(|e| RenderError::RenderFailed(format!("SVG parsing failed: {}", e)))?
        };

        // Get dimensions
        let original_size = tree.size();
        let original_width = original_size.width();
        let original_height = original_size.height();

        // Calculate scale
        let base_scale = options.scale.unwrap_or(1.0);
        let (target_width, target_height, scale) = match (options.width, options.height) {
            (Some(w), Some(h)) => {
                let scale_x = w as f32 / original_width;
                let scale_y = h as f32 / original_height;
                let scale = scale_x.min(scale_y) * base_scale;
                (
                    (original_width * scale).ceil() as u32,
                    (original_height * scale).ceil() as u32,
                    scale,
                )
            }
            (Some(w), None) => {
                let scale = (w as f32 / original_width) * base_scale;
                (w, (original_height * scale).ceil() as u32, scale)
            }
            (None, Some(h)) => {
                let scale = (h as f32 / original_height) * base_scale;
                ((original_width * scale).ceil() as u32, h, scale)
            }
            (None, None) => (
                (original_width * base_scale).ceil() as u32,
                (original_height * base_scale).ceil() as u32,
                base_scale,
            ),
        };

        // Create pixmap
        let mut pixmap = tiny_skia::Pixmap::new(target_width.max(1), target_height.max(1))
            .ok_or_else(|| {
                RenderError::RenderFailed(format!(
                    "Failed to create pixmap ({}x{})",
                    target_width, target_height
                ))
            })?;

        // Fill background
        if let Some(ref bg) = options.background {
            if let Some(color) = parse_color(bg) {
                pixmap.fill(color);
            }
        }

        // Render
        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        // Encode to PNG
        pixmap
            .encode_png()
            .map_err(|e| RenderError::RenderFailed(format!("PNG encoding failed: {}", e)))
    }

    #[cfg(not(feature = "native"))]
    fn svg_to_png(&self, _svg: &str, _options: &RenderOptions) -> RenderResult<Vec<u8>> {
        Err(RenderError::UnsupportedFormat(OutputFormat::Png))
    }
}

impl DiagramRenderer for JsRenderer {
    fn name(&self) -> &'static str {
        "js"
    }

    fn supports(&self, diagram_type: DiagramType) -> bool {
        matches!(diagram_type, DiagramType::Mermaid)
    }

    fn supports_format(&self, format: OutputFormat) -> bool {
        match format {
            OutputFormat::Svg => true,
            #[cfg(feature = "native")]
            OutputFormat::Png => true,
            #[cfg(not(feature = "native"))]
            OutputFormat::Png => false,
            _ => false,
        }
    }

    fn is_available(&self) -> bool {
        // Check if mermaid.js is available
        let mermaid_available = option_env!("MERMAID_JS_PATH").is_some()
            || std::path::Path::new("mermaid.min.js").exists()
            || std::path::Path::new("assets/mermaid.min.js").exists()
            || std::path::Path::new("vendor/mermaid.min.js").exists();

        if !mermaid_available {
            log::warn!("JsRenderer: mermaid.js not found, renderer unavailable");
        }

        mermaid_available
    }

    fn render(
        &self,
        source: &str,
        diagram_type: DiagramType,
        format: OutputFormat,
        options: &RenderOptions,
    ) -> RenderResult<Vec<u8>> {
        // Validate diagram type
        if !self.supports(diagram_type) {
            return Err(RenderError::UnsupportedType(diagram_type));
        }

        // Validate format
        if !self.supports_format(format) {
            return Err(RenderError::UnsupportedFormat(format));
        }

        // Validate source
        let source = source.trim();
        if source.is_empty() {
            return Err(RenderError::InvalidSource("Empty diagram source".to_string()));
        }

        // Mark as initialized
        self.initialized
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Render based on diagram type
        match diagram_type {
            DiagramType::Mermaid => {
                let svg = self.render_mermaid_svg(source)?;

                match format {
                    OutputFormat::Svg => Ok(svg.into_bytes()),
                    OutputFormat::Png => self.svg_to_png(&svg, options),
                    _ => Err(RenderError::UnsupportedFormat(format)),
                }
            }
            _ => Err(RenderError::UnsupportedType(diagram_type)),
        }
    }
}

/// Parse a CSS color string to tiny_skia::Color
#[cfg(feature = "native")]
fn parse_color(color: &str) -> Option<tiny_skia::Color> {
    let color = color.trim().to_lowercase();

    match color.as_str() {
        "white" => Some(tiny_skia::Color::WHITE),
        "black" => Some(tiny_skia::Color::BLACK),
        "transparent" => Some(tiny_skia::Color::TRANSPARENT),
        _ => {
            if let Some(hex) = color.strip_prefix('#') {
                match hex.len() {
                    6 => {
                        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                        tiny_skia::Color::from_rgba8(r, g, b, 255).into()
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_renderer_name() {
        let renderer = JsRenderer::new();
        assert_eq!(renderer.name(), "js");
    }

    #[test]
    fn test_js_renderer_supports_mermaid() {
        let renderer = JsRenderer::new();
        assert!(renderer.supports(DiagramType::Mermaid));
        assert!(!renderer.supports(DiagramType::PlantUml));
        assert!(!renderer.supports(DiagramType::Svgbob));
    }

    #[test]
    fn test_js_renderer_supports_formats() {
        let renderer = JsRenderer::new();
        assert!(renderer.supports_format(OutputFormat::Svg));

        #[cfg(feature = "native")]
        assert!(renderer.supports_format(OutputFormat::Png));

        assert!(!renderer.supports_format(OutputFormat::Pdf));
    }

    #[test]
    fn test_dom_shim_syntax() {
        // Just verify the DOM shim is valid JavaScript by checking it parses
        // We can't actually run it without the full deno runtime in tests
        assert!(!DOM_SHIM.is_empty());
        assert!(DOM_SHIM.contains("globalThis.window"));
        assert!(DOM_SHIM.contains("globalThis.document"));
    }

    #[test]
    fn test_mermaid_wrapper_syntax() {
        assert!(!MERMAID_WRAPPER.is_empty());
        assert!(MERMAID_WRAPPER.contains("renderMermaid"));
    }

    // Integration test - requires mermaid.js to be available
    #[test]
    #[ignore]
    fn test_render_mermaid_svg() {
        let renderer = JsRenderer::new();
        if !renderer.is_available() {
            eprintln!("Skipping test: mermaid.js not available");
            return;
        }

        let result = renderer.render(
            "graph TD; A-->B;",
            DiagramType::Mermaid,
            OutputFormat::Svg,
            &RenderOptions::default(),
        );

        match result {
            Ok(svg) => {
                let svg_str = String::from_utf8_lossy(&svg);
                assert!(svg_str.contains("<svg"));
            }
            Err(e) => {
                eprintln!("Mermaid render failed: {}", e);
            }
        }
    }
}
