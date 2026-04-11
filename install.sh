#!/usr/bin/env bash
# install.sh — kctop installer
# builds release binary, installs to ~/.local/bin,
# creates .desktop entry and icon so it appears in app launcher

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
BIN_PATH="${BIN_DIR}/kctop"
DESKTOP_DIR="${HOME}/.local/share/applications"
DESKTOP_PATH="${DESKTOP_DIR}/kctop.desktop"
ICON_DIR="${HOME}/.local/share/icons"
ICON_PATH="${ICON_DIR}/kctop.svg"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo ""
printf "  ${CYAN}${BOLD}kctop${RESET} — koktail's system monitor  ${BOLD}installer${RESET}\n"
echo "  ─────────────────────────────────────────"
echo ""

# ── Check cargo ───────────────────────────────────────────────────────────────
info "Checking cargo..."
command -v cargo &>/dev/null || error "cargo not found — install Rust from https://rustup.rs"
success "cargo $(cargo --version | cut -d' ' -f2) found"

# ── Build release binary ──────────────────────────────────────────────────────
info "Building kctop (release)..."
cd "$SCRIPT_DIR"
cargo build --release 2>&1 | grep -E "Compiling ktop|Finished|^error" || true
[[ -f "${SCRIPT_DIR}/target/release/kctop" ]] || error "Build failed — run 'cargo build --release' to see full output"
success "Build complete"

# ── Install binary ────────────────────────────────────────────────────────────
info "Installing binary to ${BIN_PATH}..."
mkdir -p "$BIN_DIR"
cp "${SCRIPT_DIR}/target/release/kctop" "$BIN_PATH"
chmod +x "$BIN_PATH"
success "Binary installed"

# ── Install icon ──────────────────────────────────────────────────────────────
info "Installing icon..."
mkdir -p "$ICON_DIR"
if [[ -f "${SCRIPT_DIR}/kctop.svg" ]]; then
    cp "${SCRIPT_DIR}/kctop.svg" "$ICON_PATH"
    success "Icon installed (SVG)"
else
    warn "Icon file kctop.svg not found — skipping"
fi

# ── Install .desktop entry ────────────────────────────────────────────────────
info "Creating desktop entry..."
mkdir -p "$DESKTOP_DIR"
cat > "$DESKTOP_PATH" << DESKTOP
[Desktop Entry]
Version=1.0
Type=Application
Name=kctop
GenericName=System Monitor
Comment=koktail's system monitor — futuristic TUI
Exec=bash -c 'kctop; read -p "Press Enter to close..."'
Icon=${ICON_PATH}
Terminal=true
Categories=System;Monitor;
Keywords=cpu;memory;disk;network;monitor;top;htop;
StartupNotify=false
DESKTOP
chmod +x "$DESKTOP_PATH"
# refresh app launcher
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
success "Desktop entry created"

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
success "${BOLD}kctop installed!${RESET}"
echo ""
printf "  Terminal:     ${CYAN}${BOLD}kctop${RESET}\n"
printf "  App launcher: search for ${CYAN}${BOLD}kctop${RESET} in your apps\n"
echo ""