#!/bin/bash
# Builds Kontrol S8 Bridge.app (#16): the menu-bar app that runs the S8 with
# djay Pro, with the release bridge, S8Host and the frozen mappings inside.
# Sends nothing to the S8. Output: dist/Kontrol S8 Bridge <commit>/ and a zip
# of the app next to it; it refuses to overwrite either. Procedure:
# docs/s8/app.md.
#
# Signing: S8_APP_SIGN_IDENTITY names the codesign identity. Unset, it takes
# the first "Developer ID Application" identity, then "Apple Development",
# then signs ad hoc ("-"). S8_APP_NOTARY_PROFILE names a notarytool keychain
# profile; with a Developer ID identity the zip is notarized and the app
# stapled.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

fail() {
    echo "app-build: $1" >&2
    exit 1
}

[ "$(uname -sm)" = "Darwin arm64" ] || fail "the app is built on an Apple Silicon Mac"
commit="$(git rev-parse --short HEAD)"
[ -z "$(git status --porcelain --untracked-files=no)" ] || commit="$commit-dirty"
name="Kontrol S8 Bridge"
out="dist/$name $commit"
zip="$out.zip"
[ ! -e "$out" ] && [ ! -e "$zip" ] || fail "$out exists; move it away first"

identity="${S8_APP_SIGN_IDENTITY:-}"
if [ -z "$identity" ]; then
    identities="$(security find-identity -v -p codesigning)"
    for kind in "Developer ID Application" "Apple Development"; do
        identity="$(printf '%s\n' "$identities" | sed -nE "s/^ *[0-9]+\) [0-9A-F]+ \"($kind: [^\"]+)\"$/\1/p" | head -1)"
        [ -z "$identity" ] || break
    done
    identity="${identity:--}"
fi

cargo build --locked --release --bin s4mk3-hid-midi-bridge
swift build --package-path macos -c release --product S8Host
swift build --package-path macos -c release --product S8App
bin="$(swift build --package-path macos -c release --show-bin-path)"

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
app="$stage/$name.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/Mappings"
cp "$bin/S8App" "$bin/S8Host" target/release/s4mk3-hid-midi-bridge "$app/Contents/MacOS/"
cp macos/S8App/Info.plist "$app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :S8BridgeCommit $commit" \
    -c "Set :CFBundleVersion $(git rev-list --count HEAD)" "$app/Contents/Info.plist"

iconset="$stage/AppIcon.iconset"
mkdir "$iconset"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" macos/Assets/AppIcon.png --out "$iconset/icon_${size}x${size}.png" > /dev/null
    sips -z $((size * 2)) $((size * 2)) macos/Assets/AppIcon.png \
        --out "$iconset/icon_${size}x${size}@2x.png" > /dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"

for binary in "$app"/Contents/MacOS/*; do
    otool -L "$binary" | awk 'NR > 1 && $1 !~ /^\/System\/Library\// && $1 !~ /^\/usr\/lib\// { print; found = 1 }
        END { exit found }' || fail "$binary links a library that another Mac does not have"
done

# The app carries the mappings this bridge generates, under the names of the
# revisions that were checked on the S8, as the kit does.
mappings="$app/Contents/Resources/Mappings"
generate() {
    local layout="$1" pattern="$2" generated="$3" frozen revision
    frozen="$(find docs/s8/mappings -name "$pattern" | sort -V | tail -1)"
    [ -n "$frozen" ] || fail "no frozen mapping matches $pattern"
    revision="$(basename "$frozen" | sed -E 's/ [0-9]{4}-[0-9]{2}-[0-9]{2}\.djayMidiMapping$//')"
    "$app/Contents/MacOS/s4mk3-hid-midi-bridge" --device s8 --s8-layout "$layout" --generate-mapping "$stage"
    cmp -s "$stage/$generated" "$frozen" ||
        fail "the generated $layout mapping differs from $frozen; check it on the S8 and freeze a new revision first"
    mv "$stage/$generated" "$mappings/$revision.djayMidiMapping"
}
generate default 'Kontrol S8 Bridge r*.djayMidiMapping' "$name.djayMidiMapping"
generate evan 'Kontrol S8 Bridge Evan r*.djayMidiMapping' "$name Evan.djayMidiMapping"

options=()
case "$identity" in
    "Developer ID Application:"*) options=(--options runtime --timestamp) ;;
esac
for binary in S8Host s4mk3-hid-midi-bridge; do
    codesign --force --sign "$identity" "${options[@]}" "$app/Contents/MacOS/$binary"
done
codesign --force --sign "$identity" "${options[@]}" "$app"
codesign --verify --deep --strict "$app"

mkdir -p "$out"
chmod 755 "$stage"
mv "$app" "$out/"
app="$out/$name.app"
ditto -c -k --norsrc --noextattr --keepParent "$app" "$zip"

if [ -n "${S8_APP_NOTARY_PROFILE:-}" ]; then
    [ ${#options[@]} -gt 0 ] || fail "notarization needs a Developer ID Application identity, not $identity"
    xcrun notarytool submit "$zip" --keychain-profile "$S8_APP_NOTARY_PROFILE" --wait
    xcrun stapler staple "$app"
    rm "$zip"
    ditto -c -k --norsrc --noextattr --keepParent "$app" "$zip"
fi

echo "Signed with: $identity"
echo "Created $app"
echo "Created $zip"
