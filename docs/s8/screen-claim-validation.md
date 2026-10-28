# S8 screen phase 1: claim interface 6

Validation record for #33 ([screens.md](screens.md), phase 1). The claim tool
(`examples/s8-screen-claim.rs`) sent nothing to the S8: no bulk or control
transfer, configuration change, reset or alternate setting.

## Answer

**The claim works.** With NIHardwareAgent stopped, the bridge's process can
claim interface 6 and release it, and nothing else is disturbed: the S8 did
not re-enumerate, HID input kept flowing, panels, LEDs and audio did not
change, and the agent took interface 6 back on relaunch.

| Part | Result |
| ---- | ------ |
| Claim with the agent stopped | Succeeded in 5 ms, tool exit 0 |
| Interface 6 owner during the hold | `s8-screen-claim` only |
| Release | Interface 6 had no client 1 s after release |
| HID input during and after the hold | Continued |
| Panels, LEDs, audio | No change reported |
| Re-enumeration | None: same device registry id throughout |
| Recovery with `open -g` | Verified: interfaces, labels, MIDI and Djay PLAY |

## Setup

As in [agent-stop-validation.md](agent-stop-validation.md): macOS 27.2, Apple
Silicon, one S8 in Controller Editor MIDI mode with Evan's template, Djay Pro
open with no track playing, Controller Editor open. Passive recorder on
interface 5 HID and every `Kontrol S8` MIDI endpoint. Run at 2026-10-28
08:35Z with `mise run s8:screen-claim` (`screen-claim-check.sh`). Raw logs
stayed outside the repository.

## Claim tool output

       0.004 opened device
       0.005 claimed interface 6; holding 5s
       5.008 released

## Input and MIDI per window

The tool ran in the claim window. Stop: agent killed, before the claim.
Released: after the claim, before recovery.

| Window | Presses asked | HID report 1 | MIDI on `Traktor Kontrol S8 Input` |
| ------ | ------------- | ------------ | ---------------------------------- |
| Baseline | PLAY, CUE A, pad 1 | 6 | PLAY ch 1 CC 10 on/off, pad 1 ch 1 CC 15 on/off |
| Stop | PLAY, CUE A, pad 1 | 6 | port gone |
| Claim (5 s) | PLAY, CUE A, pad 1 | 2 | port gone |
| Released | PLAY, CUE A, pad 1 | 10 | port gone |
| Recover | PLAY | 2 | PLAY ch 1 CC 10 on/off; Djay reacted |

- The claim window holds one press and release. The hold is only 5 s and the
  window ends when the tool exits; [INFERENCE] the remaining presses landed
  just after release. HID flowing during the claim is shown by those two
  reports, not by all three controls.
- CUE A again produced HID only, as in phase 0.

## Interface ownership

| Interface | Agent stopped | Claim + 2 s | Released + 1 s | Recovered + 5 s |
| --------- | ------------- | ----------- | -------------- | --------------- |
| 3 MIDI | `MIDIServer` | `MIDIServer` | `MIDIServer` | `MIDIServer` |
| 5 HID | recorder | recorder | recorder | recorder, NIHardwareAgent |
| 6 BD | no client | `s8-screen-claim` | no client | NIHardwareAgent |
| Device | `MIDIServer` | `MIDIServer`, `s8-screen-claim` | `MIDIServer` | `MIDIServer`, 2× NIHardwareAgent |

The device registry id stayed `0x1138e6119` in every snapshot. Brave Browser
and ChatGPT held device user clients throughout, as in phase 0.

## Not covered

- Whether nusb's host-side exclusive device open (`USBDeviceOpen`, tolerated
  on failure) succeeded; the tool has no logger. It causes no bus traffic.
  nusb read the active configuration from cached descriptors (one
  configuration), so no GET_CONFIGURATION request was sent.
- Holding the claim longer than 5 s, or claiming with the agent running.
- Any transfer on interface 6: that is phase 2.

## Rerunning

From an interactive terminal with someone at the S8:

       mise run s8:screen-claim

The script checks for one S8 and one verified NIHardwareAgent, starts the
recorder, asks for presses before and after a SIGKILL of the agent, runs the
claim tool with an `ioreg` snapshot during the hold, restores the agent and
prints per-window counts to `target/s8-captures/`.
