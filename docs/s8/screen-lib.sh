#!/bin/bash
# Shared parts of the attended S8 screen runners. Source it; it does nothing
# on its own.
#   screen_init NAME      create docs/s8/captures/NAME-UTC and set $run
#   screen_preflight      refuse unless one S8, no rival apps, one genuine agent
#   screen_build PROFILE [TOOL]  build the recorder and TOOL (default s8-screen-frame); sets $tool, $recorder_bin
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
screen_mode=run
screen_only=""
screen_stream=""
screen_started=""
screen_context=""
# shellcheck disable=SC2034 # the runners that source this file use the styles
{
    bold="" dim="" red="" green="" yellow="" cyan="" reset=""
    if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
        bold=$'\033[1m' dim=$'\033[2m' reset=$'\033[0m'
        red=$'\033[1;31m' green=$'\033[1;32m' yellow=$'\033[1;33m' cyan=$'\033[1;36m'
    fi
}

# Use the LED runner's space-delimited membership check for row identifiers.
has_row() {
    case " $1 " in
        *" $2 "*) return 0 ;;
        *) return 1 ;;
    esac
}

require_row() {
    case "$2" in
        '' | *[!A-Za-z0-9]*) ;;
        *) has_row "$1" "$2" && return 0 ;;
    esac
    printf 'Unknown row %q. Selectable rows: %s\n' "$2" "$1" >&2
    exit 1
}

# screen_selection LIVE_ROWS [plan] [only] [ROW...]
screen_selection() {
    local live="$1" row_id
    shift
    if [ "${1:-}" = plan ]; then
        screen_mode=plan
        shift
    fi
    if [ "${1:-}" = only ]; then
        shift
        [ "$#" -gt 0 ] || {
            echo "Specify at least one row after 'only'." >&2
            exit 1
        }
    fi
    for row_id in "$@"; do
        require_row "$live" "$row_id"
    done
    screen_only="$*"
}

screen_selected() { [ -z "$screen_only" ] || has_row "$screen_only" "$1"; }

# Return true when this part has live rows. In plan mode print those rows
# and the mandatory stop row, using the same selection as the live runner.
screen_part() {
    local name="$1" rows="$2" stop="$3" row_id selected=""
    for row_id in $rows; do
        if screen_selected "$row_id"; then
            selected="${selected:+$selected }$row_id"
        fi
    done
    [ -n "$selected" ] || return 1
    if [ "$screen_mode" = plan ]; then
        printf '%s: rows %s; stop row %s\n' "$name" "$selected" "$stop"
    fi
}

screen_build_context() {
    mkdir -p macos/.build
    screen_context="macos/.build/attended-djay-context"
    swiftc docs/s8/attended-djay-context.swift -o "$screen_context"
}

screen_clock() { perl -MTime::HiRes=clock_gettime,CLOCK_MONOTONIC -e 'printf "%.3f", clock_gettime(CLOCK_MONOTONIC)'; }

screen_stream_start() {
    screen_started="$(screen_clock)"
    screen_stream="$1"
}

screen_stream_stop() { screen_stream=""; }

screen_answer_context() {
    local utc elapsed=n/a context
    utc="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    if [ -n "$screen_stream" ]; then
        elapsed="$(awk -v now="$(screen_clock)" -v start="$screen_started" 'BEGIN { printf "%.3f", now - start }')"
    fi
    # Keep any partial result, even if the Accessibility window was unavailable.
    context="$("$screen_context" 2> /dev/null)" || true
    context="${context:-layout=\"unknown\" visible_space=unknown}"
    printf '  utc=%s stream=%s elapsed_s=%s %s\n' "${utc}" "${screen_stream:-none}" "$elapsed" "$context" >> "$run/notes.txt"
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
    if [ -n "$screen_context" ]; then
        screen_answer_context
    fi
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
    local flag=() example="${2:-s8-screen-frame}"
    [ "$1" != release ] || flag=(--release)
    cargo build --locked --quiet "${flag[@]+"${flag[@]}"}" --example s8-passive-recorder --example "$example"
    # shellcheck disable=SC2034 # read by the runner that sources this file
    tool="target/$1/examples/$example"
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

screen_row() {
    screen_selected "${1%%.*}" || return 0
    row "$1"
}

# Always record the stop answer for a started part; a prior stop request
# still skips all later parts.
screen_stop_row() {
    local prior="$answer"
    ask "$1"
    if [ "$prior" = stop ]; then
        answer=stop
    fi
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
    if [ -n "$screen_context" ]; then
        screen_stream_start "$name"
    fi
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
    screen_stream_stop
    host_pid=""
    exec 9>&- || true
    rm -f "$fifo"
    fifo=""
    note "$name: S8Host exit status $status (expected 0)."
    note "$name: status lines: $(grep -o 'S8_SCREEN_STATUS.*' "$log" | tr '\n' ';')"
    note "$name: bridge processes left: $(pgrep -f "$bridge" | tr '\n' ' ')"
    snap "$name-stopped"
}
