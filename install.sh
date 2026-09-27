#!/usr/bin/env bash
# Installation script for Cosmic Byte Spectrum Linux Suite (CLI + Web GUI)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== Cosmic Byte Spectrum Linux Suite Installer ==="

# Build release binaries if needed
echo "[1/4] Ensuring release binaries are up-to-date..."
cargo build --release

# Determine installation targets
if [ "$EUID" -eq 0 ]; then
    BIN_DIR="/usr/local/bin"
    ICON_DIR="/usr/share/icons/hicolor/scalable/apps"
    DESKTOP_DIR="/usr/share/applications"
    UDEV_DIR="/etc/udev/rules.d"
else
    BIN_DIR="$HOME/.local/bin"
    ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
    DESKTOP_DIR="$HOME/.local/share/applications"
    UDEV_DIR=""
fi

mkdir -p "$BIN_DIR" "$ICON_DIR" "$DESKTOP_DIR"

echo "[2/4] Installing binaries to $BIN_DIR..."
install -m 755 "$SCRIPT_DIR/target/release/spectrumctl" "$BIN_DIR/spectrumctl"
install -m 755 "$SCRIPT_DIR/target/release/spectrum-gui" "$BIN_DIR/spectrum-gui"

echo "[3/4] Installing desktop entry and icon..."
install -m 644 "$SCRIPT_DIR/packaging/desktop/cosmic-byte-spectrum.svg" "$ICON_DIR/cosmic-byte-spectrum.svg"

# Modify desktop file Exec path if ~/.local/bin is used and not in standard path
DESKTOP_SRC="$SCRIPT_DIR/packaging/desktop/cosmic-byte-spectrum.desktop"
if [ "$EUID" -ne 0 ]; then
    sed "s|Exec=spectrum-gui|Exec=$BIN_DIR/spectrum-gui|g" "$DESKTOP_SRC" > "$DESKTOP_DIR/cosmic-byte-spectrum.desktop"
else
    install -m 644 "$DESKTOP_SRC" "$DESKTOP_DIR/cosmic-byte-spectrum.desktop"
fi

if [ -n "$UDEV_DIR" ]; then
    echo "[4/4] Installing udev rules to $UDEV_DIR..."
    install -m 644 "$SCRIPT_DIR/packaging/udev/99-cosmic-byte-spectrum.rules" "$UDEV_DIR/99-cosmic-byte-spectrum.rules"
    udevadm control --reload-rules && udevadm trigger --subsystem-match=hidraw || true
else
    echo "[4/4] Note: Run 'sudo cp packaging/udev/99-cosmic-byte-spectrum.rules /etc/udev/rules.d/ && sudo udevadm control --reload-rules' to apply persistent udev rules for all users."
fi

# Refresh desktop database if tool is present
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
fi

echo ""
echo "============================================================"
echo " Installation Complete!"
echo " Binaries installed:"
echo "   - CLI:     $BIN_DIR/spectrumctl"
echo "   - GUI App: $BIN_DIR/spectrum-gui"
echo ""
echo " Quick Usage:"
echo "   - Launch GUI:          spectrum-gui"
echo "   - Or from project:     ./run-gui.sh"
echo "   - CLI Status:          spectrumctl get"
echo "   - Set Polling (1000):  spectrumctl polling set 1000"
echo "   - Set DPI (800):       spectrumctl dpi set 800"
echo "   - Set RGB (Neon):      spectrumctl rgb neon"
echo "============================================================"
