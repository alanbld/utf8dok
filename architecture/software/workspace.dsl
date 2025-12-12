/*
 * utf8dok Architecture - Structurizr DSL
 *
 * This file defines the software architecture using the C4 model.
 * Visualize at: https://structurizr.com/dsl
 */

workspace "utf8dok" "A blazing-fast document processor for UTF-8 text formats" {

    model {
        user = person "Document Author" "Writes documentation in AsciiDoc format"
        developer = person "Developer" "Integrates utf8dok into applications"

        utf8dok = softwareSystem "utf8dok" "Document processor for UTF-8 text formats" {
            cli = container "utf8dok-cli" "Command-line interface" "Rust Binary" {
                tags "CLI"
            }

            core = container "utf8dok-core" "Core parsing and processing library" "Rust Library" {
                parser = component "Parser" "Parses AsciiDoc syntax using pest" "Rust Module"
                processor = component "Processor" "Transforms AST to output formats" "Rust Module"
            }

            ast = container "utf8dok-ast" "Abstract Syntax Tree definitions" "Rust Library" {
                nodes = component "AST Nodes" "Document structure types" "Rust Types"
                visitor = component "Visitor" "AST traversal pattern" "Rust Trait"
            }

            wasm = container "utf8dok-wasm" "WebAssembly bindings" "WASM Module" {
                tags "WASM"
            }
        }

        browser = softwareSystem "Web Browser" "Runs WASM module for client-side processing" {
            tags "External"
        }

        # Relationships
        user -> cli "Uses"
        developer -> core "Integrates"
        developer -> wasm "Embeds in web apps"

        cli -> core "Uses"
        wasm -> core "Uses"
        core -> ast "Defines structures with"

        browser -> wasm "Loads"
    }

    views {
        systemContext utf8dok "SystemContext" {
            include *
            autoLayout
        }

        container utf8dok "Containers" {
            include *
            autoLayout
        }

        component core "CoreComponents" {
            include *
            autoLayout
        }

        styles {
            element "Software System" {
                background #1168bd
                color #ffffff
            }
            element "Person" {
                shape person
                background #08427b
                color #ffffff
            }
            element "Container" {
                background #438dd5
                color #ffffff
            }
            element "Component" {
                background #85bbf0
                color #000000
            }
            element "External" {
                background #999999
                color #ffffff
            }
            element "CLI" {
                shape RoundedBox
            }
            element "WASM" {
                shape Hexagon
            }
        }
    }
}
