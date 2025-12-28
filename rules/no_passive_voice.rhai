// no_passive_voice.rhai - Detect passive voice in documents
// Usage: utf8dok check doc.adoc --plugin rules/no_passive_voice.rhai

let diagnostics = [];

// Iterate through all blocks in the document
for block in doc.blocks {
    // Check paragraphs
    if block.Paragraph != () {
        for inline in block.Paragraph.inlines {
            if inline.Text != () {
                let text = inline.Text;
                if has_passive_pattern(text) {
                    diagnostics.push(#{
                        code: "STYLE001",
                        severity: "warning",
                        message: "Passive voice detected",
                        help: "Consider using active voice for clearer writing",
                        context: text
                    });
                }
            }
        }
    }
}

diagnostics
