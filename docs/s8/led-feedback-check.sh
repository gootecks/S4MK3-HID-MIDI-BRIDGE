#!/bin/bash
# Attended check of the S8 deck LEDs driven from djay's feedback (#22, #23).
# Plan, expected results and record: docs/s8/led-feedback-validation.md.
#   led-feedback-check.sh [ROW]     ROW (L1-L21) is the first row to ask
# Builds the bridge, installs a new revision of the S8 mapping, stops
# NIHardwareAgent with SIGKILL and runs `--device s8 --midi full --s8-leds`.
# The runner writes nothing to the S8 itself; only the bridge does, and only
# the deck reports 0x80 and 0x81. It asks one row at a time and records each
# answer unchanged in notes.txt. Answering "stop" skips to the exit rows. On
# every exit path the bridge gets SIGINT (it then clears the LEDs) and
# NIHardwareAgent is restored.
# Logs go to docs/s8/captures/led-feedback-UTC/, outside target/.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
# shellcheck source=docs/s8/screen-lib.sh
. docs/s8/screen-lib.sh

live_rows="L1 L2 L3 L4 L5 L6 L7 L8 L9 L10 L11 L12 L13 L14 L15 L16 L17 L18 L19 L20 L21"
[ "$#" -le 1 ] || {
    echo "Usage: led-feedback-check.sh [ROW]"
    exit 1
}
from="${1:-}"
case " $live_rows " in
    *" ${from:-L1} "*) ;;
    *)
        echo "Unknown row $from. Rows: $live_rows"
        exit 1
        ;;
esac

bridge="target/debug/s4mk3-hid-midi-bridge"
mappings="$HOME/Music/djay/MIDI Mappings"
tmp=""
bridge_pid=""
stopped=""
skip=""
[ -z "$from" ] || skip=yes

screen_preflight
for pattern in '(^|/)s4mk3-hid-midi-bridge( |$)' '(^|/)S8Host( |$)' '(^|/)S8Preview( |$)'; do
    if pgrep -f "$pattern" > /dev/null; then
        echo "Another bridge, S8Host or S8Preview is running (matched $pattern). Stop it first."
        exit 1
    fi
done
pgrep -f 'djay.app/Contents/MacOS/' > /dev/null || {
    echo "Start djay first."
    exit 1
}
[ -d "$mappings" ] || {
    echo "Djay mapping folder missing: $mappings"
    exit 1
}
pgrep -f 'Controller Editor.app/Contents/MacOS/Controller Editor' > /dev/null ||
    echo "Warning: Controller Editor is not running. Started like this, the S8 was once dark and unusable; see docs/s8/translator-validation.md."

# Build before the agent is stopped.
cargo build --locked --quiet --bin s4mk3-hid-midi-bridge

screen_init led-feedback

# Install the generated mapping as the next revision; never overwrite. A rerun
# reuses the highest revision when it is the same file.
tmp="$(mktemp -d "${TMPDIR:-/tmp}/s8-led-mapping.XXXXXX")"
"$bridge" --device s8 --generate-mapping "$tmp"
generated="$tmp/Kontrol S8 Bridge.djayMidiMapping"
[ -f "$generated" ] || {
    echo "The generator did not write $generated"
    rm -rf "$tmp"
    exit 1
}
next=1
latest=""
for file in "$mappings"/"Kontrol S8 Bridge r"*.djayMidiMapping; do
    [ -e "$file" ] || continue
    number="${file##* r}"
    number="${number%.djayMidiMapping}"
    case "$number" in
        '' | *[!0-9]*) continue ;;
    esac
    if [ "$number" -ge "$next" ]; then
        next=$((number + 1))
        latest="$file"
    fi
done
if [ -n "$latest" ] && cmp -s "$generated" "$latest"; then
    name="Kontrol S8 Bridge r$((next - 1))"
else
    name="Kontrol S8 Bridge r$next"
    target="$mappings/$name.djayMidiMapping"
    if [ -e "$target" ]; then
        echo "Refusing to overwrite $target"
        rm -rf "$tmp"
        exit 1
    fi
    cp "$generated" "$target"
fi
rm -rf "$tmp"
tmp=""

stop_bridge() {
    [ -n "$bridge_pid" ] || return 0
    local status=0 tries=0
    kill -INT "$bridge_pid" 2> /dev/null || true
    while kill -0 "$bridge_pid" 2> /dev/null && [ "$tries" -lt 100 ]; do
        sleep 0.1
        tries=$((tries + 1))
    done
    if kill -0 "$bridge_pid" 2> /dev/null; then
        note "Bridge still running 10 s after SIGINT; sending SIGKILL. The LEDs were not cleared."
        kill -9 "$bridge_pid" 2> /dev/null || true
    fi
    wait "$bridge_pid" || status=$?
    bridge_pid=""
    note "Bridge exit status $status."
}

restore() {
    stop_bridge
    [ -z "$tmp" ] || rm -rf "$tmp"
    if [ -z "$(agent_pid)" ]; then
        note "Restoring NIHardwareAgent."
        open -g "$bundle"
    fi
}
trap restore EXIT
trap 'exit 1' TERM HUP
trap : INT

# check_bridge: false, and $stopped set, when the bridge died or turned the LEDs off.
check_bridge() {
    if ! kill -0 "$bridge_pid" 2> /dev/null; then
        stopped="bridge exited"
        note "The bridge is no longer running; see $run/bridge.log."
        return 1
    fi
    if grep -q -e S8_LEDS_DISABLED -e S8_LEDS_CLEAR_FAILED "$run/bridge.log"; then
        stopped="LED path disabled"
        note "The bridge logged S8_LEDS_DISABLED or S8_LEDS_CLEAR_FAILED; see $run/bridge.log."
        return 1
    fi
}

show_row() {
    printf '\n%s\n  Do:     %s\n  Expect: %s\n' "$1" "$2" "$3"
}

# step ID ACTION EXPECTED QUESTION: one asked row while the bridge runs.
step() {
    [ -z "$stopped" ] || return 0
    if [ -n "$skip" ]; then
        [ "$1" = "$from" ] || return 0
        skip=""
    fi
    check_bridge || return 0
    show_row "$1" "$2" "$3"
    ask "$1 $4"
    if [ "$answer" = stop ]; then
        stopped="$1"
        note "Stopped at $1."
    fi
}

# after ID ACTION EXPECTED QUESTION: an exit or recovery row; always asked.
after() {
    show_row "$1" "$2" "$3"
    ask "$1 $4"
}

plan() {
    step L1 \
        "Nothing is loaded on decks A to D. Look at both sides and touch nothing." \
        "Each side: PLAY, CUE, FLUX off; SYNC, loop ring, FX 1-4, ON 1-4, FX SELECT, BACK, CAPTURE, EDIT dim; HOTCUE full, LOOP, FREEZE, REMIX dim; SHIFT dim; DECK full (blue, decks A and B); pads, touchstrips and display buttons dark. Open in the output check: the dim level and which colour each dim LED takes." \
        "Per side, which LEDs are full, dim or dark, and how do the dim ones compare with the full ones? Describe."
    step L2 \
        "Look at the LEDs the bridge does not write: filter on and the arrows at the top of each deck, the mixer CUE buttons, MIC 1, MIC 2, SNAP, QUANTIZE." \
        "They stay as the firmware leaves them. The bridge writes only 0x80 and 0x81, so it does not change them." \
        "Which of these are lit now, and does any change when the bridge starts or later? Describe."
    step L3 \
        "In djay load one test track on deck A and one on deck B. Do not play." \
        "Left shows A, right shows B. PLAY off; strip blue bank dark while progress is 0; hot cue pads dark unless the track has saved cues, which light in their cue colours." \
        "What changed on each side after loading? Describe, including the pads and the strip."
    step L4 \
        "Press left PLAY to play deck A, then again to pause it." \
        "PLAY full while A plays, off when paused (root playing output)." \
        "Is left PLAY lit only while A plays? yes or no, describe if not."
    step L5 \
        "Pause deck A at its start. Hold left CUE, then release it. Do not hold Shift." \
        "CUE full while it is held, from the local state and from djay's CUE note. After the release it follows djay's CUE feedback." \
        "Is CUE lit during the hold and what does it show after the release? Describe."
    step L6 \
        "Play deck B, play deck A, press left SYNC to switch sync on, then again to switch it off." \
        "SYNC (green LED) full with sync on, dim with sync off. The red SYNC LED stays off. Open in the output check: the dim level." \
        "Colour and brightness of SYNC with sync on and off, and is the red SYNC LED lit? Describe."
    step L7 \
        "Press left FLUX, then again." \
        "FLUX full while slip is on, off otherwise." \
        "Does FLUX follow djay's slip mode? yes or no, describe if not."
    step L8 \
        "Press the left loop encoder to start a loop on deck A, then again to end it." \
        "All four ring segments full while the loop is active, dim otherwise: blue on decks A and B, white on C and D." \
        "What does the ring show in the loop and outside it: colour, brightness, segments? Describe."
    step L9 \
        "On the left side press HOTCUE, FREEZE, REMIX, then Shift + HOTCUE." \
        "The button of the active pad mode is full and the other three are dim. Shift + HOTCUE makes LOOP the full one." \
        "After each press, which of HOTCUE, LOOP, FREEZE, REMIX is full? Describe."
    step L10 \
        "On the left side select FREEZE, then Shift + HOTCUE, then REMIX, and look at the eight pads each time. Do not press a pad." \
        "FREEZE: all pads dim blue. Shift + HOTCUE: all pads dim green. REMIX: all pads dim orange. Dim is the lit colour divided by 6." \
        "Pad colour and brightness in each mode? Describe, including whether the dim pads are visible."
    step L11 \
        "Select HOTCUE. On deck A set a hot cue on pad 1 and pad 2, then delete the cue of pad 2 in djay." \
        "Set pads light in djay's cue colour (tokens 1-8 from the S4 bridge's default colours, 9 white); unset pads are dark; the deleted pad goes dark." \
        "Which colour do pad 1 and pad 2 show against djay's cue colours, and what does pad 2 show after the delete? Describe."
    step L12 \
        "Select REMIX. Press sampler pad 1 to play its sample, then stop it. The slot needs a sample." \
        "The pad is orange while its sample plays and dim orange when it stops. Deck A and deck C read the same sampler bank, B and D the other." \
        "Does the pad light orange while the sample plays and dim afterwards? Describe, or say the slot is empty."
    step L13 \
        "Press left FX 1, 2, 3 and 4 on, then off." \
        "Each FX LED is full while its effect is on, dim while off. The side shows the unit's target deck, chosen with the mixer FX buttons." \
        "Does each FX LED follow its effect on and off? Describe differences."
    step L14 \
        "Needs a stem track on deck A. Press left ON 1, 2, 3 and 4 to mute and unmute stems." \
        "Each ON LED is full when djay reports that stem muted, dim otherwise." \
        "Is an ON LED full when its stem is muted or when it is audible? Describe, or say the track has no stems."
    step L15 \
        "Press left DECK to switch to deck C, look at the left side, then press DECK again." \
        "On C the DECK LED turns white and the side shows C: PLAY off, pads dark, strip dark, ring and SYNC dim. Back on A it shows A's PLAY, SYNC, ring and pads again. Open in the output check: the order inside the DECK group." \
        "On C and back on A, what do DECK, PLAY, SYNC, the pads and the strip show? Describe the DECK colour."
    step L16 \
        "Repeat on the right side with deck B: play, hold CUE, SYNC, loop, FLUX, FX 1, a pad-mode button and a hot cue pad." \
        "The right side behaves as the left with its own report 0x81, independent of the left. Only PLAY on the right is established." \
        "Does each right-side LED behave as on the left, and does the left side stay unchanged? List any difference."
    step L17 \
        "Put deck A at the very start of its track (progress 0) and look at the left touchstrip." \
        "The blue bank is dark at progress 0; one LED once progress is above 0. Open in the output check: which row of the strip the blue bank lights." \
        "What does the left strip show, and on which row? Describe."
    step L18 \
        "Seek deck A to about half of the track and look at the left strip." \
        "A blue bar from the physical left end, about 12 or 13 of 25 LEDs." \
        "How long is the bar and does it start at the left end? Describe."
    step L19 \
        "Seek deck A to its last seconds, then let it play on." \
        "The bar grows to 24 or 25 LEDs near the end. Open in the output check: whether progress covers the whole track." \
        "How long is the bar near the end and does it follow the playhead? Describe."
    step L20 \
        "Touch the left strip at its left end without moving, slide to the middle and to the right end, then release." \
        "No orange dot at the touch alone; one orange dot from the first movement, at the finger's position; it goes out on release. The blue bar stays. Open in the output check: the bottom row." \
        "When does the orange dot appear, does it follow the finger and go out on release, and which row is it on? Describe."
    step L21 \
        "Hold left Shift, then release it." \
        "SHIFT full while held, dim otherwise. Nothing else on that side changes." \
        "Does left SHIFT light while held and does anything else change? Describe."
}

log_summary() {
    local enabled disabled failed
    enabled=$(grep -c S8_LEDS_ENABLED "$run/bridge.log" || true)
    disabled=$(grep -c S8_LEDS_DISABLED "$run/bridge.log" || true)
    failed=$(grep -c S8_LEDS_CLEAR_FAILED "$run/bridge.log" || true)
    note "Bridge log: S8_LEDS_ENABLED $enabled, S8_LEDS_DISABLED $disabled, S8_LEDS_CLEAR_FAILED $failed."
    note "Bridge processes left: $(pgrep -f '(^|/)s4mk3-hid-midi-bridge( |$)' | tr '\n' ' ')"
}

note "LED feedback check (#22, #23). Mapping: $name. Bridge: $bridge (debug)."
[ -z "$from" ] || note "Starting at $from; earlier rows are skipped."
snap pre
screen_agent_stop

mark start
"$bridge" --device s8 --midi full --s8-leds --until-stopped > >(tee -i "$run/bridge.log") 2>&1 &
bridge_pid=$!
ready=""
for _ in $(seq 1 50); do
    kill -0 "$bridge_pid" 2> /dev/null || break
    if grep -q MIDI_FULL_READY "$run/bridge.log" 2> /dev/null &&
        grep -q S8_LEDS_ENABLED "$run/bridge.log" 2> /dev/null; then
        ready=yes
        break
    fi
    sleep 0.2
done
[ -n "$ready" ] || {
    note "The bridge did not log MIDI_FULL_READY and S8_LEDS_ENABLED; see $run/bridge.log."
    exit 1
}
note "Bridge running, pid $bridge_pid."
cat << EOF

In djay: MIDI settings, device "Kontrol S8 MIDI Full", mapping "$name".
Answer each row with what you see; "stop" skips to the exit rows. Ctrl-C ends
the check: the bridge clears the LEDs and NIHardwareAgent is restored.
EOF
ask "Mapping selected? Press Enter."
mark rows
plan
[ -z "$stopped" ] || note "Stopped: $stopped."

mark stop-bridge
stop_bridge
sleep 1
log_summary
snap "bridge-stopped"
after L22 \
    "The bridge has stopped with Ctrl-C. NIHardwareAgent is still stopped. Look at both sides." \
    "Every LED the bridge owns is dark on both sides: buttons, pads, loop ring, strips. The LEDs of L2 are as before." \
    "Which LEDs are still lit on each side? Describe."

mark recover
[ -n "$(agent_pid)" ] || open -g "$bundle"
sleep 5
snap "recovered+5s"
after L23 \
    "NIHardwareAgent has been started again. Look at both panels and the LEDs." \
    "Controller Editor labels on both panels and the template's LEDs are back." \
    "What do the panels and the LEDs show? Describe."
after L24 \
    "Press left PLAY once, then display button 1 or 4." \
    "Djay reacts to PLAY through the Controller Editor template; the NI action of the display button works." \
    "Did djay react to left PLAY, and does the display button work again? Describe."
mark end
trap - EXIT INT TERM HUP
note "Recorded in $run"
[ -z "$stopped" ]
