#!/bin/bash
# Attended screen phase 1 (#33): claim S8 interface 6 for 5 s, no transfers.
# Stops NIHardwareAgent with SIGKILL, runs examples/s8-screen-claim.rs and
# restores the agent. Raw logs go to target/s8-captures/ and stay out of git.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

bundle="/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"
exe="$bundle/Contents/MacOS/NIHardwareAgent"
run="target/s8-captures/screen-claim-$(date -u +%Y%m%dT%H%M%SZ)"
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

cargo build --locked --quiet --example s8-passive-recorder --example s8-screen-claim
target/debug/examples/s8-passive-recorder --out "$run/recorder.jsonl" --seconds 900 &
recorder=$!
claim=""
restore() {
    [ -z "$claim" ] || kill -INT "$claim" 2> /dev/null || true
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
sleep 1
snap "stopped+1s"
ask "Agent stopped: press left PLAY, CUE A and pad 1 once each, then Enter."

note "Next: the tool claims interface 6 for 5 s and sends nothing."
ask "During the hold, press left PLAY, CUE A and pad 1 once each. Enter to start."
mark claim
target/debug/examples/s8-screen-claim > "$run/claim.txt" 2>&1 &
claim=$!
sleep 2
snap "claim+2s"
status=0
wait "$claim" || status=$?
claim=""
mark released
note "Claim tool exit status $status:"
tee -a "$run/notes.txt" < "$run/claim.txt"
sleep 1
snap "released+1s"
ask "During or after the hold, did panels, LEDs or audio change? Describe, or 'no'."
ask "Released: press left PLAY, CUE A and pad 1 once each, then Enter."

mark recover
[ -n "$(agent_pid)" ] || open -g "$bundle"
sleep 5
snap "recovered+5s"
ask "After recovery: do the panels show Controller Editor labels again? y/n"
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
