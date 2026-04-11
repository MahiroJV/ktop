#!/usr/bin/env bash
# install.sh — kctop installer
# works on any Linux distro — Ubuntu, Fedora, Arch, Debian, openSUSE etc.

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
printf "  ${CYAN}${BOLD}kctop${RESET} — koktail claude's top  ${BOLD}installer${RESET}\n"
echo "  ─────────────────────────────────────────"
echo ""

# ── OS detection ──────────────────────────────────────────────────────────────
if [[ -f /etc/os-release ]]; then
    source /etc/os-release
    info "Detected: ${NAME} ${VERSION_ID:-}"
elif [[ "$(uname)" != "Linux" ]]; then
    error "kctop only supports Linux"
fi

# ── Check bash version ────────────────────────────────────────────────────────
if [[ "${BASH_VERSINFO[0]}" -lt 4 ]]; then
    error "bash 4+ required (you have ${BASH_VERSION})"
fi

# ── Check cargo ───────────────────────────────────────────────────────────────
info "Checking cargo..."
if ! command -v cargo &>/dev/null; then
    error "cargo not found — install Rust from https://rustup.rs\n  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi
success "cargo $(cargo --version | cut -d' ' -f2) found"

# ── Build release binary ──────────────────────────────────────────────────────
info "Building kctop (release)..."
cd "$SCRIPT_DIR"
cargo build --release 2>&1 | grep -E "Compiling kctop|Finished|^error" || true
[[ -f "${SCRIPT_DIR}/target/release/kctop" ]] || \
    error "Build failed — run 'cargo build --release' for full output"
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
if [[ -f "${SCRIPT_DIR}/assets/kctop.svg" ]]; then
    cp "${SCRIPT_DIR}/assets/kctop.svg" "$ICON_PATH"
    success "Icon installed"
elif [[ -f "${SCRIPT_DIR}/kctop.svg" ]]; then
    cp "${SCRIPT_DIR}/kctop.svg" "$ICON_PATH"
    success "Icon installed"
else
    warn "Icon file not found — skipping"
    ICON_PATH="utilities-system-monitor"  # fallback to system icon
fi

# ── Detect terminal emulator ──────────────────────────────────────────────────
info "Detecting terminal emulator..."
TERM_EXEC=""

# check $TERMINAL env var first (set by some WMs like i3, openbox)
if [[ -n "$TERMINAL" ]] && command -v "$TERMINAL" &>/dev/null; then
    TERM_EXEC="$TERMINAL"
fi

# otherwise scan common terminals
if [[ -z "$TERM_EXEC" ]]; then
    for term in \
        kitty alacritty wezterm foot \
        konsole gnome-terminal xfce4-terminal mate-terminal \
        tilix terminator lxterminal rxvt-unicode urxvt \
        st xterm uxterm; do
        if command -v "$term" &>/dev/null; then
            TERM_EXEC="$term"
            break
        fi
    done
fi

if [[ -z "$TERM_EXEC" ]]; then
    warn "No terminal emulator found — desktop entry will use x-terminal-emulator"
    TERM_EXEC="x-terminal-emulator"  # Debian/Ubuntu alternatives system
fi

success "Terminal: $TERM_EXEC"

# build Exec line — each terminal has different flags for "hold open after exit"
case "$TERM_EXEC" in
    kitty)              EXEC_LINE="kitty --hold ${BIN_PATH}" ;;
    alacritty)          EXEC_LINE="alacritty --hold -e ${BIN_PATH}" ;;
    wezterm)            EXEC_LINE="wezterm start -- ${BIN_PATH}" ;;
    foot)               EXEC_LINE="foot ${BIN_PATH}" ;;
    konsole)            EXEC_LINE="konsole --noclose -e ${BIN_PATH}" ;;
    gnome-terminal)     EXEC_LINE="gnome-terminal -- bash -c '${BIN_PATH}; exec bash'" ;;
    xfce4-terminal)     EXEC_LINE="xfce4-terminal --hold -e ${BIN_PATH}" ;;
    mate-terminal)      EXEC_LINE="mate-terminal --wait -e ${BIN_PATH}" ;;
    tilix)              EXEC_LINE="tilix -e ${BIN_PATH}" ;;
    terminator)         EXEC_LINE="terminator -x ${BIN_PATH}" ;;
    lxterminal)         EXEC_LINE="lxterminal --command=${BIN_PATH}" ;;
    rxvt-unicode|urxvt) EXEC_LINE="urxvt -e ${BIN_PATH}" ;;
    st)                 EXEC_LINE="st -e ${BIN_PATH}" ;;
    xterm|uxterm)       EXEC_LINE="xterm -hold -e ${BIN_PATH}" ;;
    x-terminal-emulator) EXEC_LINE="x-terminal-emulator -e ${BIN_PATH}" ;;
    *)                  EXEC_LINE="${TERM_EXEC} -e ${BIN_PATH}" ;;
esac

# ── Install .desktop entry ────────────────────────────────────────────────────
info "Creating desktop entry..."
mkdir -p "$DESKTOP_DIR"
cat > "$DESKTOP_PATH" << DESKTOP
[Desktop Entry]
Version=1.0
Type=Application
Name=kctop
GenericName=System Monitor
Comment=koktail claude's top — futuristic TUI system monitor
Exec=${EXEC_LINE}
Icon=${ICON_PATH}
Terminal=false
Categories=System;Monitor;GTK;
Keywords=cpu;memory;disk;network;monitor;top;htop;kctop;system;
StartupNotify=false
DESKTOP

chmod +x "$DESKTOP_PATH"

# refresh app launcher — command differs by distro
if command -v update-desktop-database &>/dev/null; then
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
fi
if command -v xdg-desktop-menu &>/dev/null; then
    xdg-desktop-menu forceupdate 2>/dev/null || true
fi

success "Desktop entry created"

# ── PATH check ────────────────────────────────────────────────────────────────
echo ""
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    warn "${BIN_DIR} is not in your PATH"
    echo ""

    # detect shell and suggest right config file
    SHELL_RC=""
    case "$(basename "$SHELL")" in
        bash) SHELL_RC="~/.bashrc" ;;
        zsh)  SHELL_RC="~/.zshrc" ;;
        fish) SHELL_RC="~/.config/fish/config.fish" ;;
        *)    SHELL_RC="~/.profile" ;;
    esac

    warn "Add this to your ${SHELL_RC}:"
    echo ""
    printf "      ${CYAN}export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}\n"
    echo ""
    warn "Then run: source ${SHELL_RC}"
    echo ""
fi

# ── Done ──────────────────────────────────────────────────────────────────────
echo "  ─────────────────────────────────────────"
printf "  ${GREEN}${BOLD}✓${RESET}  kctop uninstalled successfully!\n"
echo ""
printf "  Terminal:     ${CYAN}${BOLD}kctop${RESET}\n"
printf "  App launcher: search for ${CYAN}${BOLD}kctop${RESET}\n"
echo ""
