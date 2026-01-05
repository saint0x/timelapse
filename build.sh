#!/bin/bash
#
# Timelapse (tl) Build Script
# Build and install tl binary
#

set -e

# =============================================================================
# Environment Configuration (required for jj-lib on macOS)
# =============================================================================

export OPENSSL_DIR="/opt/homebrew/opt/openssl@3"
export PKG_CONFIG_PATH="/opt/homebrew/opt/openssl@3/lib/pkgconfig:/opt/homebrew/opt/libssh2/lib/pkgconfig"
export RUSTFLAGS="-L /opt/homebrew/opt/openssl@3/lib -L /opt/homebrew/opt/libssh2/lib"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

print_header() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

print_success() { echo -e "${GREEN}✓ $1${NC}"; }
print_error() { echo -e "${RED}✗ $1${NC}"; }
print_warning() { echo -e "${YELLOW}! $1${NC}"; }
print_info() { echo -e "${CYAN}→ $1${NC}"; }

start_timer() { START_TIME=$(date +%s); }
end_timer() {
    END_TIME=$(date +%s)
    ELAPSED=$((END_TIME - START_TIME))
    print_info "Completed in ${ELAPSED}s"
}

# =============================================================================
# Build Commands
# =============================================================================

cmd_check() {
    print_header "Checking (fast compilation check)"
    start_timer
    cargo check --all-targets
    end_timer
    print_success "Check passed"
}

cmd_debug() {
    print_header "Building debug binary"
    start_timer
    cargo build
    end_timer
    print_success "Debug build: target/aarch64-apple-darwin/debug/tl"
}

cmd_release() {
    print_header "Building release binary"
    start_timer
    cargo build --release
    end_timer
    print_success "Release build: target/aarch64-apple-darwin/release/tl"
}

cmd_install() {
    print_header "Installing release binary to ~/.cargo/bin"
    start_timer
    cargo build --release
    cargo install --path crates/cli --force
    end_timer
    print_success "Installed to ~/.cargo/bin/tl"

    # Verify
    if command -v tl &>/dev/null; then
        print_info "Version: $(tl --version)"
        print_info "Location: $(which tl)"
    else
        echo ""
        print_error "tl not found in PATH. Add to your shell config:"
        echo '  export PATH="$HOME/.cargo/bin:$PATH"'
    fi
}

cmd_clean() {
    print_header "Cleaning build artifacts"
    cargo clean
    print_success "Clean completed"
}

cmd_info() {
    print_header "Build Info"
    echo "Current tl in PATH:"
    if command -v tl &>/dev/null; then
        print_info "Location: $(which tl)"
        print_info "Version:  $(tl --version)"
    else
        print_warning "tl not found in PATH"
    fi
    echo ""
    echo "Local builds:"
    [ -f "target/aarch64-apple-darwin/release/tl" ] && print_info "Release: target/aarch64-apple-darwin/release/tl"
    [ -f "target/aarch64-apple-darwin/debug/tl" ] && print_info "Debug:   target/aarch64-apple-darwin/debug/tl"
    [ -f "target/release/tl" ] && print_info "Release: target/release/tl"
    [ -f "target/debug/tl" ] && print_info "Debug:   target/debug/tl"
}

# =============================================================================
# Usage
# =============================================================================

show_usage() {
    echo ""
    echo -e "${CYAN}Timelapse Build Script${NC}"
    echo ""
    echo "Usage: ./build.sh <command>"
    echo ""
    echo -e "${YELLOW}Commands:${NC}"
    echo "  check      Fast compilation check (no codegen)"
    echo "  debug      Build debug binary (fast, unoptimized)"
    echo "  release    Build release binary (slow, optimized)"
    echo "  install    Build release + install to ~/.cargo/bin"
    echo "  clean      Remove build artifacts"
    echo "  info       Show current tl binary info"
    echo ""
    echo -e "${YELLOW}Examples:${NC}"
    echo "  ./build.sh check      # Quick syntax check"
    echo "  ./build.sh debug      # Fast dev build"
    echo "  ./build.sh release    # Optimized build"
    echo "  ./build.sh install    # Build + install to PATH"
    echo ""
}

# =============================================================================
# Main
# =============================================================================

main() {
    cd "$(dirname "$0")"

    case "${1:-}" in
        check)      cmd_check ;;
        debug)      cmd_debug ;;
        release)    cmd_release ;;
        install)    cmd_install ;;
        clean)      cmd_clean ;;
        info)       cmd_info ;;
        -h|--help|help|"")
            show_usage
            ;;
        *)
            print_error "Unknown command: $1"
            show_usage
            exit 1
            ;;
    esac
}

main "$@"
