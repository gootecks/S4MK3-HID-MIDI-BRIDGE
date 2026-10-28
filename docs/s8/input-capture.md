# S8 input capture on macOS (MIDI mode)

Passive capture for #10. Goal: find out whether the bridge can read S8
controls while the S8 runs Controller Editor MIDI mode for Djay, and which
controls produce no MIDI.

## Setup

- macOS, Apple Silicon. S8 in MIDI mode via Controller Editor (power-cycled
  after switching). Djay Pro open with `Evan's Traktor Kontrol S8 1.1`.
- `NIHardwareAgent` and Controller Editor running.
- A throwaway Swift recorder opened the S8 HID interface (5) through
  `IOHIDManager` without seizing it, and listened to both S8 CoreMIDI sources
  (`Traktor Kontrol S8`, `Traktor Kontrol S8 Input`).
- Nothing was sent to the S8. No report, feature request or control transfer.
- Field names and offsets come from `data/HID_INPUT.json` in
  [kontrol-s8-protocol](https://github.com/gusgustavodj/kontrol-s8-protocol)
  (CC BY 4.0). Offsets below exclude the report ID unless stated.

## Interface ownership (ioreg, same session)

| Interface | Client |
| --------- | ------ |
| 3 MIDI    | Apple USB MIDI driver; CoreMIDI source is device-backed |
| 5 HID     | `AppleUserUSBHostHIDDevice` (system HID driver) |
| 6 BD      | `NIHardwareAgent` user client (`AppleUSBHostInterfaceUserClient`) |

An earlier check with `NIHardwareAgent` not running showed no client on
interface 6.

## Results

Both input reports keep arriving in MIDI mode: report 1 (41 bytes) for buttons
and report 2 (109 bytes) for knobs and faders. Shared-mode reads did not
disturb Djay or the MIDI port.

| Control | HID field (report 1) | MIDI from S8 |
| ------- | -------------------- | ------------ |
| Left PLAY | `BtnPlayButtonLeft` | ch 1 CC 10 (127 / 0) |
| Right PLAY | `BtnPlayButtonRight` | ch 2 CC 10 (127 / 0) |
| CUE (PFL) A | `[19]` mask `0x08` | none |
| CUE (PFL) B | `[22]` mask `0x20` | none |
| CUE (PFL) C | `[19]` mask `0x04` | none |
| CUE (PFL) D | `[22]` mask `0x40` | none |
| Left display buttons 1–8 | `DISPLAY_LEFT_BTN_01..08` | none |
| Right display buttons 1–8 | `DISPLAY_RIGHT_BTN_01..08` | none |

Every press matched the dataset field and mask; no extra or missing bits.

Display button numbering: 1–4 down the left edge of each display, 5–8 down
the right edge. Two of them are handled by NI software or firmware, not MIDI:

- Button 1: display colour settings.
- Button 4: Controller Editor template switch (Hotcues ↔ Slicer). The pad
  MIDI changes with the template.

That leaves 12 display buttons (2, 3, 5–8 on each side) and the four CUE
buttons with no MIDI. They are available to the bridge without displacing
Evan's mapping.

## Fields not in the dataset

| Bytes (no report ID) | Behaviour | Reading |
| -------------------- | --------- | ------- |
| `[38]` bits `0x01`/`0x02`/`0x04`/`0x08` | Toggle on each CUE A/B/C/D press; survive release | Latched PFL state for A/B/C/D, kept by the S8 itself |
| `[25]`, `[26]`, `[31]`, `[32]` | Change on every report | Counter or timestamp; ignore for control decoding |

Byte `[38]` is `[39]` with the report ID. Two passes over the CUE buttons
turned all four bits on, then all off. A bridge can use it to keep CUE
indicators in step with the hardware latch.

## Not covered

- Report 2 fields were recorded but not checked control by control. One
  unidentified analog movement produced ch 1 CC 67 alongside report 2.
- Touchstrips, encoders, pads and the remaining mixer buttons.
- Controls pressed while `NIHardwareAgent` is stopped. Interface ownership
  and MIDI endpoints after a stop are in
  [agent-stop-validation.md](agent-stop-validation.md).
- Long sessions and reconnects.
