//! utf8dok Language Server binary entry point
//!
//! This is a thin wrapper that calls the library's `run_server()` function.

use utf8dok_lsp::run_server;

#[tokio::main]
async fn main() {
    run_server().await;
}
