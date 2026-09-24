#!/usr/bin/env bash
# Build Northstar and install it for the current user.
# Tested target: Pop!_OS / Debian / Ubuntu with KDE Plasma or GNOME.
#
#   ./install.sh              build and install (or upgrade in place)
#   ./install.sh --uninstall  remove the app; your scripts are left alone
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

say() { printf '\033[1;31m::\033[0m %s\n' "$*"; }

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"

refresh_desktop() {
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true
  command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
  # KDE Plasma reads its menu from a cache of its own
  command -v kbuildsycoca6 >/dev/null 2>&1 && kbuildsycoca6 >/dev/null 2>&1 || true
  command -v kbuildsycoca5 >/dev/null 2>&1 && kbuildsycoca5 >/dev/null 2>&1 || true
}

if [ "${1:-}" = "--uninstall" ]; then
  rm -f "$BIN_DIR/northstar" "$APP_DIR/northstar.desktop" "$ICON_DIR/northstar.svg"
  refresh_desktop
  say "Northstar removed. Your scripts are still in ~/.local/share/northstar"
  exit 0
fi

# ---- build dependencies -------------------------------------------------
PKGS=(
  build-essential pkg-config
  libgl1-mesa-dev
  libx11-dev libxcursor-dev libxrandr-dev libxi-dev
  libxkbcommon-dev libxkbcommon-x11-dev
  libwayland-dev
)
missing=()
for p in "${PKGS[@]}"; do
  dpkg -s "$p" >/dev/null 2>&1 || missing+=("$p")
done
if [ ${#missing[@]} -gt 0 ]; then
  say "Installing build dependencies: ${missing[*]}"
  # a stale third-party repository makes `apt-get update` fail as a whole;
  # the packages needed here all come from the distribution itself
  sudo apt-get update || say "apt-get update reported errors (often a stale PPA); carrying on"
  sudo apt-get install -y "${missing[@]}"
fi

# ---- rust ---------------------------------------------------------------
if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
  cat <<'EOF'
Rust was not found. Install it with rustup (recommended over apt, which ships
an older toolchain):

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source "$HOME/.cargo/env"

Then run this script again.
EOF
  exit 1
fi

# ---- build --------------------------------------------------------------
say "Building (this takes a few minutes the first time)"
cargo build --release

# ---- install ------------------------------------------------------------
mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"
install -m 755 target/release/northstar "$BIN_DIR/northstar"
install -m 644 packaging/northstar.desktop "$APP_DIR/northstar.desktop"

# The launcher icon is drawn by the same code as the mark inside the app, so
# the two can never drift apart.
say "Drawing the icon"
"$BIN_DIR/northstar" --emit-icon "$ICON_DIR/northstar.svg" >/dev/null
install -m 644 "$ICON_DIR/northstar.svg" packaging/northstar.svg

refresh_desktop

say "Installed to $BIN_DIR/northstar"
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
  say "Note: $BIN_DIR is not on your PATH. Add this to ~/.bashrc or ~/.zshrc:"
  echo '    export PATH="$HOME/.local/bin:$PATH"'
fi
say "Scripts live in ~/.local/share/northstar/scripts"
if [ -f "$HOME/.local/share/tesseract/settings.conf" ]; then
  say "Tesseract found — Northstar will wear the same theme (Settings → Match Tesseract)."
fi
say "Launch it from the application menu, or run: northstar"
