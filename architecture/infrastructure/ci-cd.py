#!/usr/bin/env python3
"""
utf8dok CI/CD Infrastructure as Code

This script generates and manages CI/CD pipeline configurations.
Currently supports GitHub Actions, with extensibility for other platforms.

Usage:
    python ci-cd.py generate    # Generate CI configuration
    python ci-cd.py validate    # Validate existing configuration
"""

import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass
class CIConfig:
    """CI/CD Configuration for utf8dok"""

    rust_version: str = "stable"
    rust_msrv: str = "1.70.0"  # Minimum Supported Rust Version
    platforms: tuple = ("ubuntu-latest", "macos-latest", "windows-latest")

    def to_github_actions(self) -> dict[str, Any]:
        """Generate GitHub Actions workflow configuration"""
        return {
            "name": "CI",
            "on": {
                "push": {"branches": ["main"]},
                "pull_request": {"branches": ["main"]},
            },
            "env": {
                "CARGO_TERM_COLOR": "always",
            },
            "jobs": {
                "test": {
                    "name": "Test",
                    "runs-on": "${{ matrix.os }}",
                    "strategy": {
                        "matrix": {
                            "os": list(self.platforms),
                            "rust": [self.rust_version, self.rust_msrv],
                        }
                    },
                    "steps": [
                        {"uses": "actions/checkout@v4"},
                        {
                            "name": "Install Rust",
                            "uses": "dtolnay/rust-toolchain@master",
                            "with": {"toolchain": "${{ matrix.rust }}"},
                        },
                        {"name": "Build", "run": "cargo build --workspace"},
                        {"name": "Test", "run": "cargo test --workspace"},
                    ],
                },
                "lint": {
                    "name": "Lint",
                    "runs-on": "ubuntu-latest",
                    "steps": [
                        {"uses": "actions/checkout@v4"},
                        {
                            "name": "Install Rust",
                            "uses": "dtolnay/rust-toolchain@stable",
                            "with": {"components": "rustfmt, clippy"},
                        },
                        {"name": "Format", "run": "cargo fmt --all -- --check"},
                        {"name": "Clippy", "run": "cargo clippy --workspace -- -D warnings"},
                    ],
                },
                "wasm": {
                    "name": "WASM Build",
                    "runs-on": "ubuntu-latest",
                    "steps": [
                        {"uses": "actions/checkout@v4"},
                        {
                            "name": "Install Rust",
                            "uses": "dtolnay/rust-toolchain@stable",
                            "with": {"targets": "wasm32-unknown-unknown"},
                        },
                        {
                            "name": "Install wasm-pack",
                            "run": "cargo install wasm-pack",
                        },
                        {
                            "name": "Build WASM",
                            "run": "wasm-pack build crates/utf8dok-wasm --target web",
                        },
                    ],
                },
            },
        }


def generate_workflow(output_path: Path) -> None:
    """Generate GitHub Actions workflow file"""
    config = CIConfig()
    workflow = config.to_github_actions()

    # Convert to YAML-like format (simplified)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    print(f"Generated CI configuration for: {', '.join(config.platforms)}")
    print(f"Rust versions: {config.rust_version}, MSRV: {config.rust_msrv}")
    print(f"Output would be written to: {output_path}")


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 1

    command = sys.argv[1]

    if command == "generate":
        output = Path(".github/workflows/ci.yml")
        generate_workflow(output)
        return 0
    elif command == "validate":
        print("Validation not yet implemented")
        return 0
    else:
        print(f"Unknown command: {command}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
