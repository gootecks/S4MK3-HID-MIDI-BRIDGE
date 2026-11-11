#!/bin/bash
# Attended deck page check on the S8 (#85): the live djay deck pages through
# `S8Preview --stream | bridge --device s8 --midi full --s8-screens-stdin`.
# Plan, pass and fault criteria: docs/s8/deck-pages.md "Attended check on the S8".
# Stops NIHardwareAgent with SIGKILL and restores it on every exit path. Each
# step says what to do in djay, asks for Enter, then streams for a fixed time.
# A step fails when the S8_SCREEN_STATUS order, the bridge exit status or an
# answer is wrong; the first failure (or the answer "stop") skips the rest and
# goes to recovery.
#   a-paused      decks 1 and 2 paused, 20 s
#   b-play        deck 1 played, paused and resumed by the observer, 60 s
#   c-four-decks  decks 3 and 4 in Four Decks, played and paused, 60 s
# Logs go to docs/s8/captures/deck-page-UTC/, outside target/.
# An optional argument names the first step to run; earlier steps are skipped.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
# shellcheck source=docs/s8/screen-lib.sh
. docs/s8/screen-lib.sh

step_names="a-paused b-play c-four-decks"
[ "$#" -le 1 ] || {
    echo "Usage: $0 [first-step]"
    exit 1
}
from="${1:-}"
case " $step_names " in *" ${from:-a-paused} "*) ;; *)
    echo "Unknown step '$from'. Steps: $step_names"
    exit 1
    ;;
esac
bridge=target/release/s4mk3-hid-midi-bridge
stream=macos/.build/release/S8Preview

screen_init deck-page
screen_preflight
cargo build --locked --quiet --release --bin s4mk3-hid-midi-bridge
swift build --package-path macos -c release --product S8Preview > "$run/swift-build.txt" 2>&1 ||
    {
        cat "$run/swift-build.txt"
        exit 1
    }

# Refuse before anything is stopped or sent when djay cannot be read.
if ! "$stream" --stream left=1 --seconds 1 > /dev/null 2> "$run/precheck.txt"; then
    cat "$run/precheck.txt"
    echo "Open djay and give this terminal Accessibility permission, then run again."
    exit 1
fi

restore() {
    if [ -z "$(agent_pid)" ]; then
        note "Restoring NIHardwareAgent."
        open -g "$bundle"
    fi
}
trap restore EXIT
trap 'exit 1' TERM HUP
trap : INT

note "Deck page check. Bridge: $bridge (release). Stream: $stream."
note "Start and stop decks in djay's window. Ctrl-C ends a step early; the panels then go black."
[ -z "$from" ] || note "Starting at $from; earlier steps are skipped."
snap pre
mark stop
kill -9 "$pid"
sleep 1
snap "stopped+1s"

stopped=""
skip=""

skipped() {
    [ -n "$from" ] || return 1
    [ "$1" != "$from" ] || {
        from=""
        return 1
    }
}

# run_step NAME SECONDS TARGETS SETUP
#   TARGETS  panel=deck list for S8Preview --stream, e.g. left=1,right=2
#   SETUP    what the observer prepares in djay before the step starts
run_step() {
    local name="$1" seconds="$2" targets="$3" setup="$4" status statuses
    if skipped "$name"; then
        skip="$name"
        return 0
    fi
    skip=""
    note "$name: $setup"
    ask "Next: $name, $seconds s, panels $targets. Enter to start, or 'stop'."
    [ "$answer" != stop ] || {
        stopped="before $name"
        return 1
    }
    mark "$name"
    set +e
    "$stream" --stream "$targets" --seconds "$seconds" 2> "$run/$name.stream.txt" |
        "$bridge" --device s8 --midi full --s8-screens-stdin --until-stopped 2>&1 | tee -i "$run/$name.txt"
    status=${PIPESTATUS[1]}
    set -e
    note "$name: bridge exit status $status, $(grep -c ' panel=' "$run/$name.stream.txt" || true) records written"
    statuses=$(sed -n 's/^S8_SCREEN_STATUS //p' "$run/$name.txt" | tr '\n' ',' | sed 's/,$//')
    note "$name: status lines: $statuses"
    snap "$name+1s"
    if [ "$statuses" != pending,active,inactive ]; then
        stopped="$name: status order '$statuses'"
        return 1
    fi
    if [ "$status" -ne 0 ]; then
        stopped="$name: bridge exit status $status"
        return 1
    fi
}

# verdict NAME QUESTION: y continues; anything else stops the run.
verdict() {
    [ "$skip" != "$1" ] || return 0
    ask "$1: $2 (y, n or describe; 'stop' ends the run)"
    case "$answer" in y | Y | yes) ;; *)
        stopped="$1: observer answered '$answer'"
        return 1
        ;;
    esac
}

steps() {
    run_step a-paused 20 left=1,right=2 \
        "Load decks 1 and 2 and leave both paused. Touch nothing during the step." || return 0
    verdict a-paused "left panel showed deck 1 and right panel deck 2, each with title, time, waveform and PAUSED" || return 0
    verdict a-paused "both times and both playheads stood still for the whole 20 s" || return 0
    verdict a-paused "both panels went black at the end" || return 0

    run_step b-play 60 left=1,right=2 \
        "During the step: play deck 1 for about 10 s, pause it for about 10 s, play it again, then pause it. Leave deck 2 paused." || return 0
    verdict b-play "while deck 1 played, the left panel said PLAYING and its time moved with djay's" || return 0
    verdict b-play "when deck 1 was paused, the left time stopped without stepping back and then stood still" || return 0
    verdict b-play "on resume the left time did not jump by the length of the pause (one move forward of less than 1 s when PLAYING appears is known, #87; answer y for that)" || return 0
    verdict b-play "the right panel (deck 2) stood still for the whole step" || return 0

    run_step c-four-decks 60 left=3,right=4 \
        "Switch djay to Four Decks with decks 3 and 4 loaded and paused. During the step: play deck 3 for about 10 s and pause it, then play deck 4 for about 10 s and pause it." || return 0
    verdict c-four-decks "left panel showed deck 3 and right panel deck 4" || return 0
    verdict c-four-decks "each time moved only while its own deck played, and on pause it stopped without stepping back and stood still (one move forward of less than 1 s when PLAYING appears is known, #87)" || return 0
}

steps
[ -z "$stopped" ] || note "Stopped: $stopped."
ask "Anything else seen on the panels (flicker, tearing, wrong colours, lag behind djay)? Describe, or Enter."

mark recover
[ -n "$(agent_pid)" ] || open -g "$bundle"
sleep 5
snap "recovered+5s"
ask "After recovery: what do the panels show (Controller Editor labels again)?"
ask "Press left PLAY once. Did Djay react? y/n"
mark end
trap - EXIT
note "Recorded in $run"
[ -z "$stopped" ]
