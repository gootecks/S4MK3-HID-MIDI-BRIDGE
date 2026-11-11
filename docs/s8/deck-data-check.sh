#!/bin/bash
# Attended check of the djay deck data on the S8 panels (#45).
# Plan, expected pictures and rows: docs/s8/deck-data-validation.md.
# SENDS SCREEN FRAMES TO THE S8 (interface 6, through the bridge's
# --s8-screens-stdin transport). Run it only with the approval that plan asks
# for. Stops NIHardwareAgent with SIGKILL, asks one question per row and
# restores the agent on every exit path. Answering "stop" goes to recovery.
#   part A  S8Preview --stream left=1,right=2 into the bridge, Two Decks
#   part B  S8Preview --stream left=3,right=4, Two Decks, then Four Decks
#   part C  S8Host --screens --deck 2, the app path
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
# shellcheck source=docs/s8/screen-lib.sh
source docs/s8/screen-lib.sh

bridge="target/release/s4mk3-hid-midi-bridge"
stream="macos/.build/release/S8Preview"
host=""
host_pid=""
fifo=""
log=""
stream_pid=""
bridge_pid=""
pipe=""
seen=0

restore() {
    [ -z "$stream_pid" ] || kill -TERM "$stream_pid" 2> /dev/null || true
    [ -z "$host_pid" ] || kill -INT "$host_pid" 2> /dev/null || true
    if [ -n "$bridge_pid" ]; then
        for _ in $(seq 1 20); do
            kill -0 "$bridge_pid" 2> /dev/null || break
            sleep 0.5
        done
        kill -INT "$bridge_pid" 2> /dev/null || true
    fi
    [ -z "$fifo" ] || rm -f "$fifo"
    [ -z "$pipe" ] || rm -f "$pipe"
    if [ -z "$(agent_pid)" ]; then
        note "Restoring NIHardwareAgent."
        open -g "$bundle"
    fi
}

stamp() {
    perl -MTime::HiRes=time -ne 'BEGIN { $| = 1 } printf "%.3f %s", time, $_'
}

# start_stream NAME TARGETS: S8Preview --stream TARGETS into the bridge.
start_stream() {
    local name="$1" targets="$2"
    log="$run/$name.stream.log"
    seen=0
    pipe="$(mktemp -u "${TMPDIR:-/tmp}/s8deck-records.XXXXXX")"
    mkfifo "$pipe"
    "$bridge" --device s8 --midi full --s8-screens-stdin --until-stopped < "$pipe" \
        > >(stamp > "$run/$name.bridge.log") 2>&1 &
    bridge_pid=$!
    "$stream" --stream "$targets" > "$pipe" 2> >(stamp > "$log") &
    stream_pid=$!
    mark "$name-started"
    for _ in $(seq 1 20); do
        grep -Eq 'S8_SCREEN_STATUS (active|failed)' "$run/$name.bridge.log" 2> /dev/null && break
        kill -0 "$bridge_pid" 2> /dev/null || break
        sleep 0.5
    done
    if ! grep -q 'S8_SCREEN_STATUS active' "$run/$name.bridge.log" 2> /dev/null; then
        note "$name: the screens did not become active within 10 s."
        cat "$log" "$run/$name.bridge.log"
        exit 1
    fi
}

# stop_stream NAME: end S8Preview; the bridge then sees its stdin close.
stop_stream() {
    local name="$1" status=0
    mark "$name-stop"
    kill -TERM "$stream_pid" 2> /dev/null || true
    wait "$stream_pid" 2> /dev/null || true
    stream_pid=""
    for _ in $(seq 1 30); do
        kill -0 "$bridge_pid" 2> /dev/null || break
        sleep 0.5
    done
    if kill -0 "$bridge_pid" 2> /dev/null; then
        note "$name: the bridge still runs 15 s after the stream ended; sending SIGINT."
        kill -INT "$bridge_pid"
    fi
    wait "$bridge_pid" || status=$?
    bridge_pid=""
    rm -f "$pipe"
    pipe=""
    note "$name: bridge exit status $status (expected 0), $(grep -c ' panel=' "$log" || true) records written."
    note "$name: status lines: $(grep -o 'S8_SCREEN_STATUS.*' "$run/$name.bridge.log" | tr '\n' ';')"
    note "$name: bridge processes left: $(pgrep -f "$bridge" | tr '\n' ' ')"
    snap "$name-stopped"
}

# sent ROW: the records S8Preview wrote since the previous call.
sent() {
    local to
    to="$(wc -l < "$log" | tr -d ' ')"
    note "$1: records since the previous row: $(awk -v from="$seen" -v to="$to" '
        NR > from && NR <= to && / panel=/ {
            side = ($4 == "panel=left") ? "left" : "right"
            count[side]++
            sub(/^[^ ]+ DECKSTREAM [^ ]+ panel=[a-z]+ /, "")
            last[side] = $0
        }
        END {
            printf "left %d", count["left"]
            if (count["left"]) printf " (last: %s)", last["left"]
            printf "; right %d", count["right"]
            if (count["right"]) printf " (last: %s)", last["right"]
        }
    ' "$log")"
    seen="$to"
}

# srow NUMBER QUESTION: a row asked while a stream runs.
srow() {
    row "$1. $2" || return 1
    sent "$1"
    if ! kill -0 "$bridge_pid" 2> /dev/null; then
        note "$1: the bridge is no longer running."
        answer=stop
        return 1
    fi
}

screen_preflight
pgrep -f 'djay.app/Contents/MacOS/' > /dev/null || {
    echo "Open djay Pro."
    exit 1
}
pgrep -f 'Controller Editor.app/Contents/MacOS/Controller Editor' > /dev/null ||
    echo "Warning: Controller Editor is not running. Started like this, the S8 was once dark and unusable; see docs/s8/translator-validation.md."

cargo build --locked --release --quiet --bin s4mk3-hid-midi-bridge
swift build --package-path macos -c release --product S8Preview
swift build --package-path macos --product S8Host
host="$(swift build --package-path macos --show-bin-path)/S8Host"

screen_init deck-data
# Refuse before anything is stopped or sent when djay cannot be read.
if ! "$stream" --stream left=1 --seconds 1 > /dev/null 2> "$run/precheck.txt"; then
    cat "$run/precheck.txt"
    echo "Open djay and give this terminal Accessibility permission, then run again."
    exit 1
fi
trap restore EXIT
trap 'exit 1' TERM HUP
trap : INT
snap pre
note "Deck data (#45). Stream: $stream, host: $host, bridge: $bridge."
note "Start, stop and load decks in djay's window."
ask "Before the stop: what do the two S8 panels show?"
ask "In djay: Two Decks layout, a track with cue points loaded and paused on decks 1 and 2, sidebar open on Library. Done?"

mark stop
kill -9 "$pid"
sleep 1
snap "stopped+1s"

part_a() {
    srow 1 "Left panel deck 1, right panel deck 2 (layout as 01-decks-1-2). Are title, artist, BPM and key on each panel the same as djay shows for that deck? y/n, or describe" || return 0
    srow 2 "Is REMAINING on each panel the same as the remaining time in djay? y/n, or describe" || return 0
    srow 3 "Cue flags above each waveform: the same number and colours, in the same order, as the deck's cue points in djay? Is the white playhead at the same place in the track as in djay's overview waveform? y/n, or describe" || return 0
    srow 4 "Play both decks, wait about 15 s, then pause both. Did both panels say PLAYING and both times move with djay's, without a panel stalling? y/n, or describe" || return 0
    srow 5 "Deck 1 paused: move it to another position (click in djay's overview waveform, or jump to a cue point). Do the time and the playhead on the left panel follow within about 1 s? y/n, or describe" || return 0
    srow 6 "Play deck 1 and jump again while it plays. Does the left panel follow the jump and go on moving with djay's time? Then pause deck 1. y/n, or describe" || return 0
    srow 7 "Move deck 2's tempo to about +8 %. Does BPM on the right panel change to the BPM djay shows? y/n, or describe" || return 0
    srow 8 "Play deck 2 for about 20 s at that tempo. Does REMAINING on the right panel stay the same as djay's? Then pause deck 2 and reset its tempo. y/n, or describe" || return 0
    srow 9 "Play deck 1, set a 4-beat loop for about 10 s, release it, pause. What did the left panel's time and playhead do during the loop? (looping is not modelled) Describe, or 'skip'." || return 0
    srow 10 "Load another track on deck 1. Does the left panel change to its title, artist, BPM, key, waveform and cue flags, with the playhead at the start? How long did it take? Describe." || return 0
    srow 11 "Optional: set a new cue point on deck 2 without reloading the track. Does a new flag appear on the right panel within about 10 s? y/n and how long, or 'skip'" || return 0
    srow 12 "Optional: load on deck 2 a track djay has not analysed. Does the right panel say 'No analysis for this track' and then show the waveform once djay has analysed it? How long after djay? Describe, or 'skip'." || return 0
    srow 13 "Optional: switch djay to One Deck view for about 10 s and back. Which deck did djay show? Did the other deck's panel say 'Deck N is not shown in djay' and return afterwards? Describe, or 'skip'." || return 0
    srow 14 "Any flicker, tearing, wrong colours or a frozen panel so far? Describe, or 'no'." || return 0
}

part_b() {
    srow 16 "djay in Two Decks: do the panels say 'Deck 3 is not shown in djay' and 'Deck 4 is not shown in djay' (as 02-decks-3-4-two-deck-layout)? y/n, or describe" || return 0
    srow 17 "Switch djay to Four Decks and load a track on decks 3 and 4 if they are empty. Do the panels fill with decks 3 and 4, with title, artist, BPM, key and remaining time as in djay? How long after the switch? Describe." || return 0
    srow 18 "Play deck 1 and deck 3 together for about 10 s, then pause both. Did the left panel move with deck 3's time, and did the right panel (deck 4) stand still? y/n, or describe" || return 0
    srow 19 "Switch djay back to Two Decks. Do the panels return to 'Deck 3 is not shown in djay' and 'Deck 4 is not shown in djay'? y/n, or describe" || return 0
}

part_c() {
    row "21. Right panel: the deck 2 page. Left panel: the library list (layout as 10-deck)? y/n, or describe" || return 0
    row "22. Play deck 2. PLAYING with a moving time and playhead on the right panel? Turn a left screen encoder through some rows: does the left panel follow while the deck page keeps moving? Then pause deck 2. y/n, or describe" || return 0
}

start_stream decks-1-2 left=1,right=2
note "Part A: decks 1 and 2 are on the panels."
part_a
stop_stream decks-1-2
[ "$answer" = stop ] || ask "15. After the stream ended, agent still stopped: what do the panels show? (expected: both black)"

if [ "$answer" != stop ]; then
    start_stream decks-3-4 left=3,right=4
    note "Part B: decks 3 and 4 are on the panels."
    part_b
    stop_stream decks-3-4
    [ "$answer" = stop ] || ask "20. After the stream ended: what do the panels show? (expected: both black)"
fi

if [ "$answer" != stop ]; then
    start_host host-deck-2 --deck 2 --log-deck-polls
    note "Part C: S8Host with the deck 2 page."
    part_c
    stop_host host-deck-2 eof
    [ "$answer" = stop ] || ask "23. After S8Host's stdin closed: what do the panels show? (expected: both black)"
fi

mark recover
[ -n "$(agent_pid)" ] || open -g "$bundle"
sleep 5
snap "recovered+5s"
ask "24. After recovery: what do the panels show (Controller Editor labels again)?"
ask "25. Press left PLAY once. Did djay react? y/n"
ask "26. Press display button 1 or 4. Does the NI action work again? y/n"
mark end
trap - EXIT INT
note "Recorded in $run"
