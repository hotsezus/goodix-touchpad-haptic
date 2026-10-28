#!/bin/sh
# Build a self-extracting touchpad-haptic-<version>.run installer into dist/.
# Arguments given to the .run file are passed to install.sh (e.g. --uninstall).
set -eu

cd "$(dirname "$0")"
VERSION=$(git describe --tags --always --dirty 2>/dev/null || echo dev)
OUT=dist/touchpad-haptic-$VERSION.run
TARGET=x86_64-unknown-linux-musl
mkdir -p dist

cargo build --release --locked --target "$TARGET"
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
cp "target/$TARGET/release/touchpad-haptic" install.sh "$stage/"

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
    tar -cz --owner=0 --group=0 --numeric-owner -C "$stage" touchpad-haptic install.sh
} > "$OUT"
chmod +x "$OUT"
echo "$OUT"
