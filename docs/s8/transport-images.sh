#!/bin/bash
# Expected pictures for the screen transport check (#44):
# docs/s8/images/transport/*.png.
#   transport-images.sh          regenerate every picture
#   transport-images.sh check    compare the committed pictures with what the
#                                tool produces now (decoded pixels); changes nothing
# Dry runs only: nothing is opened or sent. Each picture is decoded from the
# frame bytes the bridge would send for the feeder's records (s8-screen-feed
# --png). Steps match docs/s8/screen-transport-check.sh.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

mode="${1:-generate}"
case "$mode" in generate | check) ;; *) echo "Usage: $0 [check]" && exit 1 ;; esac
dir=docs/s8/images/transport
doc=docs/s8/screen-transport.md
cargo build --locked --release --example s8-screen-feed > /dev/null
tool=target/release/examples/s8-screen-feed

steps=(
    "a-left-10|--panels left --rate 10 --frames 50 --png-frames 0,25,49"
    "b-right-30|--panels right --rate 30 --frames 150 --png-frames 0,149"
    "c-alternate-10|--panels alternate --rate 10 --frames 50 --png-frames 0,25,49"
    "d-alternate-30|--panels alternate --rate 30 --frames 150 --png-frames 0,149"
    "e-over-cap|--panels left --rate 60 --frames 180 --png-frames 179"
    "f-stall|--panels left --rate 10 --frames 30 --png-frames 29"
    "g-eof|--panels alternate --rate 10 --frames 30 --png-frames 29"
    "h-bad-record|--panels left --rate 10 --frames 20 --png-frames 19"
    "i-ctrl-c|--panels left --rate 10 --frames 600 --png-frames 0"
    "exit-black|--panels left --rate 10 --frames 1 --png-exit"
)

if [ "$mode" = generate ]; then
    mkdir -p "$dir"
    rm -f "$dir"/*.png
fi

checked=0
for step in "${steps[@]}"; do
    name="${step%%|*}"
    # shellcheck disable=SC2086,SC2046
    out=$("$tool" ${step#*|} --dry-run --png "$dir" --name "$name" $([ "$mode" = check ] && echo --png-check) 2> /dev/null)
    line=$(echo "$out" | grep -E '^(wrote|checked) [0-9]+ pictures')
    echo "$name: $line"
    checked=$((checked + $(echo "$line" | awk '{print $2}')))
done

files=$(find "$dir" -name '*.png' | wc -l | tr -d ' ')
echo "pictures made or checked: $checked, files: $files"
[ "$checked" -eq "$files" ] || {
    echo "FAIL: $files files but $checked pictures from the tool"
    exit 1
}
for file in "$dir"/*.png; do
    grep -qF "(images/transport/$(basename "$file"))" "$doc" || {
        echo "FAIL: $doc does not embed $file"
        exit 1
    }
done
grep -oE '\(images/transport/[^)]+\)' "$doc" | tr -d '()' | while read -r path; do
    [ -f "docs/s8/$path" ] || {
        echo "FAIL: $doc embeds $path, which does not exist"
        exit 1
    }
done
echo "PASS"
