#!/usr/bin/env bash
# uninstall.sh — kctop uninstaller

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

info()    { printf "  ${CYAN}${BOLD}→${RESET}  %s\n" "$*"; }
success() { printf "  ${GREEN}${BOLD}✓${RESET}  %s\n" "$*"; }
warn()    { printf "  ${YELLOW}${BOLD}!${RESET}  %s\n" "$*"; }
error()   { printf "  ${RED}${BOLD}✗${RESET}  %s\n" "$*"; }

BIN_PATH="${HOME}/.local/bin/kctop"
DESKTOP_PATH="${HOME}/.local/share/applications/kctop.desktop"
ICON_PATH="${HOME}/.local/share/icons/kctop.png"

echo ""
printf "  ${CYAN}${BOLD}kctop${RESET} — uninstaller\n"
echo "  ─────────────────────────────────────────"
echo ""

removed=0

# ── Binary ────────────────────────────────────────────────────────────────────
info "Removing binary..."
if [[ -f "$BIN_PATH" ]]; then
    rm -f "$BIN_PATH"
    success "Removed ${BIN_PATH}"
    removed=1
else
    warn "Binary not found at ${BIN_PATH}"
fi

# ── Desktop entry ─────────────────────────────────────────────────────────────
info "Removing desktop entry..."
if [[ -f "$DESKTOP_PATH" ]]; then
    rm -f "$DESKTOP_PATH"
    # refresh app launcher cache
    update-desktop-database "${HOME}/.local/share/applications" 2>/dev/null || true
    success "Removed ${DESKTOP_PATH}"
    removed=1
else
    warn "Desktop entry not found at ${DESKTOP_PATH}"
fi

# ── Icon ──────────────────────────────────────────────────────────────────────
info "Removing icon..."
if [[ -f "$ICON_PATH" ]]; then
    rm -f "$ICON_PATH"
    success "Removed ${ICON_PATH}"
    removed=1
else
    warn "Icon not found at ${ICON_PATH}"
fi

echo ""
echo "  ─────────────────────────────────────────"
if [[ $removed -eq 1 ]]; then
    success "${BOLD}kctop uninstalled successfully!${RESET}"
else
    warn "Nothing was removed — kctop may not have been installed."
fi
echo ""