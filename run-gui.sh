#!/usr/bin/env bash
# Quick launcher for Cosmic Byte Spectrum Control Center GUI

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$SCRIPT_DIR/target/release/spectrum-gui"

if [ ! -f "$BIN" ]; then
    echo "[*] Building spectrum-gui release binary..."
    cargo build --release -p spectrum-ui
fi

echo "============================================================"
echo " Starting Cosmic Byte Spectrum Control Center..."
echo " Web UI will automatically open in your default browser."
echo " Dashboard URL: http://127.0.0.1:4567"
echo " (Press Ctrl+C to stop)"
echo "============================================================"

exec "$BIN" "$@"
