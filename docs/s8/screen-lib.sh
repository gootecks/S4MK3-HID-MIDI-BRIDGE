#!/bin/bash
# Shared parts of the attended S8 screen runners (screen-frame-check.sh,
# screen-stream-check.sh). Source it; it does nothing on its own.
#   screen_init NAME      create docs/s8/captures/NAME-UTC and set $run
#   screen_preflight      refuse unless one S8, no rival apps, one genuine agent
#   screen_build PROFILE  build the recorder and screen tool; sets $tool, $recorder_bin
#   screen_start          start the passive recorder and the restore trap
#   screen_agent_stop     baseline PLAY check, SIGKILL the agent, PLAY check
#   screen_finish         release wait, recovery, PLAY check, jq per-window summary

bundle="/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"
exe="$bundle/Contents/MacOS/NIHardwareAgent"
answer=""

now_ms() { echo $(( $(date +%s) * 1000 )); }
mark() { echo "$1 $(now_ms)" >> "$run/events.txt"; }
note() { printf '%s\n' "$1" | tee -a "$run/notes.txt"; }
ask() { read -r -p "$1 " answer; printf '%s: %s\n' "$1" "$answer" >> "$run/notes.txt"; }
agent_pid() { pgrep -f "$exe" || true; }
snap() {
    {
        echo "### $1 $(date -u +%H:%M:%SZ)"
        echo "agent: $(pgrep -f "$exe" | tr '\n' ' ')"
        ioreg -w0 -r -c IOUSBHostDevice -n 'Traktor Kontrol S8' -l |
            grep -E '^[ |]*\+-o|"bInterfaceNumber"|"IOUserClientCreator"' || true
    } >> "$run/snapshots.txt"
}

screen_init() {
    run="docs/s8/captures/$1-$(date -u +%Y%m%dT%H%M%SZ)"
    mkdir -p "$run"
}

screen_preflight() {
    [ -t 0 ] || { echo "Run this from an interactive terminal."; exit 1; }
    [ "$(ioreg -r -c IOUSBHostDevice -n 'Traktor Kontrol S8' | grep -c '^+-o Traktor Kontrol S8@')" -eq 1 ] ||
        { echo "Connect exactly one Kontrol S8."; exit 1; }
    if pgrep -i -f 'Traktor.app|Maschine|Komplete Kontrol' > /dev/null; then
        echo "Quit Traktor, Maschine and Komplete Kontrol first."; exit 1
    fi
    pid=$(agent_pid)
    [ "$(echo "$pid" | wc -w | tr -d ' ')" -eq 1 ] || { echo "Expected one NIHardwareAgent, found: ${pid:-none}"; exit 1; }
    [ "$(ps -o command= -p "$pid")" = "$exe" ] || { echo "NIHardwareAgent identity mismatch."; exit 1; }
}

screen_build() {
    local flag=()
    [ "$1" != release ] || flag=(--release)
    cargo build --locked --quiet "${flag[@]+"${flag[@]}"}" --example s8-passive-recorder --example s8-screen-frame
    tool="target/$1/examples/s8-screen-frame"
    recorder_bin="target/$1/examples/s8-passive-recorder"
}

screen_restore() {
    if [ -z "$(agent_pid)" ]; then
        note "Restoring NIHardwareAgent."
        open -g "$bundle"
    fi
    kill -INT "$recorder" 2> /dev/null || true
}

screen_start() {
    "$recorder_bin" --out "$run/recorder.jsonl" --seconds 1800 &
    recorder=$!
    trap screen_restore EXIT
    sleep 2
    snap pre
}

screen_agent_stop() {
    mark baseline
    ask "Agent running: press left PLAY once, then Enter."
    mark stop
    kill -9 "$pid"
    sleep 1
    snap "stopped+1s"
    ask "Agent stopped: press left PLAY once, then Enter."
}

screen_finish() {
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

    local edges
    edges=$(awk '{ printf "%s{\"name\":\"%s\",\"ms\":%s}", (NR > 1 ? "," : "["), $1, $2 } END { print "]" }' "$run/events.txt")
    jq -r --argjson edges "$edges" '
        def window: (.unix_ms) as $ms
            | ([$edges[] | select(.ms <= $ms)] | last | .name) // "before";
        select(.src == "midi" or .src == "hid")
        | "\(window)\t\(.src)\t\(.port // ("report " + (.id | tostring)))\t\(if .src == "midi" then .hex else "" end)"
    ' "$run/recorder.jsonl" | sort | uniq -c | tee "$run/summary.txt"
    note "Recorded in $run"
}
