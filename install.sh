#!/bin/sh
# Install or remove touchpad-haptic.
#   ./install.sh               install CLI and systemd service
#   ./install.sh --uninstall   remove them, keep the saved level
#   ./install.sh --purge       remove them and the saved level
set -eu

BIN=/usr/local/bin/touchpad-haptic
UNIT=/etc/systemd/system/touchpad-haptic.service
CONFIG=/etc/touchpad-haptic.conf
SRC=$(cd "$(dirname "$0")" && pwd)

case "${1:-}" in
    ""|--uninstall|--purge) ;;
    *) echo "usage: install.sh or touchpad-haptic-*.run [--uninstall | --purge]" >&2; exit 2 ;;
esac

if [ "$(id -u)" -ne 0 ]; then
    exec sudo sh "$SRC/install.sh" "$@"
fi

uninstall() {
    systemctl disable touchpad-haptic.service 2>/dev/null || true
    rm -f "$BIN" "$UNIT"
    systemctl daemon-reload
    echo "Removed $BIN and $UNIT"
}

case "${1:-}" in
    --uninstall)
        uninstall
        echo "Kept $CONFIG (use --purge to remove it)"
        ;;
    --purge)
        uninstall
        rm -f "$CONFIG"
        echo "Removed $CONFIG"
        ;;
    "")
        # .run payload ships the binary next to this script; a clone has it under target/.
        for f in "$SRC/touchpad-haptic" "$SRC/target/x86_64-unknown-linux-musl/release/touchpad-haptic" "$SRC/target/release/touchpad-haptic"; do
            [ -x "$f" ] && { BIN_SRC=$f; break; }
        done
        [ -n "${BIN_SRC:-}" ] || { echo "touchpad-haptic binary not found, run: cargo build --release" >&2; exit 1; }
        install -m 755 "$BIN_SRC" "$BIN"
        install -m 644 "$SRC/touchpad-haptic.service" "$UNIT"
        systemctl daemon-reload
        systemctl enable touchpad-haptic.service
        echo "Installed $BIN and $UNIT"
        "$BIN" status
        if [ ! -f "$CONFIG" ]; then
            echo
            echo "Choose a level with: touchpad-haptic set N   (0 = off, 100 = strongest)"
        fi
        ;;
esac
