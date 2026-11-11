---
summary: "Passive HID and MIDI capture in Controller Editor MIDI mode: controls without MIDI and undocumented bytes (#10)."
type: validation
---

# S8 input capture on macOS (MIDI mode)

Passive capture for #10. Goal: find out whether the bridge can read S8
controls while the S8 runs Controller Editor MIDI mode for Djay, and which
controls produce no MIDI.

<?toc
max-level: 2
?>

- [Setup](#setup)
- [Interface ownership (ioreg, same session)](#interface-ownership-ioreg-same-session)
- [Results](#results)
- [Fields not in the dataset](#fields-not-in-the-dataset)
- [Not covered](#not-covered)
- [Agent-stopped capture (2026-11-09)](#agent-stopped-capture-2026-11-09)

<?/toc?>

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

| Interface | Client                                                            |
| --------- | ----------------------------------------------------------------- |
| 3 MIDI    | Apple USB MIDI driver; CoreMIDI source is device-backed           |
| 5 HID     | `AppleUserUSBHostHIDDevice` (system HID driver)                   |
| 6 BD      | `NIHardwareAgent` user client (`AppleUSBHostInterfaceUserClient`) |

An earlier check with `NIHardwareAgent` not running showed no client on
interface 6.

## Results

Both input reports keep arriving in MIDI mode: report 1 (41 bytes) for buttons
and report 2 (109 bytes) for knobs and faders. Shared-mode reads did not
disturb Djay or the MIDI port.

| Control                   | HID field (report 1)       | MIDI from S8         |
| ------------------------- | -------------------------- | -------------------- |
| Left PLAY                 | `BtnPlayButtonLeft`        | ch 1 CC 10 (127 / 0) |
| Right PLAY                | `BtnPlayButtonRight`       | ch 2 CC 10 (127 / 0) |
| CUE (PFL) A               | `[19]` mask `0x08`         | none                 |
| CUE (PFL) B               | `[22]` mask `0x20`         | none                 |
| CUE (PFL) C               | `[19]` mask `0x04`         | none                 |
| CUE (PFL) D               | `[22]` mask `0x40`         | none                 |
| Left display buttons 1–8  | `DISPLAY_LEFT_BTN_01..08`  | none                 |
| Right display buttons 1–8 | `DISPLAY_RIGHT_BTN_01..08` | none                 |

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

| Bytes (no report ID)                    | Behaviour                                         | Reading                                              |
| --------------------------------------- | ------------------------------------------------- | ---------------------------------------------------- |
| `[38]` bits `0x01`/`0x02`/`0x04`/`0x08` | Toggle on each CUE A/B/C/D press; survive release | Latched PFL state for A/B/C/D, kept by the S8 itself |
| `[25]`, `[26]`, `[31]`, `[32]`          | Change on every report                            | Counter or timestamp; ignore for control decoding    |

Byte `[38]` is `[39]` with the report ID. Two passes over the CUE buttons
turned all four bits on, then all off. A bridge can use it to keep CUE
indicators in step with the hardware latch.

## Not covered

- Report 2 fields were recorded but not checked control by control in this
  pass. One unidentified analog movement produced ch 1 CC 67 alongside
  report 2.
- Touchstrips, encoders, pads and the remaining mixer buttons, and controls
  pressed while `NIHardwareAgent` is stopped. The agent-stopped capture
  below covers them. Interface ownership and MIDI endpoints after a stop are
  in [agent-stop-validation.md](agent-stop-validation.md).
- Long sessions and reconnects.

## Agent-stopped capture (2026-11-09)

Attended capture for #10 with `mise run s8:input-capture`: `NIHardwareAgent`
stopped, interface 5 opened shared and read-only by `examples/s8-input-diff`,
nothing sent to the S8. All 191 rows of
[capture-checklist.md](capture-checklist.md) were stamped across five runs;
where a row was stamped more than once, its last run counts. The raw logs
were kept in `target/s8-captures/` and were lost on 2026-11-09 when the build
cache was garbage-collected. A copy of the reports recovered from the analysis
session is committed in
[captures/2026-11-09-input-capture](captures/2026-11-09-input-capture/README.md),
alongside the reduced fixture described under
[Decoder fixture](#decoder-fixture-12). Keep capture logs outside `target/`.

Offsets in this section **include the report ID**, as the recorder prints
them: `r1` is report 1 (41 bytes), `r2` is report 2 (109 bytes). Multi-byte
fields are little-endian.

### Findings

- **Buttons, pads and display buttons:** two presses gave four clean edges at
  the mask both sources list. Pads are digital; no pressure bytes changed.
- **12-bit controls:** FX knobs, deck faders, gain, EQ, filter, channel faders
  and the crossfader each use two bytes with the upper nibble always clear.
  Sweeps ran from 8–22 at one end to 4074–4095 at the other, so full scale
  differs slightly per control.
- **Channel faders** are 12-bit at `r2 [33..40]`, not one byte each.
- **EQ LOW on channels C and D** is 12-bit at `r2 [95..96]` and
  `r2 [105..106]`, like every other mixer knob.
- **Screen encoders** hold a position from 0 to 999 that wraps, in two bytes
  each at `r2 [1..16]`. Clockwise counts up. They have no detents. One full
  turn is 1000 counts, so the value is the angle within one revolution; see
  [Screen encoder counts per turn](#screen-encoder-counts-per-turn).
- **Browse, loop and tempo encoders** step a 4-bit counter by one per click,
  up when turned clockwise, on all five encoders.
- **Touch caps** report in report 1 at the kontrol-s8-protocol locations:
  bytes 10 and 11 for the left deck, 18 and 19 for the right. The right deck
  fader 1 touch bit (`r1 [19] 0x01`) never changed in any run and is
  **Unverified**.
- **Touchstrips** report a position only: 0 when released and 1–1024 while
  touched, at `r1 [28..29]` (left) and `r1 [34..35]` (right). There is no
  separate touch bit. The bit kontrol-s8-protocol calls touch
  (`[29]` / `[35]` mask `0x01`) is bit 8 of the position.
- **Crossfader assign** is a three-position switch per channel in `r1 [24]`.
  The left position sets the end bit, the middle sets the middle bit and the
  right position clears both. Moves went end ↔ middle ↔ other end with each
  position held for a second or more. The left/middle/right naming was
  checked against the panel on channels A, B, C and D in the 2026-11-09
  re-check.

  | Channel | Left (end bit) | Middle bit | Right   |
  | ------- | -------------- | ---------- | ------- |
  | A       | `0x20`         | `0x10`     | neither |
  | B       | `0x01`         | `0x02`     | neither |
  | C       | `0x80`         | `0x40`     | neither |
  | D       | `0x04`         | `0x08`     | neither |

  A repeat of the channel B rows gave the same bits.

- **Latches:** the S8 keeps a toggle state for CUE (`r1 [39]` masks
  `0x01`/`0x02`/`0x04`/`0x08` for A/B/C/D), FILTER ON (`r1 [38]` masks
  `0x10`/`0x20`/`0x40`/`0x80`) and MIC 1 / MIC 2 (`r1 [39]` masks `0x10` /
  `0x20`). Each flips once per press.
- **Counters:** `r1 [26..27]` and `r1 [32..33]` are 16-bit counters that
  change on every report; the second is always the first minus one. Ignore
  them for control decoding.

### Screen encoder counts per turn

Re-check on 2026-11-09 after the capture, same read-only setup. A tape mark
on the knob counted whole turns; position deltas were summed modulo 1000.

| Encoder | Turns        | Summed counts | Counts per turn |
| ------- | ------------ | ------------- | --------------- |
| Left 1  | 10 slow, CW  | +9999         | 999.9           |
| Left 1  | 10 slow, CCW | −9985         | 998.5           |
| Right 1 | 10 slow, CW  | +10044        | 1004.4          |
| Left 1  | 10 fast, CW  | +10040        | 1004.0          |

One turn is 1000 counts, independent of speed and direction. An earlier fast
run that summed to +10982 was 11 turns, not 10. The other six screen encoders
were not counted turn by turn. The re-check logs were lost with the capture
logs; these totals were read from them before the loss.

The same re-check left the right deck fader 1 touch bit **Unverified**: no
report arrived on touching that fader in two sessions while fader 2 reported
normally. A dead touch cap on this unit is suspected, not shown.

### Decoder fixture (#12)

`tests/fixtures/s8-input-reports.jsonl` holds a reduced copy of the capture. It has one
line per checklist row plus 24 screen-encoder sub-steps: 215 lines and 1049 reports in
total. `tests/s8_input.rs` replays every line through the bridge's real decoder and
compares the decoded events. It also checks that every verified control in
[input-capture-rows.md](input-capture-rows.md) appears.

How each row was reduced:

- Each line starts with a base report per report ID. The first report of an
  ID only sets the resting state and produces no events.
- Buttons, pads, touch caps, switches and step encoders keep every report 1
  in which a digital field changed.
- Faders and knobs keep the reports at the lowest and highest value of the
  sweep.
- Touchstrips keep the base report, the first touched report, the highest
  position and the return to 0.
- Expected events are sorted within each report, so the test does not depend
  on the order the decoder emits fields of one report.

The right deck fader 1 touch bit is left out of the verified set.

### Live `--raw` check (#12)

Attended on 2026-11-10 with `NIHardwareAgent` stopped. There were two runs of
`cargo run --locked -- --device s8 --raw`, each stopped with Ctrl-C. The main run was at
06:21 UTC (86 seconds, 391 reports). A second run at 06:29 UTC covered the right BROWSE
encoder (29 seconds, 112 reports). Both logs are in
[captures/2026-11-09-input-capture](captures/2026-11-09-input-capture/README.md).

| Control moved                     | Decoded as                                                                                                    | Result  |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------- | ------- |
| Left PLAY, 2 presses              | `left_play` pressed / released, `r1 [9] 0x10`                                                                 | Correct |
| Channel A volume fader            | `mixer_fader_a`, 152 values from 0.002 to 0.996                                                               | Correct |
| Crossfader assign A, left         | `xfader_assign_left_a` on, `xfader_assign_middle_a` off                                                       | Correct |
| Crossfader assign A, right        | both off                                                                                                      | Correct |
| Crossfader assign A, middle       | `xfader_assign_middle_a` on                                                                                   | Correct |
| Channel A mixer CUE               | `mixer_cue_a` pressed / released, `mixer_cue_a_latch` on                                                      | Correct |
| Left display side button          | `left_screen_left_2` pressed / released, `r1 [4] 0x10`                                                        | Correct |
| Left screen encoder 1             | `left_screen_encoder_touch_1` and 132 `left_screen_encoder_1` turns                                           | Correct |
| Left touchstrip                   | `left_touchstrip` 0 to 1.0 and back, `left_touchstrip_touch`                                                  | Correct |
| Right LOOP encoder                | `right_loop_encoder_touch`, `right_loop_encoder` +1 × 6, `right_loop_encoder_press`                           | Correct |
| Right BROWSE encoder (second run) | `right_browse_encoder_touch`, `right_browse_encoder` +1 × 10 then −1 × 11, `right_browse_encoder_press` twice | Correct |
| Right pad                         | `right_pad_1` pressed / released, `r1 [16] 0x08`                                                              | Correct |

- The main run moved the right LOOP encoder where the list asked for BROWSE
  (`r1 [18] 0x01`, `r1 [2]` low nibble, `r1 [15] 0x10`). The second run
  moved BROWSE: touch at `r1 [18] 0x20`, turns in the `r1 [2]` high nibble,
  clockwise counting up, and push at `r1 [12] 0x08`. Both match
  [input-capture-rows.md](input-capture-rows.md).
- The first report of each ID is logged as events: the four crossfader assign
  switches resting in the middle, the CUE latches that were on and every
  fader and knob position appear once at the start. They are resting state,
  not presses. A Djay mapping must not treat them as presses (#13).
- The run was stopped with Ctrl-C through a pipe and the closing summary line
  was not captured. `led_writes=0` was seen in the earlier unattended run; the
  S8 descriptor has no output reports to write.
- Controls not in the table were not exercised live; the fixture test covers
  them.

### Results per control

All 191 checklist rows, with field, observed range and status, are in
[input-capture-rows.md](input-capture-rows.md).

### Corrections to the sources

These corrections are for a later Encdr PR and for the bridge's own S8 descriptor
corrections (#12). They apply to Encdr `encdr/descriptors/ni_kontrol_s8.json`. The same
entries exist at the bridge's pin `5d7a689`.

| Encdr entry                                     | Encdr says                             | Observed                                                                   |
| ----------------------------------------------- | -------------------------------------- | -------------------------------------------------------------------------- |
| `left_touchstrip_touch`                         | `r1 [16] 0x01`                         | That bit is right pad 2. The strip has no touch bit; use position ≠ 0      |
| `right_touchstrip_touch`                        | `r1 [32] 0x01`                         | Low bit of the counter at `r1 [32..33]`; flips on unrelated reports        |
| `left_touchstrip`, `right_touchstrip`           | 10 bits, max 1024                      | Reaches 1024, which needs 11 bits                                          |
| `xfader_assign_*_b`, `xfader_assign_*_d`        | B at `0x08`/`0x04`, D at `0x02`/`0x01` | Swapped: B is `0x02`/`0x01`, D is `0x08`/`0x04`                            |
| `xfader_assign_left_*`, `xfader_assign_right_*` | Two independent switches per channel   | One three-position switch; see the crossfader assign table in Findings     |
| Deck fader `on_touch`, `*_fx_dial_touch_*`      | Report 2, bytes 11 and 19              | Those bytes are position data. Touch is in report 1, bytes 10/11 and 18/19 |
| `*_screen_encoder_*`                            | `wrap16_wide` (wraps at 65536)         | Wraps at 1000                                                              |
| `mixer_eq_low_c`, `mixer_eq_low_d`              | 4 bits at `[96]` / `[106]`             | 12 bits at `[95..96]` / `[105..106]`                                       |
| Slider `max_value` 4078                         | One full scale                         | 4074–4095 depending on the control                                         |

kontrol-s8-protocol `data/HID_INPUT.json`:

| Topic                                                   | Dataset says                 | Observed                                |
| ------------------------------------------------------- | ---------------------------- | --------------------------------------- |
| Touchstrip touch                                        | `[29]` / `[35]` mask `0x01`  | Bit 8 of the position, not a touch flag |
| Mixer channel faders                                    | One byte each, range 0–15    | 12-bit at `r2 [33..40]`                 |
| EQ LOW C and D                                          | One byte at `[96]` / `[106]` | 12-bit at `[95..96]` / `[105..106]`     |
| FILTER ON, deck assign, crossfader assign, MIC, latches | Not listed                   | Present; see the rows file              |
