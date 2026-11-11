#!/bin/bash
# Attended check of the display buttons and screen encoders (#51).
# Rows: docs/s8/display-buttons-validation.md "Not checked: needs the S8".
# Sends nothing to the S8. Stops NIHardwareAgent with SIGKILL, runs S8Host
# (which runs the bridge with --app-events), asks one question per row and
# restores the agent on every exit path. Answering "stop" goes to recovery.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
# shellcheck source=docs/s8/screen-lib.sh
source docs/s8/screen-lib.sh

bridge="target/release/s4mk3-hid-midi-bridge"
host=""
host_pid=""
fifo=""

restore() {
    [ -z "$host_pid" ] || kill -TERM "$host_pid" 2> /dev/null || true
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

screen_init display-buttons
trap restore EXIT
trap : INT
snap pre
note "Display buttons and screen encoders (#51). Host: $host, bridge: $bridge."
ask "Before the stop: what do the two S8 panels show?"

mark stop
kill -9 "$pid"
sleep 1
snap "stopped+1s"

fifo="$(mktemp -u "${TMPDIR:-/tmp}/s8host-stdin.XXXXXX")"
mkfifo "$fifo"
"$host" --bridge "$bridge" < "$fifo" \
    > >(perl -MTime::HiRes=time -ne 'BEGIN { $| = 1 } printf "%.3f %s", time, $_' > "$run/host.log") 2>&1 &
host_pid=$!
exec 9> "$fifo"
sleep 3
if ! kill -0 "$host_pid" 2> /dev/null; then
    wait "$host_pid" || note "S8Host exit status $? at start (3: Accessibility or djay missing; 4: bridge path)."
    host_pid=""
    cat "$run/host.log"
    exit 1
fi
mark started
note "S8Host is running. In djay: sidebar open on Library, click the main track table once so no sidebar row is selected."

rows() {
    row "1. Turn the left screen encoder 1 a few degrees (it has no clicks). How many rows did the sidebar selection move? (expected: at most one)" || return 0
    row "2. Press left button 3. Which djay tab is showing? (expected: Queue)" || return 0
    row "3. Hold left button 3 again, then release it. Did the release change anything? (expected: no)" || return 0
    row "4. Press left button 5. Tab? (expected: Match)" || return 0
    row "5. Press left button 6. Tab? (expected: History)" || return 0
    row "6. Press left button 7. Tab? (expected: Info)" || return 0
    row "7. Press right button 3. What shows? (expected: Info > Related)" || return 0
    row "8. Press right button 2. What shows? (expected: Info sub-tab)" || return 0
    row "9. Press left button 2. Tab? (expected: Library)" || return 0
    row "10. Press left button 8, then right buttons 5, 6, 7, 8. Any change in djay? (expected: no)" || return 0
    row "11. Press buttons 1 and 4 on both sides. Any change in djay or on the panels? (expected: no; the NI actions need the agent)" || return 0
    row "12. Left encoders 1 to 4, one at a time: clockwise, then back. Which moved the selection down on clockwise? (expected: all four)" || return 0
    row "13. Right encoders 1 to 4 the same way. Which moved down on clockwise? (expected: all four)" || return 0
    row "14. One full turn moves 20 rows. Too fast, too slow or right?" || return 0
    row "15. Spin an encoder fast through a long list. Does the selection keep up, lag or stop? Describe." || return 0
    row "16. Select a row with an encoder in Queue or History, then load or queue it with your usual control. Which control, and did it act on that row?" || return 0
    row "17. Did either S8 panel change at any time since the stop? (expected: no)" || return 0
}
rows

mark interrupt
kill -INT "$host_pid" 2> /dev/null || true
for _ in 1 2 3 4 5 6 7 8 9 10; do
    kill -0 "$host_pid" 2> /dev/null || break
    sleep 0.5
done
if kill -0 "$host_pid" 2> /dev/null; then
    note "S8Host still running 5 s after SIGINT; closing its stdin."
    exec 9>&-
    sleep 3
fi
status=0
if kill -0 "$host_pid" 2> /dev/null; then
    note "S8Host still running; sending SIGTERM."
    kill -TERM "$host_pid"
fi
wait "$host_pid" || status=$?
host_pid=""
exec 9>&- || true
note "S8Host exit status $status."
note "Bridge processes left: $(pgrep -f "$bridge" | tr '\n' ' ')"
snap "host-stopped"
ask "S8Host stopped, agent still stopped: what do the panels show?"

mark recover
[ -n "$(agent_pid)" ] || open -g "$bundle"
sleep 5
snap "recovered+5s"
ask "After recovery: what do the panels show (Controller Editor labels again)?"
ask "Press left PLAY once. Did Djay react? y/n"
ask "Press display button 1 or 4. Does the NI action work again? y/n"
mark end
rm -f "$fifo"
fifo=""
trap - EXIT INT
note "Recorded in $run"
