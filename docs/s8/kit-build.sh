#!/bin/bash
# Builds the S8 kit (#131): what a second Apple Silicon Mac needs to run the
# S8 with djay Pro without the repository or a toolchain. Sends nothing.
# Output: dist/Kontrol S8 Kit <commit>/ and the same as a zip. It refuses to
# overwrite an existing kit. Procedure: docs/s8/kit.md.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

fail() {
    echo "kit-build: $1" >&2
    exit 1
}

[ "$(uname -sm)" = "Darwin arm64" ] || fail "the kit is built on an Apple Silicon Mac"
commit="$(git rev-parse --short HEAD)"
[ -z "$(git status --porcelain --untracked-files=no)" ] || commit="$commit-dirty"
kit="dist/Kontrol S8 Kit $commit"
[ ! -e "$kit" ] && [ ! -e "$kit.zip" ] || fail "$kit exists; move it away first"

cargo build --locked --release --bin s4mk3-hid-midi-bridge
swift build --package-path macos -c release --product S8Host
bridge="target/release/s4mk3-hid-midi-bridge"
host="$(swift build --package-path macos -c release --show-bin-path)/S8Host"

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
cp "$bridge" "$host" "$stage/"
cp docs/s8/kit-start.sh "$stage/Start S8.command"
chmod +x "$stage/Start S8.command"

for binary in "$stage/S8Host" "$stage/s4mk3-hid-midi-bridge"; do
    otool -L "$binary" | awk 'NR > 1 && $1 !~ /^\/System\/Library\// && $1 !~ /^\/usr\/lib\// { print; found = 1 }
        END { exit found }' || fail "$binary links a library that another Mac does not have"
    codesign --force --sign - "$binary"
done

# The kit carries the mapping this bridge generates, under the name of the
# revision that was checked on the S8.
frozen="$(find docs/s8/mappings -name 'Kontrol S8 Bridge r*.djayMidiMapping' | sort -V | tail -1)"
revision="$(basename "$frozen" | sed -E 's/ [0-9]{4}-[0-9]{2}-[0-9]{2}\.djayMidiMapping$//')"
"$stage/s4mk3-hid-midi-bridge" --device s8 --generate-mapping "$stage"
cmp -s "$stage/Kontrol S8 Bridge.djayMidiMapping" "$frozen" ||
    fail "the generated mapping differs from $frozen; check it on the S8 and freeze a new revision first"
mv "$stage/Kontrol S8 Bridge.djayMidiMapping" "$stage/$revision.djayMidiMapping"

sed -e "s/@COMMIT@/$commit/" -e "s/@KIT@/$(basename "$kit")/" -e "s/@MAPPING@/$revision/" \
    docs/s8/kit-readme.txt > "$stage/README.txt"

mkdir -p dist
chmod 755 "$stage"
mv "$stage" "$kit"
trap - EXIT
ditto -c -k --norsrc --noextattr --keepParent "$kit" "$kit.zip"
echo "Created $kit"
echo "Created $kit.zip"
