// test_plugin.rhai - Simple test plugin that always returns a diagnostic
// Usage: utf8dok check doc.adoc --plugin rules/test_plugin.rhai

let diagnostics = [];

// Always return a static diagnostic to prove the plugin runs
diagnostics.push(#{
    code: "PLUGIN001",
    severity: "info",
    message: "Plugin test: Successfully executed Rhai validation script",
    help: "This is a test diagnostic from a Rhai plugin"
});

// Count blocks in the document
let block_count = doc.blocks.len();
diagnostics.push(info("PLUGIN002", `Document contains ${block_count} blocks`));

diagnostics
