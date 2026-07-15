//! Matter LSP delivery binary (`matter-lsp` / `matter-lsp.exe`).
//!
//! Transport: **stdio only** (Content-Length framed JSON-RPC).
//! No network listener, shell, package install, or package resolver.

#[tokio::main]
async fn main() {
    // Banners must never be written to stdout (LSP protocol).
    // Errors may go to stderr if start fails before the server loop.
    matter_lsp::start_server().await;
}
