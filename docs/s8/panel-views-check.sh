#!/bin/bash
# Attended check of the library and deck views on the S8 panels (#27).
# Plan, expected pictures and rows: docs/s8/panel-views.md "Attended check".
# SENDS SCREEN FRAMES TO THE S8 (interface 6, through the bridge's
# --s8-screens-stdin transport). Run it only with the approval that plan asks
# for. Stops NIHardwareAgent with SIGKILL, runs S8Host --screens twice
# (library views, then the deck page), asks one question per row and restores
# the agent on every exit path. Answering "stop" goes to recovery.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
# shellcheck source=docs/s8/screen-lib.sh
source docs/s8/screen-lib.sh

bridge="target/release/s4mk3-hid-midi-bridge"
host=""
host_pid=""
fifo=""
log=""

restore() {
    [ -z "$host_pid" ] || kill -INT "$host_pid" 2> /dev/null || true
    [ -z "$fifo" ] || rm -f "$fifo"
    if [ -z "$(agent_pid)" ]; then
        note "Restoring NIHardwareAgent."
        open -g "$bundle"
    fi
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

screen_preflight
pgrep -f 'djay.app/Contents/MacOS/' > /dev/null ||
    { echo "Open djay Pro with its library sidebar showing."; exit 1; }
pgrep -f 'Controller Editor.app/Contents/MacOS/Controller Editor' > /dev/null ||
    echo "Warning: Controller Editor is not running. Started like this, the S8 was once dark and unusable; see docs/s8/translator-validation.md."

cargo build --locked --release --quiet
swift build --package-path macos --product S8Host
host="$(swift build --package-path macos --show-bin-path)/S8Host"

screen_init panel-views
trap restore EXIT
trap : INT
snap pre
note "Panel views (#27). Host: $host, bridge: $bridge."
ask "Before the stop: what do the two S8 panels show?"
ask "In djay: sidebar open on Library, a track loaded on deck 1. Click a row of the main track table so that no sidebar row is selected. Done?"

mark stop
kill -9 "$pid"
sleep 1
snap "stopped+1s"

library_rows() {
    row "1. Both panels: do they match 01-library (list without highlight; 'No track selected')? y/n, or describe" || return 0
    row "2. Turn left screen encoder 1 until a sidebar track is selected, then two rows further. Do the highlight and the right panel follow each row (as 02-library-selected)? y/n, or describe" || return 0
    row "3. Keep turning past the last visible row. Does the list scroll and keep the highlight visible (as 03-library-scrolled)? y/n, or describe" || return 0
    row "4. Is 'n of m' on the right panel the position of the row among the sidebar's tracks? y/n, or describe" || return 0
    row "5. Press left button 3. Do the panels show Queue (as 04-queue, or your queue)? y/n, or describe" || return 0
    row "6. Press left button 5. Match (as 05-match)? y/n, or describe" || return 0
    row "7. Press left button 6. History (as 06-history, with your sessions)? y/n, or describe" || return 0
    row "8. Press left button 7. Info, and Info/Related labels on right buttons 2 and 3 (as 07-info)? y/n, or describe" || return 0
    row "9. Press right button 3. Related (as 08-related)? Then right button 2: Info again? y/n, or describe" || return 0
    row "10. Press left button 2. Library again? y/n, or describe" || return 0
    row "11. With the mouse, select another sidebar row in djay. How long until the panels follow? (expected: about 1 s)" || return 0
    row "12. Close djay's sidebar. Does the left panel say 'Open the djay sidebar' (as 09-sidebar-closed)? Open it again: does the list return? y/n, or describe" || return 0
    row "13. From an encoder turn to the panel change: instant, a short lag, or seconds? Describe." || return 0
    row "14. Spin an encoder fast through a long list. Do the panels end on the row djay selected? y/n, or describe" || return 0
    row "15. Press left PLAY once. Did djay react (bridge MIDI while the screens run)? y/n" || return 0
    row "16. Any flicker, tearing, wrong colours or a frozen panel at any time? Describe, or 'no'." || return 0
}

start_host library
note "Library views are running."
library_rows
if [ "$answer" != stop ]; then
    stop_host library int
    ask "17. After Ctrl-C, agent still stopped: what do the panels show? (expected: both black)"
fi

deck_rows() {
    row "18. Right panel: deck 1 page with your track's title, time, BPM, key and waveform (layout as 10-deck)? Left panel: the library list? y/n, or describe" || return 0
    row "19. Press PLAY on deck 1. Does the page say PLAYING and do the time and the playhead move? Press it again: PAUSED? y/n, or describe" || return 0
    row "20. Turn a left screen encoder. Does the left panel still follow the selection while the deck page stays on the right? y/n, or describe" || return 0
}

if [ "$answer" != stop ]; then
    start_host deck --deck 1
    note "Deck page is running."
    deck_rows
    stop_host deck eof
    ask "21. After S8Host's stdin closed: what do the panels show? (expected: both black)"
fi
[ -z "$host_pid" ] || stop_host recovery int

mark recover
[ -n "$(agent_pid)" ] || open -g "$bundle"
sleep 5
snap "recovered+5s"
ask "22. After recovery: what do the panels show (Controller Editor labels again)?"
ask "23. Press left PLAY once. Did djay react? y/n"
ask "24. Press display button 1 or 4. Does the NI action work again? y/n"
mark end
trap - EXIT INT
note "Recorded in $run"
