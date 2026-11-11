#!/bin/bash
# Shared parts of the attended S8 screen runners. Source it; it does nothing
# on its own.
#   screen_init NAME      create docs/s8/captures/NAME-UTC and set $run
#   screen_preflight      refuse unless one S8, no rival apps, one genuine agent
#   screen_build PROFILE  build the recorder and screen tool; sets $tool, $recorder_bin
#   screen_start          start the passive recorder and the restore trap
#   screen_agent_stop     baseline PLAY check, SIGKILL the agent, PLAY check
#   screen_finish         release wait, recovery, PLAY check, jq per-window summary
#   row QUESTION          ask and record; false when the answer is "stop"
#   note TEXT [STYLE]     print TEXT, in STYLE on a terminal, and record it plain
#   start_host NAME ARGS  run S8Host --screens ARGS with $host and $bridge, log to $run/NAME.log
#   stop_host NAME HOW    stop it with Ctrl-C (int) or by closing its stdin (eof)
# Styles ($bold $dim $red $green $yellow $cyan, closed by $reset) are empty when
# stdout is not a terminal or NO_COLOR is set. The log files never get them.

bundle="/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"
exe="$bundle/Contents/MacOS/NIHardwareAgent"
answer=""
# shellcheck disable=SC2034 # the runners that source this file use the styles
{
    bold="" dim="" red="" green="" yellow="" cyan="" reset=""
    if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
        bold=$'\033[1m' dim=$'\033[2m' reset=$'\033[0m'
        red=$'\033[1;31m' green=$'\033[1;32m' yellow=$'\033[1;33m' cyan=$'\033[1;36m'
    fi
}

now_ms() { echo $(($(date +%s) * 1000)); }
mark() { echo "$1 $(now_ms)" >> "$run/events.txt"; }
note() {
    printf '%s\n' "$1" >> "$run/notes.txt"
    printf '%s%s%s\n' "${2:-}" "$1" "$reset"
}
ask() {
    printf '\n%s%s%s\n' "$bold" "$1" "$reset"
    read -r -p "$green> $reset" answer
    printf '%s: %s\n' "$1" "$answer" >> "$run/notes.txt"
}
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
    [ -t 0 ] || {
        echo "Run this from an interactive terminal."
        exit 1
    }
    [ "$(ioreg -r -c IOUSBHostDevice -n 'Traktor Kontrol S8' | grep -c '^+-o Traktor Kontrol S8@')" -eq 1 ] ||
        {
            echo "Connect exactly one Kontrol S8."
            exit 1
        }
    if pgrep -i -f 'Traktor.app|Maschine|Komplete Kontrol' > /dev/null; then
        echo "Quit Traktor, Maschine and Komplete Kontrol first."
        exit 1
    fi
    pid=$(agent_pid)
    [ "$(echo "$pid" | wc -w | tr -d ' ')" -eq 1 ] || {
        echo "Expected one NIHardwareAgent, found: ${pid:-none}"
        exit 1
    }
    [ "$(ps -o command= -p "$pid")" = "$exe" ] || {
        echo "NIHardwareAgent identity mismatch."
        exit 1
    }
}

screen_build() {
    local flag=()
    [ "$1" != release ] || flag=(--release)
    cargo build --locked --quiet "${flag[@]+"${flag[@]}"}" --example s8-passive-recorder --example s8-screen-frame
    # shellcheck disable=SC2034 # read by the runner that sources this file
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

row() {
    ask "$1"
    [ "$answer" != stop ]
}

start_host() {
    local name="$1"
    shift
    log="$run/$name.log"
    fifo="$(mktemp -u "${TMPDIR:-/tmp}/s8host-stdin.XXXXXX")"
    mkfifo "$fifo"
    # shellcheck disable=SC2154 # $host and $bridge come from the sourcing runner
    "$host" --bridge "$bridge" --screens "$@" < "$fifo" \
        > >(perl -MTime::HiRes=time -ne 'BEGIN { $| = 1 } printf "%.3f %s", time, $_' > "$log") 2>&1 &
    host_pid=$!
    exec 9> "$fifo"
    mark "$name-started"
    for _ in $(seq 1 20); do
        grep -Eq 'S8_SCREEN_STATUS (active|failed)' "$log" 2> /dev/null && break
        kill -0 "$host_pid" 2> /dev/null || break
        sleep 0.5
    done
    if ! grep -q 'S8_SCREEN_STATUS active' "$log" 2> /dev/null; then
        note "$name: the screens did not become active within 10 s."
        cat "$log"
        exit 1
    fi
}

# stop_host NAME int|eof: Ctrl-C to S8Host, or close its stdin.
stop_host() {
    local name="$1" how="$2" status=0
    mark "$name-$how"
    if [ "$how" = int ]; then
        kill -INT "$host_pid" 2> /dev/null || true
    else
        exec 9>&-
    fi
    for _ in $(seq 1 30); do
        kill -0 "$host_pid" 2> /dev/null || break
        sleep 0.5
    done
    if kill -0 "$host_pid" 2> /dev/null; then
        note "$name: S8Host still running 15 s after $how; sending SIGTERM."
        kill -TERM "$host_pid"
    fi
    wait "$host_pid" || status=$?
    host_pid=""
    exec 9>&- || true
    rm -f "$fifo"
    fifo=""
    note "$name: S8Host exit status $status (expected 0)."
    note "$name: status lines: $(grep -o 'S8_SCREEN_STATUS.*' "$log" | tr '\n' ';')"
    note "$name: bridge processes left: $(pgrep -f "$bridge" | tr '\n' ' ')"
    snap "$name-stopped"
}
