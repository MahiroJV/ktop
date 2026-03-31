#!/usr/bin/env bash
# install.sh — ktop Rust installer
# builds in release mode and installs to ~/.local/bin/ktop

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

info()    { printf "  ${CYAN}${BOLD}→${RESET}  %s\n" "$*"; }
success() { printf "  ${GREEN}${BOLD}✓${RESET}  %s\n" "$*"; }
warn()    { printf "  ${YELLOW}${BOLD}!${RESET}  %s\n" "$*"; }
error()   { printf "  ${RED}${BOLD}✗${RESET}  %s\n" "$*" >&2; exit 1; }

BIN_DIR="${HOME}/.local/bin"
BIN_PATH="${BIN_DIR}/ktop-r"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo ""
printf "  ${CYAN}${BOLD}ktop${RESET} — koktail's system monitor  ${BOLD}installer${RESET}\n"
echo "  ─────────────────────────────────────────"
echo ""

# ── Check cargo ───────────────────────────────────────────────────────────────
info "Checking cargo..."
command -v cargo &>/dev/null || error "cargo not found — install Rust from https://rustup.rs"
success "cargo $(cargo --version | cut -d' ' -f2) found"

# ── Build release binary ───────────────────────────────────────────────────────
info "Building ktop (release)..."
cd "$SCRIPT_DIR"
cargo build --release 2>&1 | grep -E "Compiling|Finished|error" || true

[[ -f "${SCRIPT_DIR}/target/release/ktop-r" ]] || error "build failed — run 'cargo build --release' to see full output"
success "Build complete"

# ── Install binary ────────────────────────────────────────────────────────────
info "Installing to ${BIN_PATH}..."
mkdir -p "$BIN_DIR"
cp "${SCRIPT_DIR}/target/release/ktop-r" "$BIN_PATH"
chmod +x "$BIN_PATH"
success "Binary installed"

# ── PATH check ────────────────────────────────────────────────────────────────
echo ""
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    warn "${BIN_DIR} is not in your PATH"
    warn "Add this to your ~/.bashrc or ~/.zshrc:"
    echo ""
    printf "      ${CYAN}export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}\n"
    echo ""
    warn "Then run: source ~/.bashrc"
    echo ""
fi

# ── Done ──────────────────────────────────────────────────────────────────────
echo "  ─────────────────────────────────────────"
success "${BOLD}ktop installed!${RESET}"
echo ""
printf "  Run it with: ${CYAN}${BOLD}ktop-r${RESET}\n"
echo ""