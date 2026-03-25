#!/usr/bin/env bash
# install.sh — Build and install tabby-zj Zellij plugin
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PLUGIN_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/zellij/plugins"
LAYOUT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/zellij/layouts"
CONFIG_DIR="${TABBY_ZJ_CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/tabby-zj}"

echo "Building tabby-zj (release)..."
cd "$SCRIPT_DIR"
cargo build --release --target wasm32-wasip1

echo "Installing plugin to $PLUGIN_DIR..."
mkdir -p "$PLUGIN_DIR"
cp target/wasm32-wasip1/release/tabby_zj.wasm "$PLUGIN_DIR/tabby-zj.wasm"

echo "Installing layout to $LAYOUT_DIR..."
mkdir -p "$LAYOUT_DIR"
cp tabby-zj.kdl "$LAYOUT_DIR/tabby-zj.kdl"

echo "Installing default config to $CONFIG_DIR..."
mkdir -p "$CONFIG_DIR"
if [ ! -f "$CONFIG_DIR/config.yaml" ]; then
    cp config.yaml "$CONFIG_DIR/config.yaml"
    echo "  Wrote default config: $CONFIG_DIR/config.yaml"
else
    echo "  Config already exists (not overwritten): $CONFIG_DIR/config.yaml"
fi

echo ""
echo "✓ tabby-zj installed successfully!"
echo ""
echo "Launch Zellij with the sidebar:"
echo "  zellij --layout tabby-zj"
echo ""
echo "Dev loop (hot-reload while editing):"
echo "  cargo watch -x 'build --target wasm32-wasip1' \\"
echo "    -s 'zellij action start-or-reload-plugin file:target/wasm32-wasip1/debug/tabby_zj.wasm'"
