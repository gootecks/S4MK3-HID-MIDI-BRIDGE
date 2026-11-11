#!/bin/bash
# Expected pictures for screen phase 5 (#43): docs/s8/images/phase5/*.png.
#   phase5-images.sh          regenerate every picture
#   phase5-images.sh check    compare the committed pictures with what the tool
#                             produces now (decoded pixels); changes nothing
# Dry runs only: nothing is opened or sent. Each picture is decoded from the
# frame bytes the tool would send. Steps match docs/s8/screen-stream-check.sh.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

mode="${1:-generate}"
case "$mode" in generate | check) ;; *) echo "Usage: $0 [check]" && exit 1 ;; esac
dir=docs/s8/images/phase5
doc=docs/s8/screen-stream-expected.md
cargo build --locked --release --example s8-screen-frame > /dev/null
tool=target/release/examples/s8-screen-frame

steps=(
    "a-left|stream --rates 10,20,30,60 --seconds 5 --panels left --screen left"
    "a-right|stream --rates 10,20,30,60 --seconds 5 --panels right --screen right"
    "b-alternate|stream --rates 10,20,30,60 --seconds 5 --panels alternate"
    "c-burst|burst --frames 120 --screen left"
    "d-soak|stream --rates 30 --seconds 60 --panels alternate"
    "e-bg|f800 --screen left"
    "e-skip|partial skip --screen left --under f800"
    "e-rect|partial rect --screen left --under f800"
    "f-end|stream --rates 30 --seconds 5 --panels alternate --png-frames last"
    "f-kill|stream --rates 30 --seconds 30 --panels alternate --png-frames 120"
    "f-clean-left|pattern --screen left"
    "f-clean-right|pattern --screen right"
)

if [ "$mode" = generate ]; then
    mkdir -p "$dir"
    rm -f "$dir"/*.png
fi

checked=0
for step in "${steps[@]}"; do
    name="${step%%|*}"
    # shellcheck disable=SC2086,SC2046
    out=$("$tool" ${step#*|} --png "$dir" --name "$name" $([ "$mode" = check ] && echo --png-check))
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
    grep -qF "images/phase5/$(basename "$file")" "$doc" || {
        echo "FAIL: $doc does not embed $file"
        exit 1
    }
done
echo "PASS"
