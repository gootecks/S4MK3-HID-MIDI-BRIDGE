# S8 screen phase 0: NIHardwareAgent stop

Validation record for #32 ([screens.md](screens.md), phase 0). Nothing was
sent to the S8: no report, feature request, control transfer or bulk traffic.

## Question

Does the S8 keep sending Controller Editor MIDI to Djay when NIHardwareAgent
is gone? The answer decides between the hybrid setup (Evan's mapping on the
S8 MIDI port, the bridge on HID input and screens) and full bridge control.

## Answer

**No.** In MIDI mode the S8's MIDI comes from NIHardwareAgent, not from the
controller. The agent reads HID input on interface 5 and publishes MIDI on
its own virtual port `Traktor Kontrol S8 Input`. With the agent stopped the
S8 still sends HID reports, but no MIDI arrives on any port and Djay does not
react. The hybrid setup does not work: with the agent stopped, the bridge has
to translate every S8 control itself, as it does for the S4.

| Part | Result |
| ---- | ------ |
| Interface ownership after SIGKILL | Verified, runs 1 and 2 |
| MIDI with the agent stopped | **None**, run 2 |
| HID input with the agent stopped | Continues, run 2 |
| Panel contents with the agent stopped | Last Controller Editor labels stay on both panels, run 2 |
| Relaunch within 30 s | None, runs 1 and 2 |
| Recovery with `open -g` | Verified: interfaces, MIDI, labels and Djay PLAY, run 2 |

## Setup

- macOS 27.2, Apple Silicon. S8 in Controller Editor MIDI mode with Evan's
  template (labels partly renamed by the user), one S8 connected, no other NI
  controller.
- Djay Pro open with `Evan's Traktor Kontrol S8 1.1`, no track playing.
  Controller Editor open. Traktor, Maschine and Komplete Kontrol not running.
- Passive recorder (`examples/s8-passive-recorder.rs`): shared-mode HID on
  interface 5 plus every CoreMIDI source and destination whose name contains
  `Kontrol S8`, following endpoints that appear or vanish.
- `ioreg` snapshots of the S8 device tree.
- Raw logs stayed outside the repository.

Run 1 (2026-10-28, 07:17Z) was unattended. The S8 sends HID reports and MIDI
only when a control changes, so it recorded ownership and endpoints but no
input. Run 2 (07:39Z) was the attended `mise run s8:agent-stop` check.

## Run 2: input and MIDI

Counts per window. Baseline: agent running. Stop: from SIGKILL until
recovery, about 5 minutes. Recover: after `open -g`.

| Window | Presses | HID report 1 | MIDI on `Traktor Kontrol S8 Input` | MIDI on `Traktor Kontrol S8` | Djay |
| ------ | ------- | ------------ | ---------------------------------- | ---------------------------- | ---- |
| Baseline | PLAY, CUE A, pad 1 | 8 | PLAY ch 1 CC 10 (2× on/off), pad 1 ch 1 CC 15 (on/off) | 0 | — |
| Stop | PLAY, CUE A, pad 1 | 8 | port gone | 0 | No reaction to PLAY |
| Recover | PLAY | 2 | PLAY ch 1 CC 10 (on/off) | 0 | — |

- The device port `Traktor Kontrol S8` (USB interface 3) carried no MIDI in
  any window, even with the agent running.
- CUE A produced HID reports only, as in [input-capture.md](input-capture.md).
- Both panels kept the last labels the agent drew while it was stopped: the
  left panel the edited Hotcue labels, the right the template defaults. After
  recovery they showed Controller Editor labels again.

Djay's mapping stores endpoint name `Traktor Kontrol S8`, yet Djay acted on
MIDI that arrived only on `Traktor Kontrol S8 Input`. Djay therefore does not
match endpoints by exact name. [INFERENCE] It probably matches by prefix or
device; this was not tested.

## Interface ownership (runs 1 and 2)

| Interface | Agent running | Agent stopped | After recovery (5 s) |
| --------- | ------------- | ------------- | -------------------- |
| 3 MIDI | `MIDIServer` | `MIDIServer` | `MIDIServer` |
| 5 HID | Apple HID driver; `IOHIDLibUserClient` for NIHardwareAgent and the recorder | recorder only | NIHardwareAgent again |
| 6 BD | NIHardwareAgent user client | **no client** | NIHardwareAgent user client |
| Device | `MIDIServer` and two NIHardwareAgent device user clients | `MIDIServer` only | as before the stop |

- The S8 did not re-enumerate, and the recorder's HID connection stayed open.
- NIHardwareAgent's interface 5 client is shared, not a seizure: the
  recorder read alongside it. This is the client it uses to produce MIDI.
- Brave Browser and ChatGPT also held device user clients the whole time,
  unaffected by the stop. Why they open the S8 was not investigated.

## MIDI endpoints

| Endpoint | Kind | Agent stopped |
| -------- | ---- | ------------- |
| `Traktor Kontrol S8` source and destination | Device (USB interface 3) | Stays; silent |
| `Traktor Kontrol S8 Input` source | NIHardwareAgent virtual port; carries the control MIDI | Removed; back 0.4 s after relaunch |
| `Traktor Kontrol S8 Output` destination | NIHardwareAgent virtual port | Removed; back 0.4 s after relaunch |

## Relaunch and recovery

- Nothing relaunched the agent while it was stopped (30 s in run 1, about
  5 minutes in run 2): not launchd, not Controller Editor. It runs as a
  LaunchServices application
  (`application.com.native-instruments.NIHardwareService…`), not as a
  keep-alive launchd job.
- `open -g` on
  `/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app`
  restored it. Within 5 seconds it held interfaces 5 and 6 and both device
  clients again, its virtual ports were back, the panels showed labels and
  PLAY reached Djay.

## Consequences

- Taking interface 6 for screens means stopping the agent, which removes all
  S8 MIDI. A bridge that drives the screens directly must also translate every
  control from HID, with its own Djay mapping (#13, #14).
- Keeping Evan's mapping and the agent means not claiming interface 6. Screens
  then need NI's own service route (Encdr's CFMessagePort path) or must wait.
- HID input keeps flowing without the agent, so full bridge control is not
  blocked by input.

## Rerunning

From an interactive terminal with someone at the S8:

       mise run s8:agent-stop

The script (`docs/s8/agent-stop-check.sh`) checks for one S8 and one
verified NIHardwareAgent and starts the recorder. It prompts for presses
before and after a SIGKILL of the agent, asks for the panel contents, waits at
least 30 seconds, restores the agent and prints per-window counts of HID
reports and MIDI per port to `target/s8-captures/`.

## Not covered

- Behaviour with Controller Editor closed.
- Which rule Djay uses to match the mapping's endpoint name.
- Whether the bridge's own process can restore the agent in the same way; the
  S4 app's restore path was not run here.
