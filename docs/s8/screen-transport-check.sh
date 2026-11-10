#!/bin/bash
# Attended screen transport check (#44): continuous frames through
# `s8-screen-feed | bridge --device s8 --midi full --s8-screens-stdin`.
# Plan and expected pictures: docs/s8/screen-transport.md "Attended check".
# Stops NIHardwareAgent with SIGKILL, restores it on every exit path. Each step
# prints its expected pictures, asks for Enter, then runs the bridge with the
# feeder on its stdin. A step fails when the S8_SCREEN_STATUS order, the bridge
# exit status or the observer's answer is wrong; the first failure (or the
# answer "stop") skips the rest and goes to recovery.
#   a  left panel, 10 fps        f  feeder stalls with stdin open
#   b  right panel, 30 fps       g  feeder closes stdin
#   c  alternating, 10 fps       h  protocol error in the stream
#   d  alternating, 30 fps       i  Ctrl-C of the bridge
#   e  60 fps, above the cap
# Logs go to docs/s8/captures/screen-transport-UTC/, outside target/. Nothing
# uses the passive recorder: the bridge reads the S8 itself.
# An optional argument names the first step to run, e.g. c-alternate-10, to
# continue a run that stopped early. Earlier steps are skipped.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
# shellcheck source=docs/s8/screen-lib.sh
. docs/s8/screen-lib.sh

step_names="a-left-10 b-right-30 c-alternate-10 d-alternate-30 e-over-cap f-stall g-eof h-bad-record i-ctrl-c"
[ "$#" -le 1 ] || { echo "Usage: $0 [first-step]"; exit 1; }
from="${1:-}"
case " $step_names " in *" ${from:-a-left-10} "*) ;; *) echo "Unknown step '$from'. Steps: $step_names"; exit 1 ;; esac
pictures_dir=docs/s8/images/transport
bridge=target/release/s4mk3-hid-midi-bridge
feeder=target/release/examples/s8-screen-feed

screen_init screen-transport
screen_preflight
cargo build --locked --quiet --release --bin s4mk3-hid-midi-bridge --example s8-screen-feed

restore() {
    if [ -z "$(agent_pid)" ]; then
        note "Restoring NIHardwareAgent."
        open -g "$bundle"
    fi
}
trap restore EXIT
trap 'exit 1' TERM HUP
trap : INT

note "Screen transport check. Djay open, no track playing. Bridge: $bridge (release). Feeder: $feeder."
[ -z "$from" ] || note "Starting at $from; earlier steps are skipped."
snap pre
screen_agent_stop

stopped=""
status=0
statuses=""
elapsed=0
skip=""

# skipped NAME: true while NAME is before the first step to run.
skipped() {
    [ -n "$from" ] || return 1
    [ "$1" != "$from" ] || { from=""; return 1; }
}

# confirm NAME: prompt, return 1 and record on "stop".
confirm() {
    ask "Next: $1. Enter to start, or 'stop'."
    [ "$answer" != stop ] || { stopped="before $1"; return 1; }
    mark "$1"
}

# pictures NAME: print the expected pictures of a step.
pictures() {
    local found=0 file
    for file in "$pictures_dir/$1"-*.png "$pictures_dir/exit-black.png"; do
        [ -e "$file" ] || continue
        found=1
        note "  expected: $file"
    done
    [ "$found" -eq 1 ] || note "  no pictures found under $pictures_dir for $1"
}

# run_step NAME HOLD EXPECT GARBAGE FEEDER_ARGS...
#   HOLD     seconds stdin stays open after the feeder is done (0 = close at once)
#   EXPECT   status order as a comma list, glob allowed, e.g. pending,active,inactive
#   GARBAGE  1 writes 12 bytes that are not a record after the feeder
# The bridge always runs with --until-stopped: a bounded run exits 1 when no
# control was touched. Stdin closing or Ctrl-C ends it. Sets $status (bridge
# exit), $statuses and $elapsed.
run_step() {
    local name="$1" hold="$2" expect="$3" garbage="$4"
    if skipped "$name"; then skip="$name"; return 0; fi
    skip=""
    shift 4
    "$feeder" "$@" --dry-run > "$run/$name.dry"
    note "$name: dry run, $(wc -l < "$run/$name.dry" | tr -d ' ') records, hashes in $run/$name.dry"
    pictures "$name"
    confirm "$name" || return 1
    local begin
    begin=$(date +%s)
    set +e
    {
        "$feeder" "$@" 2> "$run/$name.feeder.txt"
        if [ "$garbage" = 1 ]; then printf 'NOPE\0\0\0\0\0\0\0\0'; fi
        sleep "$hold"
    } | "$bridge" --device s8 --midi full --s8-screens-stdin --until-stopped 2>&1 | tee "$run/$name.txt"
    status=${PIPESTATUS[1]}
    set -e
    elapsed=$(( $(date +%s) - begin ))
    note "$name: bridge exit status $status after $elapsed s"
    statuses=$(sed -n 's/^S8_SCREEN_STATUS //p' "$run/$name.txt" | tr '\n' ',' | sed 's/,$//')
    note "$name: status lines: $statuses"
    snap "$name+1s"
    # shellcheck disable=SC2053
    if [[ "$statuses" != $expect ]]; then stopped="$name: status order '$statuses', expected '$expect'"; return 1; fi
    if [ "$status" -ne 0 ]; then stopped="$name: bridge exit status $status"; return 1; fi
}

# verdict NAME QUESTION: y continues; anything else stops the run.
verdict() {
    [ "$skip" != "$1" ] || return 0
    ask "$1: $2 (y, n or describe; 'stop' ends the run)"
    case "$answer" in y | Y | yes) ;; *) stopped="$1: observer answered '$answer'"; return 1 ;; esac
}

steps() {
    run_step a-left-10 5 'pending,active,inactive' 0 --panels left --rate 10 --frames 50 || return 0
    verdict a-left-10 "left panel: moving bar, counter ends at 49 and holds 5 s, then black; right panel unchanged" || return 0

    run_step b-right-30 5 'pending,active,inactive' 0 --panels right --rate 30 --frames 150 || return 0
    verdict b-right-30 "right panel: steady bar, counter ends at 149 and holds 5 s, then black; left panel unchanged" || return 0

    run_step c-alternate-10 5 'pending,active,inactive' 0 --panels alternate --rate 10 --frames 50 || return 0
    verdict c-alternate-10 "both panels move, both counters end at 49 and hold, then both black; no panel shows the other's title" || return 0

    run_step d-alternate-30 5 'pending,active,inactive' 0 --panels alternate --rate 30 --frames 150 || return 0
    verdict d-alternate-30 "both panels steady without tearing or flicker, both counters end at 149, then both black" || return 0

    run_step e-over-cap 5 'pending,active,inactive' 0 --panels left --rate 60 --frames 180 || return 0
    verdict e-over-cap "left panel: bar moves in visible jumps (frames dropped by the 30 fps cap) but the counter ends at 179, then black" || return 0

    run_step f-stall 15 'pending,active,inactive' 0 --panels left --rate 10 --frames 30 || return 0
    verdict f-stall "left panel kept frame 29 unchanged and without flicker for the 15 s after the feeder stopped, then went black" || return 0

    run_step g-eof 0 'pending,active,inactive' 0 --panels alternate --rate 10 --frames 30 || return 0
    [ "$elapsed" -le 15 ] || { stopped="g-eof: bridge needed $elapsed s to stop after stdin closed"; return 0; }
    verdict g-eof "both panels went black within about a second of the last frame and the bridge exited by itself" || return 0

    run_step h-bad-record 5 'pending,active,failed *' 1 --panels left --rate 10 --frames 20 || return 0
    verdict h-bad-record "left panel counted to 19, then went black at once; the bridge kept running until the 5 s hold ended" || return 0

    note "i-ctrl-c: the feeder streams for 60 s. Press Ctrl-C once after 5 s or more."
    run_step i-ctrl-c 0 'pending,active,inactive' 0 --panels left --rate 10 --frames 600 || return 0
    verdict i-ctrl-c "the bar was moving; after Ctrl-C the left panel went black and the bridge exited" || return 0
}

steps
[ -z "$stopped" ] || note "Stopped: $stopped."

mark released
sleep 3
ask "Bridge gone, agent still stopped: what do the panels show? Photo."

mark recover
[ -n "$(agent_pid)" ] || open -g "$bundle"
sleep 5
snap "recovered+5s"
ask "After recovery: what do the panels show (Controller Editor labels again)? Photo."
ask "Press left PLAY once, then Enter."
ask "Did Djay react to left PLAY? y/n"
mark end
trap - EXIT
note "Recorded in $run"
[ -z "$stopped" ]
