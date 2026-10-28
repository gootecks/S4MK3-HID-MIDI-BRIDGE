#!/bin/bash
# Attended screen phases 2 and 4 (#34, #35): send single S8 frames.
#   first          phase 2: black and F800 on the left panel, then the right
#   pattern ORDER  phase 4: full LITERAL test pattern, ORDER rgb or bgr
# Stops NIHardwareAgent with SIGKILL, sends each frame with
# examples/s8-screen-frame.rs after an Enter, and restores the agent.
# Answering "stop" at any prompt skips to recovery. Raw logs go to
# target/s8-captures/ and stay out of git.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

phase="${1:-}"
case "$phase" in
    first) steps=("black left" "f800 left" "black right" "f800 right"); order=rgb ;;
    pattern)
        order="${2:-}"
        [[ "$order" =~ ^(rgb|bgr)$ ]] || { echo "Usage: $0 pattern rgb|bgr"; exit 1; }
        steps=("pattern left" "pattern right") ;;
    *) echo "Usage: $0 first | pattern rgb|bgr"; exit 1 ;;
esac

bundle="/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"
exe="$bundle/Contents/MacOS/NIHardwareAgent"
run="target/s8-captures/screen-$phase-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$run"

now_ms() { echo $(( $(date +%s) * 1000 )); }
mark() { echo "$1 $(now_ms)" >> "$run/events.txt"; }
note() { printf '%s\n' "$1" | tee -a "$run/notes.txt"; }
answer=""
ask() { read -r -p "$1 " answer; printf '%s: %s\n' "$1" "$answer" >> "$run/notes.txt"; }
snap() {
    {
        echo "### $1 $(date -u +%H:%M:%SZ)"
        echo "agent: $(pgrep -f "$exe" | tr '\n' ' ')"
        ioreg -w0 -r -c IOUSBHostDevice -n 'Traktor Kontrol S8' -l |
            grep -E '^[ |]*\+-o|"bInterfaceNumber"|"IOUserClientCreator"' || true
    } >> "$run/snapshots.txt"
}
agent_pid() { pgrep -f "$exe" || true; }

[ -t 0 ] || { echo "Run this from an interactive terminal."; exit 1; }
[ "$(ioreg -r -c IOUSBHostDevice -n 'Traktor Kontrol S8' | grep -c '^+-o Traktor Kontrol S8@')" -eq 1 ] ||
    { echo "Connect exactly one Kontrol S8."; exit 1; }
if pgrep -i -f 'Traktor.app|Maschine|Komplete Kontrol' > /dev/null; then
    echo "Quit Traktor, Maschine and Komplete Kontrol first."; exit 1
fi
pid=$(agent_pid)
[ "$(echo "$pid" | wc -w | tr -d ' ')" -eq 1 ] || { echo "Expected one NIHardwareAgent, found: ${pid:-none}"; exit 1; }
[ "$(ps -o command= -p "$pid")" = "$exe" ] || { echo "NIHardwareAgent identity mismatch."; exit 1; }

cargo build --locked --quiet --example s8-passive-recorder --example s8-screen-frame
tool=target/debug/examples/s8-screen-frame
target/debug/examples/s8-passive-recorder --out "$run/recorder.jsonl" --seconds 1800 &
recorder=$!
restore() {
    if [ -z "$(agent_pid)" ]; then
        note "Restoring NIHardwareAgent."
        open -g "$bundle"
    fi
    kill -INT "$recorder" 2> /dev/null || true
}
trap restore EXIT
sleep 2
snap pre

note "Phase $phase, order $order. Djay open, no track playing."
mark baseline
ask "Agent running: press left PLAY once, then Enter."

mark stop
kill -9 "$pid"
sleep 1
snap "stopped+1s"
ask "Agent stopped: press left PLAY once, then Enter."

stopped=""
for step in "${steps[@]}"; do
    read -r frame side <<< "$step"
    name="$frame-$side"
    preview=()
    [ "$frame" != pattern ] || preview=(--preview "$run/$name.ppm")
    "$tool" "$frame" --screen "$side" --order "$order" --dump "$run/$name.bin" ${preview[@]+"${preview[@]}"} > /dev/null
    note "$name: $(wc -c < "$run/$name.bin" | tr -d ' ') bytes, sha256 $(shasum -a 256 "$run/$name.bin" | cut -d' ' -f1)"
    [ "$frame" = pattern ] || note "$name: $(xxd -p -c64 "$run/$name.bin")"
    ask "Next: send $name. Enter to send, or 'stop'."
    [ "$answer" != stop ] || { stopped="before $name"; break; }
    mark "$name"
    status=0
    "$tool" "$frame" --screen "$side" --order "$order" > "$run/$name.txt" 2>&1 || status=$?
    note "$name exit status $status:"
    tee -a "$run/notes.txt" < "$run/$name.txt"
    sleep 1
    snap "$name+1s"
    [ "$status" -eq 0 ] || { stopped="$name failed"; break; }
    ask "What do the panels show now? Take a photo. Describe, or 'stop'."
    [ "$answer" != stop ] || { stopped="after $name"; break; }
    ask "Press left PLAY once, then Enter. Did LEDs or audio change? Describe, or 'no'."
done
[ -z "$stopped" ] || note "Stopped: $stopped."

mark released
sleep 3
ask "Interface released, agent still stopped: what do the panels show? Photo."

mark recover
[ -n "$(agent_pid)" ] || open -g "$bundle"
sleep 5
snap "recovered+5s"
ask "After recovery: what do the panels show (Controller Editor labels again)? Photo."
ask "Press left PLAY once, then Enter."
ask "Did Djay react to left PLAY? y/n"
mark end
sleep 1
kill -INT "$recorder"
wait "$recorder" || true
trap - EXIT

edges=$(awk '{ printf "%s{\"name\":\"%s\",\"ms\":%s}", (NR > 1 ? "," : "["), $1, $2 } END { print "]" }' "$run/events.txt")
jq -r --argjson edges "$edges" '
    def window: (.unix_ms) as $ms
        | ([$edges[] | select(.ms <= $ms)] | last | .name) // "before";
    select(.src == "midi" or .src == "hid")
    | "\(window)\t\(.src)\t\(.port // ("report " + (.id | tostring)))\t\(if .src == "midi" then .hex else "" end)"
' "$run/recorder.jsonl" | sort | uniq -c | tee "$run/summary.txt"
note "Recorded in $run"
