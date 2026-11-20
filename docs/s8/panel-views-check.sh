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

rows_library="1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16"
rows_deck="18 19 20 21"
screen_selection "$rows_library $rows_deck" "$@"
if [ "$screen_mode" = plan ]; then
    screen_part "Library" "$rows_library" 17 || true
    screen_part "Deck" "$rows_deck" 22 || true
    echo "Recovery: rows 23 24 25"
    exit 0
fi

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
    {
        echo "Open djay Pro with its library sidebar showing."
        exit 1
    }
pgrep -f 'Controller Editor.app/Contents/MacOS/Controller Editor' > /dev/null ||
    echo "Warning: Controller Editor is not running. Started like this, the S8 was once dark and unusable; see docs/s8/translator-validation.md."

cargo build --locked --release --quiet
swift build --package-path macos --product S8Host
host="$(swift build --package-path macos --show-bin-path)/S8Host"
screen_build_context

screen_init panel-views
if ! "$screen_context" > "$run/precheck.txt" 2>&1; then
    cat "$run/precheck.txt"
    echo "Open djay and give this terminal Accessibility permission, then run again."
    exit 1
fi
trap restore EXIT
trap 'exit 1' TERM HUP
trap : INT
snap pre
note "Panel views (#27). Host: $host, bridge: $bridge."
ask "Before the stop: what do the two S8 panels show?"
[ "$answer" != stop ] || exit 0
ask "In djay: sidebar open on Library, a track loaded on deck 1. Click a row of the main track table so that no sidebar row is selected. Done?"
[ "$answer" != stop ] || exit 0

mark stop
kill -9 "$pid"
sleep 1
snap "stopped+1s"

library_rows() {
    screen_row "1. Both panels: do they match 01-library (list without highlight; 'No track selected')? y/n, or describe" || return 0
    screen_row "2. Turn left screen encoder 1 until a sidebar track is selected, then two rows further. Do the highlight and the right panel follow each row (as 02-library-selected)? y/n, or describe" || return 0
    screen_row "3. Turn left screen encoder 1 past the last visible row. Does the list scroll and keep the highlight visible (as 03-library-scrolled)? y/n, or describe" || return 0
    screen_row "4. If necessary, select a sidebar track with the left screen encoder. Is 'n of m' on the right panel the position of the row among the sidebar's tracks? y/n, or describe" || return 0
    screen_row "5. Press left button 3. Do the panels show Queue (as 04-queue, or your queue)? y/n, or describe" || return 0
    screen_row "6. Press left button 5. Match (as 05-match)? y/n, or describe" || return 0
    screen_row "7. Press left button 6. History (as 06-history, with your sessions)? y/n, or describe" || return 0
    screen_row "8. Press left button 7. Info, and Info/Related labels on right buttons 2 and 3 (as 07-info)? y/n, or describe" || return 0
    screen_row "9. Switch to Info with left button 7 if needed. Press right button 3. Related (as 08-related)? Then right button 2: Info again? y/n, or describe" || return 0
    screen_row "10. Press left button 2. Library again? y/n, or describe" || return 0
    screen_row "11. If needed, switch to Library with left button 2. With the mouse, select another sidebar row in djay. How long until the panels follow? (expected: about 1 s)" || return 0
    screen_row "12. Close djay's sidebar. Does the left panel say 'Open the djay sidebar' (as 09-sidebar-closed)? Open it again: does the list return? y/n, or describe" || return 0
    screen_row "13. From an encoder turn to the panel change: instant, a short lag, or seconds? Describe." || return 0
    screen_row "14. Spin an encoder fast through a long list. Do the panels end on the row djay selected? y/n, or describe" || return 0
    screen_row "15. Press left PLAY once. Did djay react (bridge MIDI while the screens run)? y/n" || return 0
    screen_row "16. Any flicker, tearing, wrong colours or a frozen panel at any time? Describe, or 'no'." || return 0
}

if screen_part Library "$rows_library" 17; then
    start_host library
    note "Library views are running."
    library_rows
    stop_host library int
    screen_stop_row "17. After Ctrl-C, agent still stopped: what do the panels show? (expected: both black)"
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

deck_row() {
    local question="$1" label="$2" number="${1%%.*}"
    screen_selected "$number" || return 0
    row "$question" || return 1
    gaps "$number $label"
}

deck_rows() {
    deck_row "18. Right panel: deck 1 page with your track's title, time, BPM, key and waveform (layout as 10-deck)? Left panel: the library list? y/n, or describe" start || return 0
    deck_row "19. With deck 1 paused, press PLAY. Does the page say PLAYING and do the time and the playhead move? Press it again: PAUSED? Then press PLAY once more and leave it playing. y/n, or describe" "play and pause" || return 0
    deck_row "20. Start deck 1 if it isn't playing. Turn a left screen encoder through at least twenty rows. Does the left panel follow the selection while the deck page keeps moving on the right? y/n, or describe" scrolling || return 0
    deck_row "21. Start deck 1 if it isn't playing. Press left button 3, then 6, then 2. Does the left panel show Queue, History and Library in turn while the deck page keeps moving? y/n, or describe" "tab switching" || return 0
}

if [ "$answer" != stop ] && screen_part Deck "$rows_deck" 22; then
    start_host deck --deck 1 --log-deck-polls
    note "Deck page is running."
    deck_rows
    stop_host deck eof
    screen_stop_row "22. After S8Host's stdin closed: what do the panels show? (expected: both black)"
fi

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
