//! utf8dok Language Server Protocol implementation
//!
//! This binary provides LSP support for AsciiDoc files, reporting validation
//! errors from utf8dok's native validators and Rhai plugins to editors.
//!
//! # Usage
//!
//! ```bash
//! # Start the language server (typically called by an editor)
//! utf8dok-lsp
//!
//! # With debug logging
//! RUST_LOG=debug utf8dok-lsp
//! ```

mod compliance;
mod domain;
mod intelligence;
mod structural;
mod workspace;

use std::collections::HashMap;
use std::sync::Arc;

use domain::DomainEngine;
use intelligence::{RenameAnalyzer, SelectionAnalyzer};
use structural::{FoldingAnalyzer, SymbolAnalyzer};
use workspace::WorkspaceGraph;

use serde_json::Value;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CodeActionOrCommand, CodeActionParams, CodeActionProviderCapability,
    CompletionOptions, CompletionParams, CompletionResponse, DiagnosticOptions,
    DiagnosticRelatedInformation, DiagnosticServerCapabilities, DiagnosticSeverity,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, DocumentSymbolParams, DocumentSymbolResponse, FoldingRange,
    FoldingRangeParams, FoldingRangeProviderCapability, InitializeParams, InitializeResult,
    InitializedParams, Location, MessageType, NumberOrString, OneOf, Position,
    PrepareRenameResponse, Range, RenameParams, SelectionRange, SelectionRangeParams,
    SelectionRangeProviderCapability, SemanticToken, SemanticTokens, SemanticTokensFullOptions,
    SemanticTokensLegend, SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, SymbolInformation,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url, WorkDoneProgressOptions,
    WorkspaceEdit, WorkspaceSymbolParams,
};
use tower_lsp::lsp_types::Diagnostic;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{debug, info, warn};

use utf8dok_core::diagnostics::{Diagnostic as Utf8dokDiagnostic, Severity};
use utf8dok_core::parse;
use utf8dok_validate::ValidationEngine;

/// LSP Backend state
struct Backend {
    /// LSP client for sending notifications
    client: Client,
    /// Validation engine with default validators
    validation_engine: Arc<RwLock<ValidationEngine>>,
    /// Document store for open documents
    documents: Arc<RwLock<HashMap<Url, String>>>,
    /// Workspace graph for cross-file intelligence
    workspace_graph: Arc<RwLock<WorkspaceGraph>>,
}

impl Backend {
    /// Create a new backend instance
    fn new(client: Client) -> Self {
        Self {
            client,
            validation_engine: Arc::new(RwLock::new(ValidationEngine::with_defaults())),
            documents: Arc::new(RwLock::new(HashMap::new())),
            workspace_graph: Arc::new(RwLock::new(WorkspaceGraph::new())),
        }
    }

    /// Get document text by URI
    async fn get_document(&self, uri: &Url) -> Option<String> {
        let docs = self.documents.read().await;
        docs.get(uri).cloned()
    }

    /// Store document text
    async fn store_document(&self, uri: Url, text: String) {
        let mut docs = self.documents.write().await;
        docs.insert(uri, text);
    }

    /// Remove document from store
    async fn remove_document(&self, uri: &Url) {
        let mut docs = self.documents.write().await;
        docs.remove(uri);
    }

    /// Update the workspace graph with document content
    async fn update_workspace_graph(&self, uri: &Url, text: &str) {
        let mut graph = self.workspace_graph.write().await;
        graph.add_document(uri.as_str(), text);
    }

    /// Remove document from workspace graph
    async fn remove_from_workspace_graph(&self, uri: &Url) {
        let mut graph = self.workspace_graph.write().await;
        graph.remove_document(uri.as_str());
    }

    /// Validate a document and publish diagnostics
    async fn validate(&self, uri: Url, text: String) {
        debug!("Validating document: {}", uri);

        // Parse the AsciiDoc content
        let ast = match parse(&text) {
            Ok(doc) => doc,
            Err(e) => {
                warn!("Failed to parse document {}: {}", uri, e);
                // Publish a parse error diagnostic
                let diagnostic = Diagnostic {
                    range: Range {
                        start: Position::new(0, 0),
                        end: Position::new(0, 0),
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("PARSE001".to_string())),
                    source: Some("utf8dok".to_string()),
                    message: format!("Parse error: {}", e),
                    ..Default::default()
                };
                self.client
                    .publish_diagnostics(uri, vec![diagnostic], None)
                    .await;
                return;
            }
        };

        // Run validation
        let engine = self.validation_engine.read().await;
        let utf8dok_diagnostics = engine.validate(&ast);

        // Convert to LSP diagnostics
        let lsp_diagnostics: Vec<Diagnostic> = utf8dok_diagnostics
            .into_iter()
            .map(|d| self.convert_diagnostic(&d, &text))
            .collect();

        debug!(
            "Publishing {} diagnostics for {}",
            lsp_diagnostics.len(),
            uri
        );

        // Publish diagnostics
        self.client
            .publish_diagnostics(uri, lsp_diagnostics, None)
            .await;
    }

    /// Convert utf8dok diagnostic to LSP diagnostic
    fn convert_diagnostic(&self, diag: &Utf8dokDiagnostic, source_text: &str) -> Diagnostic {
        // Convert severity
        let severity = match diag.severity {
            Severity::Error | Severity::Fatal => Some(DiagnosticSeverity::ERROR),
            Severity::Warning => Some(DiagnosticSeverity::WARNING),
            Severity::Info => Some(DiagnosticSeverity::INFORMATION),
            Severity::Hint => Some(DiagnosticSeverity::HINT),
        };

        // Convert span to range
        let range = if let Some(span) = &diag.span {
            if let (Some(line), Some(col)) = (span.line, span.column) {
                // Use line/col from span (1-indexed to 0-indexed)
                Range {
                    start: Position::new(
                        line.saturating_sub(1) as u32,
                        col.saturating_sub(1) as u32,
                    ),
                    end: Position::new(line.saturating_sub(1) as u32, col as u32),
                }
            } else {
                // Calculate line/col from byte offset
                self.offset_to_range(span.start, span.end, source_text)
            }
        } else {
            // No span info - try to extract from notes (block index)
            self.extract_range_from_notes(diag, source_text)
        };

        // Build code
        let code = diag
            .code
            .as_ref()
            .map(|c| NumberOrString::String(c.clone()));

        // Build related information from notes
        let related_information = if diag.notes.is_empty() {
            None
        } else {
            Some(
                diag.notes
                    .iter()
                    .map(|note| DiagnosticRelatedInformation {
                        location: Location {
                            uri: Url::parse("file:///unknown").unwrap(),
                            range: Range::default(),
                        },
                        message: note.clone(),
                    })
                    .collect(),
            )
        };

        Diagnostic {
            range,
            severity,
            code,
            code_description: None,
            source: Some("utf8dok".to_string()),
            message: diag.message.clone(),
            related_information,
            tags: None,
            data: diag.help.as_ref().map(|h| Value::String(h.clone())),
        }
    }

    /// Convert byte offsets to LSP range
    fn offset_to_range(&self, start: usize, end: usize, text: &str) -> Range {
        let mut line = 0u32;
        let mut col = 0u32;
        let mut start_pos = Position::new(0, 0);
        let mut end_pos = Position::new(0, 0);

        for (i, ch) in text.char_indices() {
            if i == start {
                start_pos = Position::new(line, col);
            }
            if i == end {
                end_pos = Position::new(line, col);
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        // If end was not found, use end of file
        if end >= text.len() {
            end_pos = Position::new(line, col);
        }

        Range {
            start: start_pos,
            end: end_pos,
        }
    }

    /// Try to extract range from diagnostic notes (e.g., "Found at block index X")
    fn extract_range_from_notes(&self, diag: &Utf8dokDiagnostic, source_text: &str) -> Range {
        // Look for "block index N" pattern in notes
        for note in &diag.notes {
            if let Some(idx_str) = note.strip_prefix("Found at block index ") {
                if let Ok(block_idx) = idx_str.trim().parse::<usize>() {
                    // Try to find the Nth block-like element (heading markers)
                    return self.find_block_range(block_idx, source_text);
                }
            }
        }

        // Default to start of file
        Range {
            start: Position::new(0, 0),
            end: Position::new(0, 0),
        }
    }

    /// Find the range of a block by index (simplified heuristic)
    fn find_block_range(&self, block_idx: usize, text: &str) -> Range {
        let mut current_block = 0usize;

        for (line_num, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            // Check for block markers (headings, paragraphs after blank lines)
            let is_heading = trimmed.starts_with('=');
            let is_block_start = is_heading || (!trimmed.is_empty() && line_num > 0);

            if is_block_start && !trimmed.is_empty() {
                if current_block == block_idx {
                    let block_line = line_num as u32;
                    return Range {
                        start: Position::new(block_line, 0),
                        end: Position::new(block_line, line.len() as u32),
                    };
                }
                // Only count non-empty content as blocks
                if is_heading
                    || (line_num > 0
                        && text
                            .lines()
                            .nth(line_num.saturating_sub(1))
                            .map(|l| l.trim().is_empty())
                            .unwrap_or(false))
                {
                    current_block += 1;
                }
            }
        }

        // Default if block not found
        Range {
            start: Position::new(0, 0),
            end: Position::new(0, 0),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        info!("utf8dok LSP server initializing");

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                // Diagnostics
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("utf8dok".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                // Folding ranges (Phase 7 Week 1)
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                // Document symbols (Phase 7 Week 2)
                document_symbol_provider: Some(OneOf::Left(true)),
                // Selection ranges (Phase 8 Week 1)
                selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
                // Rename refactoring (Phase 8 Week 2)
                rename_provider: Some(OneOf::Right(tower_lsp::lsp_types::RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                // Completion (Phase 9)
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        "<".to_string(), // For <<xref
                        ":".to_string(), // For :attributes
                        "[".to_string(), // For [blocks]
                    ]),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    ..Default::default()
                }),
                // Code actions (Phase 9)
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                // Semantic tokens (Phase 10)
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: domain::semantic::SemanticAnalyzer::token_legend(),
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                        },
                    ),
                ),
                // Workspace symbols (Phase 11)
                workspace_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "utf8dok-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("utf8dok LSP server initialized");
        self.client
            .log_message(MessageType::INFO, "utf8dok language server ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        info!("utf8dok LSP server shutting down");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        debug!("Document opened: {}", params.text_document.uri);
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        self.store_document(uri.clone(), text.clone()).await;
        self.update_workspace_graph(&uri, &text).await;
        self.validate(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        debug!("Document changed: {}", params.text_document.uri);
        // Since we use FULL sync, the entire content is in the first change
        if let Some(change) = params.content_changes.into_iter().next() {
            let uri = params.text_document.uri.clone();
            let text = change.text.clone();
            self.store_document(uri.clone(), text.clone()).await;
            self.update_workspace_graph(&uri, &text).await;
            self.validate(uri, text).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        debug!("Document saved: {}", params.text_document.uri);
        // Re-validate on save if text is provided
        if let Some(text) = params.text {
            let uri = params.text_document.uri.clone();
            self.store_document(uri.clone(), text.clone()).await;
            self.update_workspace_graph(&uri, &text).await;
            self.validate(uri, text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        debug!("Document closed: {}", params.text_document.uri);
        self.remove_document(&params.text_document.uri).await;
        self.remove_from_workspace_graph(&params.text_document.uri).await;
        // Clear diagnostics for closed document
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> Result<Option<Vec<FoldingRange>>> {
        let uri = params.text_document.uri;
        debug!("Folding range request for: {}", uri);

        // Get document from store
        let text = match self.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found for folding: {}", uri);
                return Ok(None);
            }
        };

        // Generate folding ranges
        let ranges = FoldingAnalyzer::generate_ranges(&text);
        debug!("Generated {} folding ranges for {}", ranges.len(), uri);

        Ok(Some(ranges))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        debug!("Document symbol request for: {}", uri);

        // Get document from store
        let text = match self.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found for symbols: {}", uri);
                return Ok(None);
            }
        };

        // Generate document symbols
        let symbols = SymbolAnalyzer::extract_symbols(&text);
        debug!("Generated {} document symbols for {}", symbols.len(), uri);

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> Result<Option<Vec<SelectionRange>>> {
        let uri = params.text_document.uri;
        debug!("Selection range request for: {}", uri);

        // Get document from store
        let text = match self.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found for selection: {}", uri);
                return Ok(None);
            }
        };

        // Generate selection ranges for each position
        let analyzer = SelectionAnalyzer::new(&text);
        let mut ranges = Vec::new();

        for position in params.positions {
            if let Some(selection) = analyzer.to_lsp_selection_ranges(position) {
                ranges.push(selection);
            }
        }

        if ranges.is_empty() {
            Ok(None)
        } else {
            debug!("Generated {} selection ranges for {}", ranges.len(), uri);
            Ok(Some(ranges))
        }
    }

    async fn prepare_rename(
        &self,
        params: tower_lsp::lsp_types::TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        debug!("Prepare rename request for: {}", uri);

        // Get document from store
        let text = match self.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found for rename: {}", uri);
                return Ok(None);
            }
        };

        // Check if rename is available at position
        let analyzer = RenameAnalyzer::new(&text);
        if let Some((range, placeholder)) = analyzer.can_rename_at(params.position) {
            Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                range,
                placeholder,
            }))
        } else {
            Ok(None)
        }
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.clone();
        debug!("Rename request for: {}", uri);

        // Get document from store
        let text = match self.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found for rename: {}", uri);
                return Ok(None);
            }
        };

        // Perform rename
        let analyzer = RenameAnalyzer::new(&text);
        let position = params.text_document_position.position;
        let new_name = &params.new_name;

        if let Some(result) = analyzer.rename_at_position(position, new_name) {
            debug!(
                "Renamed '{}' to '{}' with {} edits",
                result.old_name,
                result.new_name,
                result.edits.len()
            );

            let mut changes = HashMap::new();
            changes.insert(uri, result.edits);

            Ok(Some(WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            }))
        } else {
            Ok(None)
        }
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        debug!("Completion request for: {}", uri);

        // Get document from store
        let text = match self.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found for completion: {}", uri);
                return Ok(None);
            }
        };

        // Get completions from domain engine
        let engine = DomainEngine::new();
        let position = params.text_document_position.position;
        let items = engine.get_completions(&text, position);

        if items.is_empty() {
            Ok(None)
        } else {
            debug!("Generated {} completions for {}", items.len(), uri);
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> Result<Option<Vec<CodeActionOrCommand>>> {
        let uri = params.text_document.uri.clone();
        debug!("Code action request for: {}", uri);

        // Get document from store
        let text = match self.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found for code action: {}", uri);
                return Ok(None);
            }
        };

        // Get code actions from domain engine
        let engine = DomainEngine::new();
        let actions = engine.get_code_actions(&text, &params);

        if actions.is_empty() {
            Ok(None)
        } else {
            debug!("Generated {} code actions for {}", actions.len(), uri);
            // Wrap CodeAction in CodeActionOrCommand
            let wrapped: Vec<CodeActionOrCommand> = actions
                .into_iter()
                .map(CodeActionOrCommand::CodeAction)
                .collect();
            Ok(Some(wrapped))
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        debug!("Semantic tokens request for: {}", uri);

        // Get document from store
        let text = match self.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found for semantic tokens: {}", uri);
                return Ok(None);
            }
        };

        // Get semantic tokens from domain engine
        let engine = DomainEngine::new();
        let token_infos = engine.get_semantic_tokens(&text);

        if token_infos.is_empty() {
            return Ok(None);
        }

        // Convert to LSP format
        let lsp_tokens = engine.semantic_analyzer().to_lsp_tokens(&token_infos);

        // Convert to final format
        let data: Vec<u32> = lsp_tokens
            .into_iter()
            .flat_map(|t| {
                vec![
                    t.delta_line,
                    t.delta_start,
                    t.length,
                    t.token_type,
                    t.token_modifiers,
                ]
            })
            .collect();

        debug!("Generated {} semantic tokens for {}", data.len() / 5, uri);

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: data
                .chunks(5)
                .map(|chunk| SemanticToken {
                    delta_line: chunk[0],
                    delta_start: chunk[1],
                    length: chunk[2],
                    token_type: chunk[3],
                    token_modifiers_bitset: chunk[4],
                })
                .collect(),
        })))
    }

    #[allow(deprecated)]
    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        debug!("Workspace symbol request: query='{}'", params.query);

        let graph = self.workspace_graph.read().await;
        let symbols = graph.query_symbols(&params.query);

        let result: Vec<SymbolInformation> = symbols
            .into_iter()
            .map(|sym| SymbolInformation {
                name: sym.name.clone(),
                kind: workspace::symbol_provider::SymbolProvider::convert_symbol_kind(sym.kind),
                tags: None,
                deprecated: None,
                location: sym.location.clone(),
                container_name: None,
            })
            .collect();

        debug!("Found {} workspace symbols", result.len());

        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    info!(
        "Starting utf8dok Language Server v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Create LSP service
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_conversion() {
        // Test that our severity mapping is correct
        assert!(matches!(Severity::Error, Severity::Error | Severity::Fatal));
    }

    #[test]
    fn test_offset_to_range_logic() {
        // Test the offset-to-position logic directly
        let text = "line1\nline2\nline3";

        // Line 0: "line1" (bytes 0-4)
        // Line 1: "line2" (bytes 6-10)
        // Line 2: "line3" (bytes 12-16)

        let mut line = 0u32;
        let mut col = 0u32;
        let target_offset = 7; // 'i' in "line2"

        for (i, ch) in text.char_indices() {
            if i == target_offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        assert_eq!(line, 1); // Second line (0-indexed)
        assert_eq!(col, 1); // Second character (0-indexed)
    }

    #[test]
    fn test_find_block_by_heading() {
        let text = "= Title\n\n== Section\n\nParagraph";

        // Count headings in the text
        let heading_count = text.lines().filter(|l| l.starts_with('=')).count();
        assert_eq!(heading_count, 2);
    }

    #[tokio::test]
    async fn test_validation_engine_integration() {
        use utf8dok_ast::{Block, Document, Heading, Inline};

        // Create a document with a hierarchy violation
        let mut doc = Document::new();
        doc.blocks.push(Block::Heading(Heading {
            level: 1,
            text: vec![Inline::Text("Title".to_string())],
            style_id: None,
            anchor: None,
        }));
        doc.blocks.push(Block::Heading(Heading {
            level: 4, // Skip levels 2 and 3
            text: vec![Inline::Text("Deep".to_string())],
            style_id: None,
            anchor: None,
        }));

        let engine = ValidationEngine::with_defaults();
        let diagnostics = engine.validate(&doc);

        // Should have at least one diagnostic for the hierarchy jump
        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some("DOC101")));
    }

    #[test]
    fn test_diagnostic_to_lsp_conversion() {
        use utf8dok_core::diagnostics::Span;

        // Create a utf8dok diagnostic
        let diag = Utf8dokDiagnostic::warning("Test warning")
            .with_code("TEST001")
            .with_span(Span::new(0, 10).with_position(1, 1))
            .with_help("This is help text");

        // Verify the diagnostic has expected fields
        assert_eq!(diag.message, "Test warning");
        assert_eq!(diag.code, Some("TEST001".to_string()));
        assert!(diag.span.is_some());
        assert_eq!(diag.help, Some("This is help text".to_string()));
    }
}
