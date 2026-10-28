#!/bin/sh
# Install or remove touchpad-haptic.
#   ./install.sh               install the CLI and the udev rule that applies the saved level at boot
#   ./install.sh --uninstall   remove them, keep the saved level
#   ./install.sh --purge       remove them and the saved level
set -eu

BIN=/usr/local/bin/touchpad-haptic
RULE=/etc/udev/rules.d/90-touchpad-haptic.rules
CONFIG=/etc/touchpad-haptic.conf
# Installed by v0.1.0; removed on upgrade and uninstall.
LEGACY_UNIT=/etc/systemd/system/touchpad-haptic.service
SRC=$(cd "$(dirname "$0")" && pwd)

case "${1:-}" in
    ""|--uninstall|--purge) ;;
    *) echo "usage: install.sh or touchpad-haptic-*.run [--uninstall | --purge]" >&2; exit 2 ;;
esac

if [ "$(id -u)" -ne 0 ]; then
    for elevator in sudo doas run0; do
        command -v "$elevator" >/dev/null && exec "$elevator" sh "$SRC/install.sh" "$@"
    done
    echo "root is required, and none of sudo, doas, run0 was found; run this as root" >&2
    exit 1
fi

reload_udev() {
    if command -v udevadm >/dev/null; then
        udevadm control --reload
    else
        echo "warning: udevadm not found; the saved level will not be applied at boot" >&2
    fi
}

remove_legacy_unit() {
    [ -f "$LEGACY_UNIT" ] || return 0
    systemctl disable touchpad-haptic.service 2>/dev/null || true
    rm -f "$LEGACY_UNIT"
    systemctl daemon-reload 2>/dev/null || true
    echo "Removed $LEGACY_UNIT (replaced by the udev rule)"
}

uninstall() {
    remove_legacy_unit
    rm -f "$BIN" "$RULE"
    reload_udev
    echo "Removed $BIN and $RULE"
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
        mkdir -p "$(dirname "$RULE")"
        "$BIN" udev-rule > "$RULE"
        chmod 644 "$RULE"
        remove_legacy_unit
        reload_udev
        echo "Installed $BIN and $RULE"
        "$BIN" status
        if [ ! -f "$CONFIG" ]; then
            echo
            echo "Choose a level with: touchpad-haptic set N   (0 = off, 100 = strongest)"
        fi
        ;;
esac
