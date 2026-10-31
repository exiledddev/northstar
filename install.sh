#!/usr/bin/env bash
# Build Blackline and install it for the current user.
# Tested target: Pop!_OS / Debian / Ubuntu with KDE Plasma or GNOME.
#
#   ./install.sh              build and install (or upgrade in place)
#   ./install.sh --uninstall  remove the app; your scripts are left alone
#
# Run from a release download (the `blackline` binary sits next to this
# script, with no source), it installs that binary and builds nothing.
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

# The library: one made when the app was called Northstar stays where it is
# and keeps being used.
SHARE="${XDG_DATA_HOME:-$HOME/.local/share}"
DATA_DIR="$SHARE/blackline"
if [ ! -e "$DATA_DIR" ] && [ -d "$SHARE/northstar" ]; then
  DATA_DIR="$SHARE/northstar"
fi

# what the app was installed as before it was called Blackline
remove_northstar() {
  rm -f "$BIN_DIR/northstar" "$APP_DIR/northstar.desktop" "$ICON_DIR/northstar.svg"
}

if [ "${1:-}" = "--uninstall" ]; then
  rm -f "$BIN_DIR/blackline" "$APP_DIR/blackline.desktop" "$ICON_DIR/blackline.svg"
  remove_northstar
  refresh_desktop
  say "Blackline removed. Your scripts are still in $DATA_DIR"
  exit 0
fi

# A release download carries the binary; a checkout of the source builds it.
PREBUILT=0
if [ -x "$here/blackline" ] && [ ! -f "$here/Cargo.toml" ]; then
  PREBUILT=1
  BUILT="$here/blackline"
else
  BUILT="target/release/blackline"
fi

if [ "$PREBUILT" = 0 ]; then
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
fi

# ---- keep a copy of the library ----------------------------------------
# Before a new build goes in, the whole library (scripts, snapshots,
# settings) is copied aside. Copied only: nothing in it is moved, changed or
# deleted, and the copy is never touched again. Delete old copies by hand when
# you no longer want them.
if [ -d "$DATA_DIR" ]; then
  BACKUP="${DATA_DIR}-backup-$(date +%Y%m%d-%H%M%S)"
  cp -a "$DATA_DIR" "$BACKUP"
  say "Your library was copied to $BACKUP first"
fi

# ---- install ------------------------------------------------------------
mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"
remove_northstar
install -m 755 "$BUILT" "$BIN_DIR/blackline"
install -m 644 packaging/blackline.desktop "$APP_DIR/blackline.desktop"

# The launcher icon is drawn by the same code as the mark inside the app, so
# the two can never drift apart.
say "Drawing the icon"
"$BIN_DIR/blackline" --emit-icon "$ICON_DIR/blackline.svg" >/dev/null
install -m 644 "$ICON_DIR/blackline.svg" packaging/blackline.svg

refresh_desktop

say "Installed to $BIN_DIR/blackline"
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
  say "Note: $BIN_DIR is not on your PATH. Add this to ~/.bashrc or ~/.zshrc:"
  echo '    export PATH="$HOME/.local/bin:$PATH"'
fi
say "Scripts live in $DATA_DIR/scripts"
say "Launch it from the application menu, or run: blackline"
