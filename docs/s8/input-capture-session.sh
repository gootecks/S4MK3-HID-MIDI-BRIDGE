#!/bin/bash
# Attended S8 input capture (#10): every control with NIHardwareAgent stopped.
# Sends nothing to the S8; interface 5 is read in shared mode only.
# Stops NIHardwareAgent with SIGKILL and restores it. Walks the rows of
# capture-checklist.md and stamps each step label into the recorder log.
# Raw logs go to target/s8-captures/ and stay out of git.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

from="${1:-1}"
bundle="/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"
exe="$bundle/Contents/MacOS/NIHardwareAgent"
run="target/s8-captures/input-capture-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$run"

agent_pid() { pgrep -f "$exe" || true; }
stamp() { echo "$1" >&3; }

[ -t 0 ] || { echo "Run this from an interactive terminal."; exit 1; }
[ "$(ioreg -r -c IOUSBHostDevice -n 'Traktor Kontrol S8' | grep -c '^+-o Traktor Kontrol S8@')" -eq 1 ] ||
    { echo "Connect exactly one Kontrol S8."; exit 1; }
if pgrep -i -f 'Traktor.app|Maschine|Komplete Kontrol' > /dev/null; then
    echo "Quit Traktor, Maschine and Komplete Kontrol first."; exit 1
fi
pid=$(agent_pid)
[ "$(echo "$pid" | wc -w | tr -d ' ')" -eq 1 ] || { echo "Expected one NIHardwareAgent, found: ${pid:-none}"; exit 1; }
[ "$(ps -o command= -p "$pid")" = "$exe" ] || { echo "NIHardwareAgent identity mismatch."; exit 1; }

awk -F'|' '$2 ~ /^ *[0-9]+ *$/ {
    for (i = 2; i <= 6; i++) gsub(/^ +| +$|`/, "", $i)
    print $2 "\t" $3 "\t" $4 "\t" $5 "\t" $6
}' docs/s8/capture-checklist.md > "$run/rows.tsv"
total=$(wc -l < "$run/rows.tsv" | tr -d ' ')
{ [ "$from" -ge 1 ] && [ "$from" -le "$total" ]; } 2> /dev/null ||
    { echo "Start row must be between 1 and $total."; exit 1; }

cargo build --locked --quiet --release --example s8-input-diff
mkfifo "$run/labels"
target/release/examples/s8-input-diff --log "$run/capture.jsonl" < "$run/labels" &
recorder=$!
exec 3> "$run/labels"
finish() {
    exec 3>&-
    kill -INT "$recorder" 2> /dev/null || true
    wait "$recorder" 2> /dev/null || true
    if [ -z "$(agent_pid)" ]; then
        echo "Restoring NIHardwareAgent."
        open -g "$bundle"
    fi
}
trap finish EXIT
sleep 1
kill -0 "$recorder" 2> /dev/null || { echo "The recorder did not start."; exit 1; }

echo "Stopping NIHardwareAgent. Djay gets no S8 MIDI until it is restored."
kill -9 "$pid"
sleep 1
stamp warmup
read -r -p "Warm-up: press left SHIFT once and nudge the crossfader, then Enter. "

next="$from"
while IFS=$'\t' read -r number label control motion expected <&4; do
    [ "$number" -ge "$from" ] || continue
    next="$number"
    printf '\n[%s/%s] %s: %s\n  Motion:   %s\n  Expected: %s\n' \
        "$number" "$total" "$label" "$control" "$motion" "$expected"
    stamp "$label"
    while :; do
        read -r -p "  Enter = next, r = redo, q = stop, other text = note: " answer
        case "$answer" in
            "") break ;;
            q) break 2 ;;
            r) stamp "$label" ;;
            *) printf '%s\t%s\n' "$label" "$answer" >> "$run/notes.tsv" ;;
        esac
    done
    next=$((number + 1))
done 4< "$run/rows.tsv"

stamp end
sleep 1
trap - EXIT
finish
sleep 5
if [ -n "$(agent_pid)" ]; then
    echo "NIHardwareAgent is running again."
else
    echo "NIHardwareAgent is not running. Open: $bundle"
fi

jq -rs '
    (map(select(.src == "change").step) | unique) as $changed
    | [.[] | select(.src == "step").label] | unique
    | map(select(. != "warmup" and . != "end") | select(. as $step | $changed | index($step) | not))
    | if length == 0 then "Every stamped step recorded a change."
      else "Steps with no recorded change: " + join(", ") end
' "$run/capture.jsonl"
if [ "$next" -le "$total" ]; then
    echo "Stopped before row $next. Continue with: mise run s8:input-capture $next"
fi
echo "Recorded in $run"
