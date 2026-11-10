#!/bin/bash
# Attended screen phase 5 (#43): repeated frames, held claim, partial updates.
# Plan: docs/s8/screens.md "Phase 5". Stops NIHardwareAgent with SIGKILL, runs
# each step of examples/s8-screen-frame after an Enter, restores the agent.
# Steps, in order; a failed step (tool error, or observer answers 0 / n)
# skips the rest and goes to recovery. Answering "stop" at any prompt does too.
#   a  held ladder 10,20,30,60 fps, 5 s each: left panel, then right panel
#   b  alternating panels at the same rates
#   c  burst of 120 frames to the left panel
#   d  soak 60 s alternating, at min(30, highest rate that passed a and b)
#   e  partial updates: SKIP variant, then sub-rectangle variant, over F800
#   f  exit state: no final frame; SIGKILL mid-stream; clean frames afterwards
# Raw logs go to docs/s8/captures/screen-stream-UTC/.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
# shellcheck source=docs/s8/screen-lib.sh
. docs/s8/screen-lib.sh

[ "$#" -eq 0 ] || { echo "Usage: $0"; exit 1; }
screen_init screen-stream
screen_preflight
screen_build release
screen_start

note "Phase 5. Djay open, no track playing. Tool: $tool (release build)."
screen_agent_stop

stopped=""
status=0

# confirm NAME: prompt, return 1 and record on "stop".
confirm() {
    ask "Next: $1. Enter to send, or 'stop'."
    [ "$answer" != stop ] || { stopped="before $1"; return 1; }
    mark "$1"
}

# pictures PICS: list the expected pictures to compare with, before the prompt.
pictures() {
    note "Expected pictures (docs/s8/screen-stream-expected.md):"
    ls docs/s8/images/phase5/"$1"-*.png
}

# run NAME PICS ARGS...: dump check (hashes only), then the real send. Sets $status.
run_tool() {
    local name="$1" pics="$2"
    shift 2
    "$tool" "$@" --dump "$run/$name.dump" > /dev/null
    note "$name: dry-run written to $run/$name.dump ($(wc -l < "$run/$name.dump" | tr -d ' ') lines)"
    pictures "$pics"
    confirm "$name" || return 1
    set +e
    "$tool" "$@" 2>&1 | tee "$run/$name.txt"
    status=${PIPESTATUS[0]}
    set -e
    note "$name exit status $status"
    sleep 1
    snap "$name+1s"
    if [ "$status" -ne 0 ]; then stopped="$name failed"; return 1; fi
}

# observe NAME: highest clean rate, 0 stops the run. Sets $rate.
observe() {
    ask "$1: highest rate (10, 20, 30 or 60) that showed a steady moving bar and counter without tearing, flicker or stale areas; 0 if none; or 'stop'."
    [ "$answer" != stop ] || { stopped="after $1"; return 1; }
    [[ "$answer" =~ ^(0|10|20|30|60)$ ]] || { stopped="$1: unreadable answer '$answer'"; return 1; }
    rate=$answer
    [ "$rate" -gt 0 ] || { stopped="$1: no clean rate"; return 1; }
}

play_check() {
    ask "Press left PLAY once, then Enter. Did LEDs or audio change? Describe, or 'no'."
}

steps() {
    local rate_left rate_right rate_alt soak
    run_tool a-left a-left stream --rates 10,20,30,60 --seconds 5 --panels left --screen left || return 0
    observe a-left || return 0; rate_left=$rate
    run_tool a-right a-right stream --rates 10,20,30,60 --seconds 5 --panels right --screen right || return 0
    observe a-right || return 0; rate_right=$rate
    play_check

    run_tool b-alternate b-alternate stream --rates 10,20,30,60 --seconds 5 --panels alternate || return 0
    observe b-alternate || return 0; rate_alt=$rate

    run_tool c-burst c-burst burst --frames 120 --screen left || return 0
    ask "c-burst: any tearing or stale areas on the left panel afterwards? Describe, or 'stop'."
    [ "$answer" != stop ] || { stopped="after c-burst"; return 0; }

    soak=$(( rate_left < rate_right ? rate_left : rate_right ))
    soak=$(( soak < rate_alt ? soak : rate_alt ))
    soak=$(( soak < 30 ? soak : 30 ))
    note "Soak rate: $soak frames/s per panel (min of a-left $rate_left, a-right $rate_right, b $rate_alt, capped at 30)."
    run_tool d-soak d-soak stream --rates "$soak" --seconds 60 --panels alternate || return 0
    ask "d-soak: steady for the whole 60 s? y/n, or describe"
    case "$answer" in y|Y|yes) ;; *) stopped="d-soak not steady"; return 0 ;; esac
    play_check

    run_tool e-bg-skip e-bg f800 --screen left || return 0
    run_tool e-skip e-skip partial skip --screen left || return 0
    ask "e-skip: top half, bottom half? Photo. Describe, or 'stop'."
    [ "$answer" != stop ] || { stopped="after e-skip"; return 0; }
    run_tool e-bg-rect e-bg f800 --screen left || return 0
    run_tool e-rect e-rect partial rect --screen left || return 0
    ask "e-rect: where did the pattern land? Photo. Describe, or 'stop'."
    [ "$answer" != stop ] || { stopped="after e-rect"; return 0; }

    run_tool f-end f-end stream --rates 30 --seconds 5 --panels alternate || return 0
    sleep 3
    ask "f-end: stream ended without a final frame. What do the panels show? Photo."
    pictures f-kill
    confirm f-kill || return 0
    "$tool" stream --rates 30 --seconds 30 --panels alternate > "$run/f-kill.txt" 2>&1 &
    local streamer=$!
    sleep 4
    mark f-killed
    kill -9 "$streamer" 2> /dev/null || true
    wait "$streamer" 2> /dev/null || true
    note "f-kill: tool killed with SIGKILL after about 4 s of streaming."
    sleep 1
    snap "f-kill+1s"
    ask "f-kill: what do the panels show after the kill? Photo."
    run_tool f-clean-left f-clean-left pattern --screen left || return 0
    run_tool f-clean-right f-clean-right pattern --screen right || return 0
    ask "f-clean: do both panels show the clean test pattern? y/n, or describe"
}

steps
[ -z "$stopped" ] || note "Stopped: $stopped."

mark released
sleep 3
ask "Interface released, agent still stopped: what do the panels show? Photo."

screen_finish
