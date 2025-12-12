#!/bin/bash
# claim-crate.sh - Claim crate names on crates.io
#
# This script helps claim the utf8dok crate names on crates.io
# by performing a dry-run publish and then the actual publish.
#
# Prerequisites:
# - cargo login (authenticate with crates.io)
# - All crate metadata complete (description, license, etc.)
#
# Usage:
#   ./claim-crate.sh          # Dry-run only
#   ./claim-crate.sh publish  # Actually publish

set -e

CRATES=(
    "utf8dok-ast"
    "utf8dok-core"
    "utf8dok-cli"
    "utf8dok-wasm"
)

echo "utf8dok - Crate Publishing Script"
echo "================================="
echo

# Check if we should actually publish
PUBLISH=false
if [ "$1" = "publish" ]; then
    PUBLISH=true
    echo "MODE: Publishing to crates.io"
else
    echo "MODE: Dry-run only (pass 'publish' argument to actually publish)"
fi
echo

for crate in "${CRATES[@]}"; do
    echo "Processing: $crate"
    echo "-------------------"

    cd "crates/$crate"

    echo "Running dry-run..."
    if cargo publish --dry-run; then
        echo "✓ Dry-run succeeded for $crate"

        if [ "$PUBLISH" = true ]; then
            echo "Publishing $crate..."
            cargo publish
            echo "✓ Published $crate"
            # Wait between publishes to avoid rate limiting
            echo "Waiting 30 seconds before next publish..."
            sleep 30
        fi
    else
        echo "✗ Dry-run failed for $crate"
        exit 1
    fi

    cd ../..
    echo
done

echo "================================="
if [ "$PUBLISH" = true ]; then
    echo "All crates published successfully!"
else
    echo "All dry-runs completed successfully!"
    echo "Run './claim-crate.sh publish' to publish to crates.io"
fi
