#!/bin/bash
# Expected pictures of the S8 panel views (#27):
# docs/s8/images/panel-views/*.png.
#   panel-views-images.sh          regenerate every picture
#   panel-views-images.sh check    compare the committed pictures with what the
#                                  tools produce now (decoded pixels); changes nothing
# Nothing is opened or sent. S8Host --replay turns each snapshot under
# docs/s8/fixtures/panel-views/ into the records it would write to the bridge;
# s8-screen-png decodes the frame bytes the bridge would send for them.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

mode="${1:-generate}"
case "$mode" in generate | check) ;; *) echo "Usage: $0 [check]" && exit 1 ;; esac
fixtures=docs/s8/fixtures/panel-views
dir=docs/s8/images/panel-views
doc=docs/s8/panel-views.md
cargo build --locked --release --example s8-screen-png > /dev/null
swift build --package-path macos --product S8Host > /dev/null
host="$(swift build --package-path macos --show-bin-path)/S8Host"
tool=target/release/examples/s8-screen-png

records="$(mktemp -d)"
trap 'rm -rf "$records"' EXIT
"$host" --replay "$fixtures" --records "$records"

if [ "$mode" = generate ]; then
    mkdir -p "$dir"
    rm -f "$dir"/*.png
    "$tool" --out "$dir" "$records"/*.s8fr
else
    "$tool" --check --out "$dir" "$records"/*.s8fr
fi

snapshots=$(find "$fixtures" -maxdepth 1 -name '*.json' | wc -l | tr -d ' ')
files=$(find "$dir" -name '*.png' | wc -l | tr -d ' ')
echo "snapshots: $snapshots, pictures: $files"
[ "$files" -eq $((snapshots * 2)) ] || { echo "FAIL: $files pictures for $snapshots snapshots"; exit 1; }
for file in "$dir"/*.png; do
    grep -qF "(images/panel-views/$(basename "$file"))" "$doc" || { echo "FAIL: $doc does not embed $file"; exit 1; }
done
grep -oE '\(images/panel-views/[^)]+\)' "$doc" | tr -d '()' | while read -r path; do
    [ -f "docs/s8/$path" ] || { echo "FAIL: $doc embeds $path, which does not exist"; exit 1; }
done
echo "PASS"
