#!/bin/bash
# gix - Git Profile Manager
# Installation Script
# https://github.com/elmanci2/gix
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/elmanci2/gix/main/install.sh | bash
#
# Or with a specific version:
#   curl -fsSL https://raw.githubusercontent.com/elmanci2/gix/main/install.sh | bash -s -- v1.0.0

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# Configuration
REPO="elmanci2/gix"
BINARY_NAME="gix"
INSTALL_DIR="${GIX_INSTALL_DIR:-$HOME/.local/bin}"

# Print colored message
print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_header() {
    echo ""
    echo -e "${CYAN}${BOLD}🔀 gix - Git Profile Manager${NC}"
    echo -e "${CYAN}   Installation Script${NC}"
    echo ""
}

# Detect OS and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    case "$OS" in
        linux*)
            OS="linux"
            ;;
        darwin*)
            OS="darwin"
            ;;
        mingw*|msys*|cygwin*)
            OS="windows"
            ;;
        *)
            print_error "Unsupported operating system: $OS"
            exit 1
            ;;
    esac

    case "$ARCH" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        armv7l)
            ARCH="armv7"
            ;;
        *)
            print_error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac

    PLATFORM="${OS}-${ARCH}"
    print_info "Detected platform: ${BOLD}$PLATFORM${NC}"
}

# Check for required tools
check_requirements() {
    local missing_tools=()

    if ! command -v curl &> /dev/null && ! command -v wget &> /dev/null; then
        missing_tools+=("curl or wget")
    fi

    if ! command -v tar &> /dev/null; then
        missing_tools+=("tar")
    fi

    if [ ${#missing_tools[@]} -ne 0 ]; then
        print_error "Missing required tools: ${missing_tools[*]}"
        print_info "Please install them and try again."
        exit 1
    fi
}

# Get the latest version from GitHub
get_latest_version() {
    if [ -n "$1" ]; then
        VERSION="$1"
        print_info "Installing specified version: ${BOLD}$VERSION${NC}"
    else
        print_info "Fetching latest version..."
        
        if command -v curl &> /dev/null; then
            VERSION=$(curl -sS "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
        else
            VERSION=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
        fi

        if [ -z "$VERSION" ]; then
            print_warning "Could not fetch latest version, using v1.0.0"
            VERSION="v1.0.0"
        fi
        
        print_info "Latest version: ${BOLD}$VERSION${NC}"
    fi
}

# Download and install
download_and_install() {
    local DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/gix-${VERSION}-${PLATFORM}.tar.gz"
    local TMP_DIR=$(mktemp -d)
    local TMP_FILE="${TMP_DIR}/gix.tar.gz"

    print_info "Downloading from: $DOWNLOAD_URL"

    # Download
    if command -v curl &> /dev/null; then
        if ! curl -fsSL "$DOWNLOAD_URL" -o "$TMP_FILE" 2>/dev/null; then
            print_warning "Pre-built binary not found. Attempting to build from source..."
            build_from_source
            return
        fi
    else
        if ! wget -q "$DOWNLOAD_URL" -O "$TMP_FILE" 2>/dev/null; then
            print_warning "Pre-built binary not found. Attempting to build from source..."
            build_from_source
            return
        fi
    fi

    # Extract
    print_info "Extracting..."
    tar -xzf "$TMP_FILE" -C "$TMP_DIR"

    # Create install directory
    mkdir -p "$INSTALL_DIR"

    # Install binary
    if [ -f "${TMP_DIR}/gix" ]; then
        mv "${TMP_DIR}/gix" "${INSTALL_DIR}/${BINARY_NAME}"
        chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    elif [ -f "${TMP_DIR}/${BINARY_NAME}" ]; then
        mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
        chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    else
        print_error "Binary not found in archive"
        rm -rf "$TMP_DIR"
        exit 1
    fi

    # Cleanup
    rm -rf "$TMP_DIR"

    print_success "Installed to: ${BOLD}${INSTALL_DIR}/${BINARY_NAME}${NC}"
}

# Build from source if pre-built binary not available
build_from_source() {
    print_info "Building from source..."

    # Check for Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust first:"
        echo ""
        echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo ""
        exit 1
    fi

    print_info "Installing via cargo..."
    cargo install --git "https://github.com/${REPO}" --force

    print_success "Installed via cargo!"
}

# Add to PATH if needed
setup_path() {
    # Check if already in PATH
    if [[ ":$PATH:" == *":$INSTALL_DIR:"* ]]; then
        print_success "Install directory already in PATH"
        return
    fi

    print_warning "Install directory not in PATH"
    
    # Detect shell and config file
    local SHELL_NAME=$(basename "$SHELL")
    local SHELL_CONFIG=""

    case "$SHELL_NAME" in
        bash)
            if [ -f "$HOME/.bashrc" ]; then
                SHELL_CONFIG="$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                SHELL_CONFIG="$HOME/.bash_profile"
            fi
            ;;
        zsh)
            SHELL_CONFIG="$HOME/.zshrc"
            ;;
        fish)
            SHELL_CONFIG="$HOME/.config/fish/config.fish"
            ;;
    esac

    if [ -n "$SHELL_CONFIG" ]; then
        echo "" >> "$SHELL_CONFIG"
        echo "# gix - Git Profile Manager" >> "$SHELL_CONFIG"
        echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$SHELL_CONFIG"
        print_success "Added to PATH in $SHELL_CONFIG"
        print_info "Run 'source $SHELL_CONFIG' or restart your terminal"
    else
        print_warning "Could not detect shell config file"
        print_info "Add the following to your shell config:"
        echo ""
        echo "    export PATH=\"\$PATH:$INSTALL_DIR\""
        echo ""
    fi
}

# Verify installation
verify_installation() {
    echo ""
    if [ -x "${INSTALL_DIR}/${BINARY_NAME}" ]; then
        print_success "Installation successful!"
        echo ""
        
        # Try to run version command
        if command -v gix &> /dev/null; then
            gix version
        else
            print_info "Note: You may need to restart your terminal or run:"
            echo ""
            echo "    source ~/.bashrc  # or ~/.zshrc"
            echo ""
        fi
    else
        print_error "Installation may have failed. Binary not found."
        exit 1
    fi
}

# Print post-install instructions
print_instructions() {
    echo ""
    echo -e "${CYAN}${BOLD}Quick Start:${NC}"
    echo ""
    echo "  1. Add a profile:"
    echo "     ${BOLD}gix profile add${NC}"
    echo ""
    echo "  2. Use a profile in a repository:"
    echo "     ${BOLD}cd your-repo && gix use${NC}"
    echo ""
    echo "  3. Run git commands through gix:"
    echo "     ${BOLD}gix push${NC}"
    echo ""
    echo "  For more help:"
    echo "     ${BOLD}gix --help${NC}"
    echo ""
}

# Uninstall function
uninstall() {
    print_header
    print_info "Uninstalling gix..."

    if [ -f "${INSTALL_DIR}/${BINARY_NAME}" ]; then
        rm -f "${INSTALL_DIR}/${BINARY_NAME}"
        print_success "Removed ${INSTALL_DIR}/${BINARY_NAME}"
    fi

    if [ -d "$HOME/.gix" ]; then
        read -p "Remove configuration directory (~/.gix)? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$HOME/.gix"
            print_success "Removed ~/.gix"
        fi
    fi

    print_success "gix has been uninstalled"
}

# Main
main() {
    print_header

    # Check for uninstall flag
    if [ "$1" = "--uninstall" ] || [ "$1" = "-u" ]; then
        uninstall
        exit 0
    fi

    # Check requirements
    check_requirements

    # Detect platform
    detect_platform

    # Get version (use argument if provided)
    get_latest_version "$1"

    # Download and install
    download_and_install

    # Setup PATH
    setup_path

    # Verify
    verify_installation

    # Print instructions
    print_instructions
}

main "$@"
