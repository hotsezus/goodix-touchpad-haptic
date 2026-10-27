#!/bin/sh
# Build a self-extracting touchpad-haptic-<version>.run installer into dist/.
# Arguments given to the .run file are passed to install.sh (e.g. --uninstall).
set -eu

cd "$(dirname "$0")"
VERSION=$(git describe --tags --always --dirty 2>/dev/null || echo dev)
OUT=dist/touchpad-haptic-$VERSION.run
mkdir -p dist

{
    cat <<'EOF'
#!/bin/sh
# touchpad-haptic self-extracting installer
set -eu
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
line=$(awk '/^__PAYLOAD_BELOW__$/ { print NR + 1; exit }' "$0")
tail -n +"$line" "$0" | tar -xz -C "$tmp"
sh "$tmp/install.sh" "$@"
exit
__PAYLOAD_BELOW__
EOF
    tar -cz --owner=0 --group=0 --numeric-owner touchpad-haptic touchpad-haptic.service install.sh
} > "$OUT"
chmod +x "$OUT"
echo "$OUT"
