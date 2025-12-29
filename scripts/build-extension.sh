#!/bin/bash
#
# UTF8DOK VS Code Extension Build Script
#
# Usage:
#   ./scripts/build-extension.sh          # Build for current platform
#   ./scripts/build-extension.sh --all    # Build for all platforms (requires cross-compilation)
#   ./scripts/build-extension.sh --dev    # Development build (no binary bundling)
#
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
EXTENSION_DIR="$PROJECT_ROOT/editors/vscode"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Parse arguments
BUILD_ALL=false
DEV_MODE=false

for arg in "$@"; do
    case $arg in
        --all)
            BUILD_ALL=true
            shift
            ;;
        --dev)
            DEV_MODE=true
            shift
            ;;
        *)
            ;;
    esac
done

echo ""
echo "================================================"
echo "  UTF8DOK VS Code Extension Builder"
echo "================================================"
echo ""

# Step 1: Build Rust binaries
if [ "$DEV_MODE" = true ]; then
    log_info "Development mode: Skipping binary bundling"
else
    log_info "Building Rust LSP binary..."

    if [ "$BUILD_ALL" = true ]; then
        log_info "Building for all platforms (requires cross-compilation toolchains)"

        # Linux x64
        log_info "Building for Linux x64..."
        if command -v cross &> /dev/null; then
            cross build --release -p utf8dok-lsp --target x86_64-unknown-linux-gnu
        else
            cargo build --release -p utf8dok-lsp
        fi

        # macOS x64 (if on macOS or with cross)
        if [ "$(uname)" = "Darwin" ] || command -v cross &> /dev/null; then
            log_info "Building for macOS x64..."
            if command -v cross &> /dev/null; then
                cross build --release -p utf8dok-lsp --target x86_64-apple-darwin
            elif [ "$(uname)" = "Darwin" ]; then
                cargo build --release -p utf8dok-lsp --target x86_64-apple-darwin
            fi
        else
            log_warn "Skipping macOS x64 build (not on macOS and cross not available)"
        fi

        # macOS ARM64
        if [ "$(uname)" = "Darwin" ]; then
            log_info "Building for macOS ARM64..."
            cargo build --release -p utf8dok-lsp --target aarch64-apple-darwin
        else
            log_warn "Skipping macOS ARM64 build (not on macOS)"
        fi

        # Windows x64
        if command -v cross &> /dev/null; then
            log_info "Building for Windows x64..."
            cross build --release -p utf8dok-lsp --target x86_64-pc-windows-msvc
        else
            log_warn "Skipping Windows build (cross not available)"
        fi
    else
        # Build for current platform only
        log_info "Building for current platform..."
        cargo build --release -p utf8dok-lsp
    fi

    log_success "Rust build complete"
fi

# Step 2: Create extension directory structure
log_info "Setting up extension directory structure..."
mkdir -p "$EXTENSION_DIR/bin"/{linux-x64,darwin-x64,darwin-arm64,win32-x64}
mkdir -p "$EXTENSION_DIR/out"

# Step 3: Copy binaries
if [ "$DEV_MODE" = false ]; then
    log_info "Copying binaries..."

    # Detect current platform for single-platform build
    if [ "$BUILD_ALL" = false ]; then
        case "$(uname -s)" in
            Linux)
                if [ -f "$PROJECT_ROOT/target/release/utf8dok-lsp" ]; then
                    cp "$PROJECT_ROOT/target/release/utf8dok-lsp" "$EXTENSION_DIR/bin/linux-x64/"
                    log_success "Copied Linux x64 binary"
                fi
                ;;
            Darwin)
                if [ -f "$PROJECT_ROOT/target/release/utf8dok-lsp" ]; then
                    if [ "$(uname -m)" = "arm64" ]; then
                        cp "$PROJECT_ROOT/target/release/utf8dok-lsp" "$EXTENSION_DIR/bin/darwin-arm64/"
                        log_success "Copied macOS ARM64 binary"
                    else
                        cp "$PROJECT_ROOT/target/release/utf8dok-lsp" "$EXTENSION_DIR/bin/darwin-x64/"
                        log_success "Copied macOS x64 binary"
                    fi
                fi
                ;;
            MINGW*|MSYS*|CYGWIN*)
                if [ -f "$PROJECT_ROOT/target/release/utf8dok-lsp.exe" ]; then
                    cp "$PROJECT_ROOT/target/release/utf8dok-lsp.exe" "$EXTENSION_DIR/bin/win32-x64/"
                    log_success "Copied Windows x64 binary"
                fi
                ;;
        esac
    else
        # Copy all built binaries
        [ -f "$PROJECT_ROOT/target/x86_64-unknown-linux-gnu/release/utf8dok-lsp" ] && \
            cp "$PROJECT_ROOT/target/x86_64-unknown-linux-gnu/release/utf8dok-lsp" "$EXTENSION_DIR/bin/linux-x64/"

        [ -f "$PROJECT_ROOT/target/x86_64-apple-darwin/release/utf8dok-lsp" ] && \
            cp "$PROJECT_ROOT/target/x86_64-apple-darwin/release/utf8dok-lsp" "$EXTENSION_DIR/bin/darwin-x64/"

        [ -f "$PROJECT_ROOT/target/aarch64-apple-darwin/release/utf8dok-lsp" ] && \
            cp "$PROJECT_ROOT/target/aarch64-apple-darwin/release/utf8dok-lsp" "$EXTENSION_DIR/bin/darwin-arm64/"

        [ -f "$PROJECT_ROOT/target/x86_64-pc-windows-msvc/release/utf8dok-lsp.exe" ] && \
            cp "$PROJECT_ROOT/target/x86_64-pc-windows-msvc/release/utf8dok-lsp.exe" "$EXTENSION_DIR/bin/win32-x64/"
    fi
fi

# Step 4: Install npm dependencies
log_info "Installing npm dependencies..."
cd "$EXTENSION_DIR"

if [ ! -f "package-lock.json" ]; then
    npm install
else
    npm ci
fi

log_success "npm dependencies installed"

# Step 5: Compile TypeScript
log_info "Compiling TypeScript..."
npm run compile
log_success "TypeScript compilation complete"

# Step 6: Run linter (optional, non-blocking)
log_info "Running linter..."
npm run lint 2>/dev/null || log_warn "Linting completed with warnings"

# Step 7: Package extension
log_info "Packaging extension..."
npm run package

# Find the generated .vsix file
VSIX_FILE=$(ls -t "$EXTENSION_DIR"/*.vsix 2>/dev/null | head -1)

if [ -n "$VSIX_FILE" ]; then
    log_success "Extension packaged successfully!"
    echo ""
    echo "================================================"
    echo "  Build Complete!"
    echo "================================================"
    echo ""
    echo "  Output: $VSIX_FILE"
    echo ""
    echo "  To install locally:"
    echo "    code --install-extension $VSIX_FILE"
    echo ""
    echo "  To publish to marketplace:"
    echo "    cd $EXTENSION_DIR && npm run publish"
    echo ""
else
    log_error "Extension packaging failed"
    exit 1
fi
