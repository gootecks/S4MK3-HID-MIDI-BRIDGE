#!/bin/bash
# Attended check of the library and deck views on the S8 panels (#27).
# Plan, expected pictures and rows: docs/s8/panel-views.md "Attended check".
# SENDS SCREEN FRAMES TO THE S8 (interface 6, through the bridge's
# --s8-screens-stdin transport). Run it only with the approval that plan asks
# for. Stops NIHardwareAgent with SIGKILL, runs S8Host --screens twice
# (library views, then the deck page with the deck reads logged), asks one
# question per row and restores the agent on every exit path. Answering
# "stop" goes to recovery.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
# shellcheck source=docs/s8/screen-lib.sh
source docs/s8/screen-lib.sh

bridge="target/release/s4mk3-hid-midi-bridge"
host=""
host_pid=""
fifo=""
log=""
gap_from=0

restore() {
    [ -z "$host_pid" ] || kill -INT "$host_pid" 2> /dev/null || true
    [ -z "$fifo" ] || rm -f "$fifo"
    if [ -z "$(agent_pid)" ]; then
        note "Restoring NIHardwareAgent."
        open -g "$bundle"
    fi
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

# gaps LABEL: the deck reads S8Host logged since the previous call.
gaps() {
    local now
    now="$(perl -MTime::HiRes=time -e 'printf "%.3f", time')"
    note "$1: $(awk -v from="$gap_from" -v to="$now" '
        / deck poll gap=/ && $1 >= from && $1 < to {
            reads++
            split($5, gap, "=")
            split($6, read, "=")
            if (gap[2] != "-" && gap[2] + 0 > longest) longest = gap[2] + 0
            if (gap[2] != "-" && gap[2] + 0 > 0.45) late++
            if (read[2] + 0 > slowest) slowest = read[2] + 0
        }
        END {
            printf "deck reads %d, longest gap %.3f s, gaps over 0.45 s %d, longest read %.3f s",
                reads, longest, late, slowest
        }
    ' "$log")"
    gap_from="$now"
}

deck_rows() {
    row "18. Right panel: deck 1 page with your track's title, time, BPM, key and waveform (layout as 10-deck)? Left panel: the library list? y/n, or describe" || return 0
    gaps "18 start"
    row "19. Press PLAY on deck 1. Does the page say PLAYING and do the time and the playhead move? Press it again: PAUSED? Then press PLAY once more and leave it playing. y/n, or describe" || return 0
    gaps "19 play and pause"
    row "20. Turn a left screen encoder through at least twenty rows. Does the left panel follow the selection while the deck page keeps moving on the right? y/n, or describe" || return 0
    gaps "20 scrolling"
    row "21. Press left button 3, then 6, then 2. Does the left panel show Queue, History and Library in turn while the deck page keeps moving? y/n, or describe" || return 0
    gaps "21 tab switching"
}

if [ "$answer" != stop ]; then
    start_host deck --deck 1 --log-deck-polls
    note "Deck page is running."
    deck_rows
    stop_host deck eof
    ask "22. After S8Host's stdin closed: what do the panels show? (expected: both black)"
fi
[ -z "$host_pid" ] || stop_host recovery int

mark recover
[ -n "$(agent_pid)" ] || open -g "$bundle"
sleep 5
snap "recovered+5s"
ask "23. After recovery: what do the panels show (Controller Editor labels again)?"
ask "24. Press left PLAY once. Did djay react? y/n"
ask "25. Press display button 1 or 4. Does the NI action work again? y/n"
mark end
trap - EXIT INT
note "Recorded in $run"
