#!/bin/bash
# Launcher of the S8 kit (#131). `mise run s8:kit` copies it into the kit as
# "Start S8.command"; it runs from the kit folder on a Mac without the
# repository. Procedure: docs/s8/kit.md.
# SENDS SCREEN FRAMES AND, WHEN ASKED, DECK LED REPORTS TO THE S8. Stops
# NIHardwareAgent with SIGKILL and restores it on every exit path.
set -euo pipefail
cd "$(dirname "$0")"

bundle="/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"
exe="$bundle/Contents/MacOS/NIHardwareAgent"
mappings="$HOME/Music/djay/MIDI Mappings"
host="./S8Host"
bridge="./s4mk3-hid-midi-bridge"

agent_pid() { pgrep -f "$exe" || true; }
fail() {
    echo "$1"
    exit 1
}
djay_running() { pgrep -f 'djay.app/Contents/MacOS/' > /dev/null; }

[ -t 0 ] || fail "Run this from a Terminal window."
[ "$(uname -m)" = arm64 ] || fail "This kit is built for Apple Silicon Macs."
[ -x "$host" ] && [ -x "$bridge" ] || fail "S8Host or the bridge is missing from $(pwd)."
[ -d "$bundle" ] || fail "NIHardwareAgent is not installed: without it the usual S8 setup cannot be restored."
[ "$(ioreg -r -c IOUSBHostDevice -n 'Traktor Kontrol S8' | grep -c '^+-o Traktor Kontrol S8@')" -eq 1 ] ||
    fail "Connect exactly one Kontrol S8."
if pgrep -i -f 'Traktor.app|Maschine|Komplete Kontrol' > /dev/null; then
    fail "Quit Traktor, Maschine and Komplete Kontrol first."
fi

mapping="$(ls ./*.djayMidiMapping)"
name="$(basename "$mapping" .djayMidiMapping)"
[ -d "$mappings" ] || fail "djay's mapping folder is missing: $mappings. Start djay Pro once, then start again."
if [ ! -e "$mappings/$name.djayMidiMapping" ]; then
    cp -n "$mapping" "$mappings/"
    echo "Installed the mapping \"$name\" in $mappings."
    if djay_running; then
        fail "Quit djay Pro and open it again so that it sees the mapping, then start again."
    fi
fi

"$host" --check || fail "Nothing was stopped and nothing was sent to the S8."

pid=$(agent_pid)
if [ -n "$pid" ]; then
    [ "$(echo "$pid" | wc -w | tr -d ' ')" -eq 1 ] || fail "Expected one NIHardwareAgent, found: $pid"
    [ "$(ps -o command= -p "$pid")" = "$exe" ] || fail "NIHardwareAgent identity mismatch."
fi
if ! pgrep -f 'Controller Editor.app/Contents/MacOS/Controller Editor' > /dev/null; then
    echo "Controller Editor is not running. Started like this, the S8 was once dark and unusable."
    read -r -p "Open Controller Editor and press Return, or type 'go' to start without it: " answer
    [ "$answer" = go ] ||
        pgrep -f 'Controller Editor.app/Contents/MacOS/Controller Editor' > /dev/null ||
        fail "Controller Editor is still not running."
fi

arguments=(--bridge "$bridge" --screens)
read -r -p "Deck LEDs from djay? Return for yes, n for no: " answer
[ "$answer" = n ] || arguments+=(--leds)
read -r -p "Deck page on the right panel: 1-4, or Return for the selected track: " answer
case "$answer" in
    [1-4]) arguments+=(--deck "$answer") ;;
    "") ;;
    *) fail "Not a deck: $answer" ;;
esac

mkdir -p logs
log="logs/s8-$(date -u +%Y%m%dT%H%M%SZ).log"

restore() {
    if [ -z "$(agent_pid)" ]; then
        echo "Restoring NIHardwareAgent."
        open -g "$bundle"
    fi
}
trap restore EXIT
trap 'exit 1' TERM HUP
trap : INT

if [ -n "$pid" ]; then
    echo "Stopping NIHardwareAgent; the usual S8 setup is off until this ends."
    kill -9 "$pid"
    sleep 1
fi

cat << EOF

In djay Pro: Settings > MIDI, device "Kontrol S8 MIDI Full", mapping "$name".
Move one control on each deck side and one mixer control first: the first
report of each kind only synchronises positions.
Ctrl-C in this window stops the bridge and brings the usual S8 setup back.

EOF
"$host" "${arguments[@]}" 2>&1 | tee -i "$log" || true

restore
trap - EXIT INT TERM HUP
sleep 5
echo
echo "Log: $(pwd)/$log"
echo "Check that the panels show the Controller Editor labels and that PLAY works with the usual setup."
