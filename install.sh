#!/usr/bin/env bash
# Build Northstar and install it for the current user.
# Tested target: Pop!_OS / Debian / Ubuntu with KDE Plasma or GNOME.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

say() { printf '\033[1;31m::\033[0m %s\n' "$*"; }

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
  sudo apt-get update
  sudo apt-get install -y "${missing[@]}"
fi

# ---- rust ---------------------------------------------------------------
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
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"
install -m 755 target/release/northstar "$BIN_DIR/northstar"
install -m 644 packaging/northstar.desktop "$APP_DIR/northstar.desktop"
install -m 644 packaging/northstar.svg "$ICON_DIR/northstar.svg"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

say "Installed to $BIN_DIR/northstar"
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
  say "Note: $BIN_DIR is not on your PATH. Add this to ~/.bashrc or ~/.zshrc:"
  echo '    export PATH="$HOME/.local/bin:$PATH"'
fi
say "Scripts will live in ~/.local/share/northstar/scripts"
say "Launch it from the application menu, or run: northstar"
