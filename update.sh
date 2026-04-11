#!/usr/bin/env bash
# update.sh — kctop updater
# pulls latest changes from git and rebuilds the binary

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
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo ""
printf "  ${CYAN}${BOLD}kctop${RESET} — updater\n"
echo "  ─────────────────────────────────────────"
echo ""

# ── Check git ─────────────────────────────────────────────────────────────────
info "Checking git..."
command -v git &>/dev/null || error "git not found — install git first"
success "git found"

# ── Check we're in a git repo ─────────────────────────────────────────────────
cd "$SCRIPT_DIR"
git rev-parse --git-dir &>/dev/null || error "Not a git repository — clone from GitHub first"

# ── Show current version ──────────────────────────────────────────────────────
CURRENT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
CURRENT_VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
info "Current version: ${CURRENT_VERSION} (${CURRENT_COMMIT})"

# ── Pull latest ───────────────────────────────────────────────────────────────
info "Pulling latest changes..."
git fetch origin 2>/dev/null

LOCAL=$(git rev-parse HEAD)
REMOTE=$(git rev-parse @{u} 2>/dev/null || echo "")

if [[ -z "$REMOTE" ]]; then
    warn "No upstream branch set — trying git pull anyway"
elif [[ "$LOCAL" == "$REMOTE" ]]; then
    success "Already up to date! (${CURRENT_VERSION})"
    echo ""
    exit 0
fi

git pull --rebase 2>&1 | tail -3

NEW_COMMIT=$(git rev-parse --short HEAD)
NEW_VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
success "Updated to ${NEW_VERSION} (${NEW_COMMIT})"

# ── Check cargo ───────────────────────────────────────────────────────────────
info "Checking cargo..."
command -v cargo &>/dev/null || error "cargo not found — install Rust from https://rustup.rs"
success "cargo $(cargo --version | cut -d' ' -f2) found"

# ── Rebuild ───────────────────────────────────────────────────────────────────
info "Rebuilding kctop (release)..."
cargo build --release 2>&1 | grep -E "Compiling kctop|Finished|^error" || true
[[ -f "${SCRIPT_DIR}/target/release/kctop" ]] || \
    error "Build failed — run 'cargo build --release' for full output"
success "Build complete"

# ── Replace binary ────────────────────────────────────────────────────────────
info "Installing updated binary..."
mkdir -p "$BIN_DIR"
cp "${SCRIPT_DIR}/target/release/kctop" "$BIN_PATH"
chmod +x "$BIN_PATH"
success "Binary updated"

# ── Done ──────────────────────────────────────────────────────────────────────
echo ""
echo "  ─────────────────────────────────────────"
success "${BOLD}kctop updated to ${NEW_VERSION}!${RESET}"
echo ""
printf "  Run it with: ${CYAN}${BOLD}kctop${RESET}\n"
echo ""