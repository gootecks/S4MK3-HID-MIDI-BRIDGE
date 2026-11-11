#!/bin/bash
# Attended translator check (#15): run the S8 bridge against Djay.
# Sends nothing to the S8. Stops NIHardwareAgent with SIGKILL and restores it.
# The bridge log goes to docs/s8/captures/translator-<UTC time>/bridge.log.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

bundle="/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"
exe="$bundle/Contents/MacOS/NIHardwareAgent"
mappings="$HOME/Music/djay/MIDI Mappings"
mapping="$mappings/Kontrol S8 Bridge.djayMidiMapping"
bridge="target/debug/s4mk3-hid-midi-bridge"
run="docs/s8/captures/translator-$(date -u +%Y%m%dT%H%M%SZ)"

agent_pid() { pgrep -f "$exe" || true; }

[ -t 0 ] || {
    echo "Run this from an interactive terminal."
    exit 1
}
[ "$(ioreg -r -c IOUSBHostDevice -n 'Traktor Kontrol S8' | grep -c '^+-o Traktor Kontrol S8@')" -eq 1 ] ||
    {
        echo "Connect exactly one Kontrol S8."
        exit 1
    }
if pgrep -i -f 'Traktor.app|Maschine|Komplete Kontrol' > /dev/null; then
    echo "Quit Traktor, Maschine and Komplete Kontrol first."
    exit 1
fi
[ -d "$mappings" ] || {
    echo "Djay mapping folder missing: $mappings"
    exit 1
}
pid=$(agent_pid)
if [ -n "$pid" ]; then
    [ "$(echo "$pid" | wc -w | tr -d ' ')" -eq 1 ] || {
        echo "Expected one NIHardwareAgent, found: $pid"
        exit 1
    }
    [ "$(ps -o command= -p "$pid")" = "$exe" ] || {
        echo "NIHardwareAgent identity mismatch."
        exit 1
    }
fi
pgrep -f 'Controller Editor.app/Contents/MacOS/Controller Editor' > /dev/null ||
    echo "Warning: Controller Editor is not running. Started like this, the S8 was once dark and unusable; see docs/s8/translator-validation.md."

cargo build --locked --quiet
mkdir -p "$run"
if [ -e "$mapping" ]; then
    echo "Keeping the existing mapping: $mapping"
    echo "Delete it first to test a regenerated mapping."
else
    "$bridge" --device s8 --generate-mapping "$mappings"
    echo "Wrote $mapping"
fi

restore() {
    if [ -z "$(agent_pid)" ]; then
        echo "Restoring NIHardwareAgent."
        open -g "$bundle"
    fi
}
trap restore EXIT
trap : INT

if [ -n "$pid" ]; then
    echo "Stopping NIHardwareAgent; the usual S8 setup is off until this ends."
    kill -9 "$pid"
    sleep 1
fi

cat << 'EOF'

In Djay: MIDI settings, device "Kontrol S8 MIDI Full", mapping "Kontrol S8 Bridge".
Move one control on each deck side and one mixer control first: the first
report of each kind only synchronises positions.
Then walk the rows in docs/s8/translator-validation.md. Ctrl-C ends the check.

EOF
"$bridge" --device s8 --midi full --until-stopped 2>&1 | tee -i "$run/bridge.log" || true

restore
trap - EXIT INT
sleep 5
echo
echo "Log: $run/bridge.log"
echo "Confirm the panels show Controller Editor labels and PLAY works with the usual setup."
