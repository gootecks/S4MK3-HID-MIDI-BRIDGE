#!/bin/bash
# Attended screen phase 0 (#32): does S8 MIDI survive a NIHardwareAgent stop?
# Sends nothing to the S8. Stops NIHardwareAgent with SIGKILL and restores it.
# Raw logs go to target/s8-captures/ and stay out of git.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

bundle="/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"
exe="$bundle/Contents/MacOS/NIHardwareAgent"
run="target/s8-captures/agent-stop-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$run"

now_ms() { echo $(( $(date +%s) * 1000 )); }
mark() { echo "$1 $(now_ms)" >> "$run/events.txt"; }
note() { printf '%s\n' "$1" | tee -a "$run/notes.txt"; }
ask() { local answer; read -r -p "$1 " answer; printf '%s: %s\n' "$1" "$answer" >> "$run/notes.txt"; }
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

cargo build --locked --quiet --example s8-passive-recorder
target/debug/examples/s8-passive-recorder --out "$run/recorder.jsonl" --seconds 600 &
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

note "Djay open, no track playing. Controller Editor may stay open."
mark baseline
ask "Agent running: press left PLAY, CUE A and pad 1 once each, then Enter."

mark stop
kill -9 "$pid"
stopped=$(date +%s)
sleep 1
snap "stopped+1s"
ask "Agent stopped: what do the two panels show?"
ask "Press left PLAY, CUE A and pad 1 once each, then Enter."
ask "Did Djay react to left PLAY (deck A play button lit or toggled)? y/n"
remaining=$(( 30 - ($(date +%s) - stopped) ))
if [ "$remaining" -gt 0 ]; then
    note "Waiting ${remaining}s for any automatic relaunch."
    sleep "$remaining"
fi
snap "stopped+30s"
if [ -n "$(agent_pid)" ]; then note "NIHardwareAgent relaunched by something else."; fi

mark recover
[ -n "$(agent_pid)" ] || open -g "$bundle"
sleep 5
snap "recovered+5s"
ask "After recovery: do the panels show Controller Editor labels again? y/n"
ask "Press left PLAY once, then Enter."
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
