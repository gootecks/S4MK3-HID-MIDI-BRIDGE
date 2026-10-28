# S8 screen phase 0: NIHardwareAgent stop

Validation record for #32 ([screens.md](screens.md), phase 0). Nothing was
sent to the S8: no report, feature request, control transfer or bulk traffic.

## Question

Does the S8 keep sending Controller Editor MIDI to Djay when NIHardwareAgent
is gone? The answer decides between the hybrid setup (Evan's mapping on the
S8 MIDI port, the bridge on HID input and screens) and full bridge control.

## Status

| Part | Result |
| ---- | ------ |
| Interface ownership after SIGKILL | Verified, run 1 |
| Which MIDI endpoints survive | Verified, run 1 |
| Relaunch within 30 s | Verified: none, run 1 |
| Recovery with `open -g` | Verified, run 1 |
| PLAY, CUE and pad MIDI with the agent stopped | **Not tested**: needs someone at the S8 |
| Panel contents with the agent stopped | **Not observed** |

Run 1 was unattended. The S8 sends HID reports and MIDI only when a control
changes: the 85-second run recorded no HID report and no MIDI message, with
the agent running, stopped or restored. With nobody pressing controls, no run
can show whether MIDI survives. The attended check below answers it.

## Run 1 setup (2026-10-28, unattended)

- macOS 27.2, Apple Silicon. S8 in Controller Editor MIDI mode, one S8
  connected, no other NI controller.
- Djay Pro open, no track loaded. Controller Editor open. Traktor, Maschine
  and Komplete Kontrol not running.
- Passive recorder: shared-mode IOHIDManager on interface 5 plus every
  CoreMIDI source and destination whose name contains `Kontrol S8`.
- `ioreg` snapshots of the S8 device tree before the stop, 1, 5, 10, 20 and
  30 s after it, and 5 and 15 s after recovery.
- Raw logs stayed outside the repository.

## Run 1 results

Times are seconds from recorder start. The agent (pid 58018) was stopped with
SIGKILL at 3.3 s and relaunched with `open -g` on its app bundle at 33.3 s.

### Interface ownership

| Interface | Agent running | Agent stopped (1–30 s) | After recovery (5 s) |
| --------- | ------------- | ---------------------- | -------------------- |
| 3 MIDI | `MIDIServer` | `MIDIServer` | `MIDIServer` |
| 5 HID | Apple HID driver; `IOHIDLibUserClient` for NIHardwareAgent and the recorder | Apple HID driver; recorder only | NIHardwareAgent again |
| 6 BD | NIHardwareAgent user client | **no client** | NIHardwareAgent user client |
| Device | `MIDIServer` and two NIHardwareAgent device user clients | `MIDIServer` only | as before the stop |

- The S8 did not re-enumerate: the USB device kept its registry entry.
- The recorder's HID connection stayed open throughout; no detach.
- NIHardwareAgent also holds an HID user client on interface 5, which
  [input-capture.md](input-capture.md) did not list. It is a shared client,
  not a seizure: the recorder read alongside it.
- Brave Browser and ChatGPT also held device user clients the whole time,
  unaffected by the stop. Why they open the S8 was not investigated; they
  are not part of the S8 setup.

### MIDI endpoints

| Endpoint | Kind | Agent stopped |
| -------- | ---- | ------------- |
| `Traktor Kontrol S8` source and destination | Device (USB interface 3) | Stayed |
| `Traktor Kontrol S8 Input` source | NIHardwareAgent virtual port | Removed at the stop, back 0.4 s after relaunch |
| `Traktor Kontrol S8 Output` destination | NIHardwareAgent virtual port | Removed at the stop, back 0.4 s after relaunch |

All four local Djay mappings for the S8, including
`Evan's Traktor Kontrol S8 1.1`, bind endpoint `Traktor Kontrol S8`: the
device port that survives. A mapping bound to the `Input` or `Output` ports
would lose its connection whenever the agent stops.

The endpoint surviving does not show that the S8 still sends on it. Two
explanations fit run 1:

1. The S8 firmware produces the MIDI on interface 3 from the template the
   agent loaded. MIDI survives the stop, and the hybrid setup works.
2. The agent translates HID input and drives the MIDI output. MIDI stops with
   the agent, and the bridge must take over every control.

The October 27 capture recorded five MIDI messages for five PLAY presses
while both sources were connected, so each press arrives on exactly one
source. That capture did not record which one.

### Relaunch and recovery

- Nothing relaunched the agent within 30 seconds: not launchd, not
  Controller Editor. It runs as a LaunchServices application
  (`application.com.native-instruments.NIHardwareService…`), not as a
  keep-alive launchd job.
- `open -g` on
  `/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app`
  restored it. Within 5 seconds it held interfaces 5 and 6 and both device
  clients again, and its virtual MIDI ports were back.
- Panel contents after recovery were not observed.

## Attended check

Run from an interactive terminal with someone at the S8:

       mise run s8:agent-stop

The script (`docs/s8/agent-stop-check.sh`) checks for one S8 and one
verified NIHardwareAgent, starts `examples/s8-passive-recorder.rs`, then asks
for left PLAY, CUE A and pad 1 with the agent running, stops the agent with
SIGKILL, asks for the panel contents and the same three presses, waits out the
30 seconds, restores the agent and asks for a final PLAY. It ends with a
per-phase count of HID reports and MIDI messages per port. Raw output stays
in `target/s8-captures/`.

Reading the result:

| `stop` window shows | Meaning |
| ------------------- | ------- |
| `midi Traktor Kontrol S8` messages for PLAY and pad 1 | MIDI survives: continue with phase 1 (#33) |
| HID reports but no MIDI | MIDI needs the agent: re-plan around full bridge control or the NI-service route |
| No HID reports either | The S8 itself stopped reporting: stop and record |

CUE A sends no MIDI even with the agent running, so only its HID report is
expected.

## Not covered

- Behaviour with Controller Editor closed.
- What the panels show while the agent is stopped and after recovery.
- Whether the bridge's own process can restore the agent unattended in the
  same way; the S4 app's restore path was not run here.
