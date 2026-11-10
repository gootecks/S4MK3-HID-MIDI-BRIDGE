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

| Encoder | Turns         | Summed counts | Counts per turn |
| ------- | ------------- | ------------- | --------------- |
| Left 1  | 10 slow, CW   | +9999         | 999.9           |
| Left 1  | 10 slow, CCW  | −9985         | 998.5           |
| Right 1 | 10 slow, CW   | +10044        | 1004.4          |
| Left 1  | 10 fast, CW   | +10040        | 1004.0          |

One turn is 1000 counts, independent of speed and direction. An earlier fast
run that summed to +10982 was 11 turns, not 10. The other six screen encoders
were not counted turn by turn. The re-check logs were lost with the capture
logs; these totals were read from them before the loss.

The same re-check left the right deck fader 1 touch bit **Unverified**: no
report arrived on touching that fader in two sessions while fader 2 reported
normally. A dead touch cap on this unit is suspected, not shown.

### Decoder fixture (#12)

`tests/fixtures/s8-input-reports.jsonl` holds a reduced copy of the capture:
one line per checklist row plus 24 screen-encoder sub-steps, 215 lines and
1049 reports in total. `tests/s8_input.rs` replays every line through the
bridge's real decoder and compares the decoded events, and checks that every
verified control in the tables below appears.

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

Attended on 2026-11-10 with `NIHardwareAgent` stopped, in two runs of
`cargo run --locked -- --device s8 --raw`, each stopped with Ctrl-C: the
main run at 06:21 UTC (86 seconds, 391 reports) and a second run at 06:29 UTC
for the right BROWSE encoder (29 seconds, 112 reports). Both logs are in
[captures/2026-11-09-input-capture](captures/2026-11-09-input-capture/README.md).

| Control moved                     | Decoded as                                                        | Result   |
| --------------------------------- | ----------------------------------------------------------------- | -------- |
| Left PLAY, 2 presses              | `left_play` pressed / released, `r1 [9] 0x10`                     | Correct  |
| Channel A volume fader            | `mixer_fader_a`, 152 values from 0.002 to 0.996                   | Correct  |
| Crossfader assign A, left         | `xfader_assign_left_a` on, `xfader_assign_middle_a` off           | Correct  |
| Crossfader assign A, right        | both off                                                          | Correct  |
| Crossfader assign A, middle       | `xfader_assign_middle_a` on                                       | Correct  |
| Channel A mixer CUE               | `mixer_cue_a` pressed / released, `mixer_cue_a_latch` on          | Correct  |
| Left display side button          | `left_screen_left_2` pressed / released, `r1 [4] 0x10`            | Correct  |
| Left screen encoder 1             | `left_screen_encoder_touch_1` and 132 `left_screen_encoder_1` turns | Correct  |
| Left touchstrip                   | `left_touchstrip` 0 to 1.0 and back, `left_touchstrip_touch`      | Correct  |
| Right LOOP encoder                | `right_loop_encoder_touch`, `right_loop_encoder` +1 × 6, `right_loop_encoder_press` | Correct  |
| Right BROWSE encoder (second run) | `right_browse_encoder_touch`, `right_browse_encoder` +1 × 10 then −1 × 11, `right_browse_encoder_press` twice | Correct  |
| Right pad                         | `right_pad_1` pressed / released, `r1 [16] 0x08`                  | Correct  |

- The main run moved the right LOOP encoder where the list asked for BROWSE
  (`r1 [18] 0x01`, `r1 [2]` low nibble, `r1 [15] 0x10`). The second run
  moved BROWSE: touch at `r1 [18] 0x20`, turns in the `r1 [2]` high nibble,
  clockwise counting up, and push at `r1 [12] 0x08`. Both match the table
  below.
- The first report of each ID is logged as events: the four crossfader assign
  switches resting in the middle, the CUE latches that were on and every
  fader and knob position appear once at the start. They are resting state,
  not presses. A Djay mapping must not treat them as presses (#13).
- The run was stopped with Ctrl-C through a pipe and the closing summary line
  was not captured. `led_writes=0` was seen in the earlier unattended run; the
  S8 descriptor has no output reports to write.
- Controls not in the table were not exercised live; the fixture test covers
  them.

### Left deck

| #  | Step             | Control                                                                | Field                         | Observed                                                    | Status   |
| -- | ---------------- | ---------------------------------------------------------------------- | ----------------------------- | ----------------------------------------------------------- | -------- |
| 1  | `L-disp-1`       | Left display button 1 (NI colour settings in MIDI mode)                | `r1 [4] 0x20`                 | 2 presses                                                   | Verified |
| 2  | `L-disp-2`       | Left display button 2                                                  | `r1 [4] 0x10`                 | 2 presses                                                   | Verified |
| 3  | `L-disp-3`       | Left display button 3                                                  | `r1 [4] 0x01`                 | 2 presses                                                   | Verified |
| 4  | `L-disp-4`       | Left display button 4 (Controller Editor template switch in MIDI mode) | `r1 [6] 0x40`                 | 2 presses                                                   | Verified |
| 5  | `L-disp-5`       | Left display button 5                                                  | `r1 [5] 0x08`                 | 2 presses                                                   | Verified |
| 6  | `L-disp-6`       | Left display button 6                                                  | `r1 [5] 0x10`                 | 2 presses                                                   | Verified |
| 7  | `L-disp-7`       | Left display button 7                                                  | `r1 [5] 0x20`                 | 2 presses                                                   | Verified |
| 8  | `L-disp-8`       | Left display button 8                                                  | `r1 [5] 0x40`                 | 2 presses                                                   | Verified |
| 9  | `L-senc-1-touch` | Left screen encoder 1 touch                                            | `r1 [10] 0x02`                | touch and release                                           | Verified |
| 10 | `L-senc-1-turn`  | Left screen encoder 1 (wrap16_wide)                                    | `r2 [1..2]`                   | 0–996, wraps past 999; clockwise counts up                  | Verified |
| 11 | `L-senc-2-touch` | Left screen encoder 2 touch                                            | `r1 [10] 0x04`                | touch and release                                           | Verified |
| 12 | `L-senc-2-turn`  | Left screen encoder 2 (wrap16_wide)                                    | `r2 [3..4]`                   | 0–997, wraps past 999; clockwise counts up                  | Verified |
| 13 | `L-senc-3-touch` | Left screen encoder 3 touch                                            | `r1 [10] 0x08`                | touch and release                                           | Verified |
| 14 | `L-senc-3-turn`  | Left screen encoder 3 (wrap16_wide)                                    | `r2 [5..6]`                   | 0–999, wraps past 999; clockwise counts up                  | Verified |
| 15 | `L-senc-4-touch` | Left screen encoder 4 touch                                            | `r1 [10] 0x10`                | touch and release                                           | Verified |
| 16 | `L-senc-4-turn`  | Left screen encoder 4 (wrap16_wide)                                    | `r2 [7..8]`                   | 0–995, wraps past 999; clockwise counts up                  | Verified |
| 17 | `L-browse-touch` | Left browse encoder touch                                              | `r1 [10] 0x20`                | touch and release                                           | Verified |
| 18 | `L-browse-push`  | Left browse encoder push                                               | `r1 [4] 0x08`                 | 2 presses                                                   | Verified |
| 19 | `L-browse-turn`  | Left browse encoder                                                    | `r1 [1] 0xf0`                 | clockwise +3, anticlockwise −3, mod 16                      | Verified |
| 20 | `L-loop-touch`   | Left loop encoder touch                                                | `r1 [10] 0x01`                | touch and release                                           | Verified |
| 21 | `L-loop-push`    | Left loop encoder push                                                 | `r1 [7] 0x10`                 | 2 presses                                                   | Verified |
| 22 | `L-loop-turn`    | Left loop encoder                                                      | `r1 [1] 0x0f`                 | clockwise +3, anticlockwise −3, mod 16                      | Verified |
| 23 | `L-back`         | Left BACK                                                              | `r1 [6] 0x80`                 | 2 presses                                                   | Verified |
| 24 | `L-edit`         | Left EDIT                                                              | `r1 [6] 0x01`                 | 2 presses                                                   | Verified |
| 25 | `L-capture`      | Left CAPTURE                                                           | `r1 [6] 0x20`                 | 2 presses                                                   | Verified |
| 26 | `L-fxselect`     | Left FX SELECT                                                         | `r1 [4] 0x40`                 | 2 presses                                                   | Verified |
| 27 | `L-fx1`          | Left FX 1 button                                                       | `r1 [4] 0x80`                 | 2 presses                                                   | Verified |
| 28 | `L-fx2`          | Left FX 2 button                                                       | `r1 [5] 0x04`                 | 2 presses                                                   | Verified |
| 29 | `L-fx3`          | Left FX 3 button                                                       | `r1 [5] 0x02`                 | 2 presses                                                   | Verified |
| 30 | `L-fx4`          | Left FX 4 button                                                       | `r1 [5] 0x01`                 | 2 presses                                                   | Verified |
| 31 | `L-on1`          | Left ON 1                                                              | `r1 [6] 0x10`                 | 2 presses                                                   | Verified |
| 32 | `L-on2`          | Left ON 2                                                              | `r1 [6] 0x08`                 | 2 presses                                                   | Verified |
| 33 | `L-on3`          | Left ON 3                                                              | `r1 [6] 0x04`                 | 2 presses                                                   | Verified |
| 34 | `L-on4`          | Left ON 4                                                              | `r1 [6] 0x02`                 | 2 presses                                                   | Verified |
| 35 | `L-deck`         | Left DECK                                                              | `r1 [8] 0x04`                 | 2 presses                                                   | Verified |
| 36 | `L-flux`         | Left FLUX                                                              | `r1 [8] 0x10`                 | 2 presses                                                   | Verified |
| 37 | `L-shift`        | Left SHIFT                                                             | `r1 [9] 0x80`                 | 2 presses                                                   | Verified |
| 38 | `L-sync`         | Left SYNC                                                              | `r1 [9] 0x40`                 | 2 presses                                                   | Verified |
| 39 | `L-cue`          | Left CUE                                                               | `r1 [9] 0x20`                 | 2 presses                                                   | Verified |
| 40 | `L-play`         | Left PLAY                                                              | `r1 [9] 0x10`                 | 2 presses                                                   | Verified |
| 41 | `L-hotcue`       | Left HOTCUE                                                            | `r1 [9] 0x08`                 | 2 presses                                                   | Verified |
| 42 | `L-loop`         | Left LOOP                                                              | `r1 [9] 0x04`                 | 2 presses                                                   | Verified |
| 43 | `L-freeze`       | Left FREEZE/SLICE                                                      | `r1 [9] 0x02`                 | 2 presses                                                   | Verified |
| 44 | `L-remix`        | Left REMIX                                                             | `r1 [9] 0x01`                 | 2 presses                                                   | Verified |
| 45 | `L-pad-1`        | Left pad 1                                                             | `r1 [8] 0x08`                 | 2 presses                                                   | Verified |
| 46 | `L-pad-2`        | Left pad 2                                                             | `r1 [8] 0x01`                 | 2 presses                                                   | Verified |
| 47 | `L-pad-3`        | Left pad 3                                                             | `r1 [7] 0x01`                 | 2 presses                                                   | Verified |
| 48 | `L-pad-4`        | Left pad 4                                                             | `r1 [7] 0x02`                 | 2 presses                                                   | Verified |
| 49 | `L-pad-5`        | Left pad 5                                                             | `r1 [8] 0x20`                 | 2 presses                                                   | Verified |
| 50 | `L-pad-6`        | Left pad 6                                                             | `r1 [8] 0x40`                 | 2 presses                                                   | Verified |
| 51 | `L-pad-7`        | Left pad 7                                                             | `r1 [8] 0x80`                 | 2 presses                                                   | Verified |
| 52 | `L-pad-8`        | Left pad 8                                                             | `r1 [8] 0x02`                 | 2 presses                                                   | Verified |
| 53 | `L-fxk-1-touch`  | Left FX knob 1 touch                                                   | `r1 [10] 0x40`                | touch and release                                           | Verified |
| 54 | `L-fxk-1`        | Left FX knob 1                                                         | `r2 [43..44]`                 | 11–4088                                                     | Verified |
| 55 | `L-fxk-2-touch`  | Left FX knob 2 touch                                                   | `r1 [10] 0x80`                | touch and release                                           | Verified |
| 56 | `L-fxk-2`        | Left FX knob 2                                                         | `r2 [45..46]`                 | 12–4084                                                     | Verified |
| 57 | `L-fxk-3-touch`  | Left FX knob 3 touch                                                   | `r1 [11] 0x10`                | touch and release                                           | Verified |
| 58 | `L-fxk-3`        | Left FX knob 3                                                         | `r2 [47..48]`                 | 15–4088                                                     | Verified |
| 59 | `L-fxk-4-touch`  | Left FX knob 4 touch                                                   | `r1 [11] 0x20`                | touch and release                                           | Verified |
| 60 | `L-fxk-4`        | Left FX knob 4                                                         | `r2 [49..50]`                 | 17–4088                                                     | Verified |
| 61 | `L-fader-1`      | Left deck fader 1 (touch + position)                                   | `r2 [17..18]`, `r1 [11] 0x01` | 12–4088; touch and release                                  | Verified |
| 62 | `L-fader-2`      | Left deck fader 2 (touch + position)                                   | `r2 [19..20]`, `r1 [11] 0x02` | 13–4087; touch release only (touched before the step began) | Verified |
| 63 | `L-fader-3`      | Left deck fader 3 (touch + position)                                   | `r2 [21..22]`, `r1 [11] 0x04` | 16–4088; touch and release                                  | Verified |
| 64 | `L-fader-4`      | Left deck fader 4 (touch + position)                                   | `r2 [23..24]`, `r1 [11] 0x08` | 13–4088; touch and release                                  | Verified |
| 65 | `L-strip-touch`  | Left touchstrip touch                                                  | `r1 [28..29]`                 | 495–509 while touched, 0 on release; no separate touch bit  | Verified |
| 66 | `L-strip-slide`  | Left touchstrip position                                               | `r1 [28..29]`                 | 1–1024 while touched, 0 on release                          | Verified |

### Mixer

| #   | Step             | Control                                                | Field                  | Observed                                        | Status   |
| --- | ---------------- | ------------------------------------------------------ | ---------------------- | ----------------------------------------------- | -------- |
| 67  | `M-A-gain`       | Channel A GAIN                                         | `r2 [69..70]`          | 14–4081                                         | Verified |
| 68  | `M-A-eqhi`       | Channel A EQ HI                                        | `r2 [71..72]`          | 16–4081                                         | Verified |
| 69  | `M-A-eqmid`      | Channel A EQ MID                                       | `r2 [73..74]`          | 13–4080                                         | Verified |
| 70  | `M-A-eqlow`      | Channel A EQ LOW                                       | `r2 [75..76]`          | 10–4077                                         | Verified |
| 71  | `M-A-filter`     | Channel A FILTER                                       | `r2 [77..78]`          | 9–4079                                          | Verified |
| 72  | `M-A-filteron`   | Channel A FILTER ON                                    | `r1 [20] 0x02`         | 2 presses; latch `r1 [38] 0x10` flips per press | Verified |
| 73  | `M-A-fx1`        | Channel A FX assign 1                                  | `r1 [22] 0x20`         | 2 presses                                       | Verified |
| 74  | `M-A-fx2`        | Channel A FX assign 2                                  | `r1 [22] 0x40`         | 2 presses                                       | Verified |
| 75  | `M-A-deckassign` | Channel A deck assign (left or right deck pair button) | `r1 [22] 0x02`         | 2 presses                                       | Verified |
| 76  | `M-A-cue`        | Channel A CUE (PFL)                                    | `r1 [20] 0x08`         | 1 press; latch `r1 [39] 0x01` flips per press   | Verified |
| 77  | `M-A-xf-left`    | Channel A crossfader assign LEFT                       | `r1 [24] 0x20`, `0x10` | one end sets `0x20`, the middle sets `0x10`     | Verified |
| 78  | `M-A-xf-right`   | Channel A crossfader assign RIGHT                      | `r1 [24]`, no bit      | other end clears both `0x20` and `0x10`         | Verified |
| 79  | `M-A-fader`      | Channel A fader                                        | `r2 [33..34]`          | 13–4084                                         | Verified |
| 80  | `M-B-gain`       | Channel B GAIN                                         | `r2 [79..80]`          | 19–4092                                         | Verified |
| 81  | `M-B-eqhi`       | Channel B EQ HI                                        | `r2 [81..82]`          | 22–4086                                         | Verified |
| 82  | `M-B-eqmid`      | Channel B EQ MID                                       | `r2 [83..84]`          | 19–4091                                         | Verified |
| 83  | `M-B-eqlow`      | Channel B EQ LOW                                       | `r2 [85..86]`          | 20–4089                                         | Verified |
| 84  | `M-B-filter`     | Channel B FILTER                                       | `r2 [87..88]`          | 18–4089                                         | Verified |
| 85  | `M-B-filteron`   | Channel B FILTER ON                                    | `r1 [23] 0x08`         | 2 presses; latch `r1 [38] 0x20` flips per press | Verified |
| 86  | `M-B-fx1`        | Channel B FX assign 1                                  | `r1 [25] 0x10`         | 2 presses                                       | Verified |
| 87  | `M-B-fx2`        | Channel B FX assign 2                                  | `r1 [25] 0x20`         | 2 presses                                       | Verified |
| 88  | `M-B-deckassign` | Channel B deck assign (left or right deck pair button) | `r1 [25] 0x02`         | 2 presses                                       | Verified |
| 89  | `M-B-cue`        | Channel B CUE (PFL)                                    | `r1 [23] 0x20`         | 1 press; latch `r1 [39] 0x02` flips per press   | Verified |
| 90  | `M-B-xf-left`    | Channel B crossfader assign LEFT                       | `r1 [24] 0x01`, `0x02` | one end sets `0x01`, the middle sets `0x02`     | Verified |
| 91  | `M-B-xf-right`   | Channel B crossfader assign RIGHT                      | `r1 [24]`, no bit      | other end clears both `0x01` and `0x02`         | Verified |
| 92  | `M-B-fader`      | Channel B fader                                        | `r2 [35..36]`          | 22–4094                                         | Verified |
| 93  | `M-C-gain`       | Channel C GAIN                                         | `r2 [89..90]`          | 13–4076                                         | Verified |
| 94  | `M-C-eqhi`       | Channel C EQ HI                                        | `r2 [91..92]`          | 10–4082                                         | Verified |
| 95  | `M-C-eqmid`      | Channel C EQ MID                                       | `r2 [93..94]`          | 12–4077                                         | Verified |
| 96  | `M-C-eqlow`      | Channel C EQ LOW                                       | `r2 [95..96]`          | 13–4078                                         | Verified |
| 97  | `M-C-filter`     | Channel C FILTER                                       | `r2 [97..98]`          | 9–4080                                          | Verified |
| 98  | `M-C-filteron`   | Channel C FILTER ON                                    | `r1 [20] 0x01`         | 2 presses; latch `r1 [38] 0x40` flips per press | Verified |
| 99  | `M-C-fx1`        | Channel C FX assign 1                                  | `r1 [22] 0x08`         | 2 presses                                       | Verified |
| 100 | `M-C-fx2`        | Channel C FX assign 2                                  | `r1 [22] 0x10`         | 2 presses                                       | Verified |
| 101 | `M-C-deckassign` | Channel C deck assign (left or right deck pair button) | `r1 [22] 0x01`         | 2 presses                                       | Verified |
| 102 | `M-C-cue`        | Channel C CUE (PFL)                                    | `r1 [20] 0x04`         | 1 press; latch `r1 [39] 0x04` flips per press   | Verified |
| 103 | `M-C-xf-left`    | Channel C crossfader assign LEFT                       | `r1 [24] 0x80`, `0x40` | one end sets `0x80`, the middle sets `0x40`     | Verified |
| 104 | `M-C-xf-right`   | Channel C crossfader assign RIGHT                      | `r1 [24]`, no bit      | other end clears both `0x80` and `0x40`         | Verified |
| 105 | `M-C-fader`      | Channel C fader                                        | `r2 [37..38]`          | 11–4083                                         | Verified |
| 106 | `M-D-gain`       | Channel D GAIN                                         | `r2 [99..100]`         | 20–4090                                         | Verified |
| 107 | `M-D-eqhi`       | Channel D EQ HI                                        | `r2 [101..102]`        | 18–4086                                         | Verified |
| 108 | `M-D-eqmid`      | Channel D EQ MID                                       | `r2 [103..104]`        | 20–4088                                         | Verified |
| 109 | `M-D-eqlow`      | Channel D EQ LOW                                       | `r2 [105..106]`        | 19–4085                                         | Verified |
| 110 | `M-D-filter`     | Channel D FILTER                                       | `r2 [107..108]`        | 19–4091                                         | Verified |
| 111 | `M-D-filteron`   | Channel D FILTER ON                                    | `r1 [23] 0x10`         | 2 presses; latch `r1 [38] 0x80` flips per press | Verified |
| 112 | `M-D-fx1`        | Channel D FX assign 1                                  | `r1 [25] 0x40`         | 2 presses                                       | Verified |
| 113 | `M-D-fx2`        | Channel D FX assign 2                                  | `r1 [25] 0x80`         | 2 presses                                       | Verified |
| 114 | `M-D-deckassign` | Channel D deck assign (left or right deck pair button) | `r1 [25] 0x04`         | 2 presses                                       | Verified |
| 115 | `M-D-cue`        | Channel D CUE (PFL)                                    | `r1 [23] 0x40`         | 1 press; latch `r1 [39] 0x08` flips per press   | Verified |
| 116 | `M-D-xf-left`    | Channel D crossfader assign LEFT                       | `r1 [24] 0x04`, `0x08` | one end sets `0x04`, the middle sets `0x08`     | Verified |
| 117 | `M-D-xf-right`   | Channel D crossfader assign RIGHT                      | `r1 [24]`, no bit      | other end clears both `0x04` and `0x08`         | Verified |
| 118 | `M-D-fader`      | Channel D fader                                        | `r2 [39..40]`          | 19–4095                                         | Verified |
| 119 | `M-xfader`       | Crossfader                                             | `r2 [41..42]`          | 22–4090                                         | Verified |
| 120 | `M-snap`         | SNAP                                                   | `r1 [22] 0x80`         | 2 presses                                       | Verified |
| 121 | `M-quantize`     | QUANTIZE                                               | `r1 [25] 0x08`         | 2 presses                                       | Verified |
| 122 | `M-tempo-push`   | TEMPO encoder push                                     | `r1 [23] 0x04`         | 2 presses                                       | Verified |
| 123 | `M-tempo-turn`   | TEMPO encoder                                          | `r1 [3] 0x0f`          | clockwise +3, anticlockwise −3, mod 16          | Verified |
| 124 | `M-mic1`         | MIC 1 button                                           | `r1 [22] 0x04`         | 2 presses; latch `r1 [39] 0x10` flips per press | Verified |
| 125 | `M-mic2`         | MIC 2 button                                           | `r1 [25] 0x01`         | 2 presses; latch `r1 [39] 0x20` flips per press | Verified |
|     | `M-cue-vol`      | CUE VOL (2026-11-10, separate raw capture)             | `r2 [65..66]`          | 19–4091                                         | Verified by order moved |
|     | `M-cue-mix`      | CUE MIX (2026-11-10, separate raw capture)             | `r2 [63..64]`          | 20–4090                                         | Verified by order moved |
|     | `M-booth`        | BOOTH (2026-11-10, separate raw capture)               | `r2 [61..62]`          | 11–4082                                         | Verified by order moved |
|     | `M-main`         | MAIN (2026-11-10, separate raw capture)                | `r2 [59..60]`          | 13–4082                                         | Verified, only field sweeping |

### Right deck

| #   | Step             | Control                                                                 | Field                                 | Observed                                                   | Status                              |
| --- | ---------------- | ----------------------------------------------------------------------- | ------------------------------------- | ---------------------------------------------------------- | ----------------------------------- |
| 126 | `R-disp-1`       | Right display button 1 (NI colour settings in MIDI mode)                | `r1 [12] 0x20`                        | 2 presses                                                  | Verified                            |
| 127 | `R-disp-2`       | Right display button 2                                                  | `r1 [12] 0x10`                        | 2 presses                                                  | Verified                            |
| 128 | `R-disp-3`       | Right display button 3                                                  | `r1 [12] 0x01`                        | 2 presses                                                  | Verified                            |
| 129 | `R-disp-4`       | Right display button 4 (Controller Editor template switch in MIDI mode) | `r1 [14] 0x40`                        | 2 presses                                                  | Verified                            |
| 130 | `R-disp-5`       | Right display button 5                                                  | `r1 [13] 0x08`                        | 2 presses                                                  | Verified                            |
| 131 | `R-disp-6`       | Right display button 6                                                  | `r1 [13] 0x10`                        | 2 presses                                                  | Verified                            |
| 132 | `R-disp-7`       | Right display button 7                                                  | `r1 [13] 0x20`                        | 2 presses                                                  | Verified                            |
| 133 | `R-disp-8`       | Right display button 8                                                  | `r1 [13] 0x40`                        | 2 presses                                                  | Verified                            |
| 134 | `R-senc-1-touch` | Right screen encoder 1 touch                                            | `r1 [18] 0x02`                        | touch and release                                          | Verified                            |
| 135 | `R-senc-1-turn`  | Right screen encoder 1 (wrap16_wide)                                    | `r2 [9..10]`                          | 1–997, wraps past 999; clockwise counts up                 | Verified                            |
| 136 | `R-senc-2-touch` | Right screen encoder 2 touch                                            | `r1 [18] 0x04`                        | touch and release                                          | Verified                            |
| 137 | `R-senc-2-turn`  | Right screen encoder 2 (wrap16_wide)                                    | `r2 [11..12]`                         | 1–994, wraps past 999; clockwise counts up                 | Verified                            |
| 138 | `R-senc-3-touch` | Right screen encoder 3 touch                                            | `r1 [18] 0x08`                        | touch and release                                          | Verified                            |
| 139 | `R-senc-3-turn`  | Right screen encoder 3 (wrap16_wide)                                    | `r2 [13..14]`                         | 2–998, wraps past 999; clockwise counts up                 | Verified                            |
| 140 | `R-senc-4-touch` | Right screen encoder 4 touch                                            | `r1 [18] 0x10`                        | touch and release                                          | Verified                            |
| 141 | `R-senc-4-turn`  | Right screen encoder 4 (wrap16_wide)                                    | `r2 [15..16]`                         | 0–999, wraps past 999; clockwise counts up                 | Verified                            |
| 142 | `R-browse-touch` | Right browse encoder touch                                              | `r1 [18] 0x20`                        | 2 touches                                                  | Verified                            |
| 143 | `R-browse-push`  | Right browse encoder push                                               | `r1 [12] 0x08`                        | 2 presses                                                  | Verified                            |
| 144 | `R-browse-turn`  | Right browse encoder                                                    | `r1 [2] 0xf0`                         | clockwise +3, anticlockwise −3, mod 16                     | Verified                            |
| 145 | `R-loop-touch`   | Right loop encoder touch                                                | `r1 [18] 0x01`                        | touch and release                                          | Verified                            |
| 146 | `R-loop-push`    | Right loop encoder push                                                 | `r1 [15] 0x10`                        | 2 presses                                                  | Verified                            |
| 147 | `R-loop-turn`    | Right loop encoder                                                      | `r1 [2] 0x0f`                         | clockwise +3, anticlockwise −3, mod 16                     | Verified                            |
| 148 | `R-back`         | Right BACK                                                              | `r1 [14] 0x80`                        | 2 presses                                                  | Verified                            |
| 149 | `R-edit`         | Right EDIT                                                              | `r1 [14] 0x01`                        | 2 presses                                                  | Verified                            |
| 150 | `R-capture`      | Right CAPTURE                                                           | `r1 [14] 0x20`                        | 2 presses                                                  | Verified                            |
| 151 | `R-fxselect`     | Right FX SELECT                                                         | `r1 [12] 0x40`                        | 2 presses                                                  | Verified                            |
| 152 | `R-fx1`          | Right FX 1 button                                                       | `r1 [12] 0x80`                        | 2 presses                                                  | Verified                            |
| 153 | `R-fx2`          | Right FX 2 button                                                       | `r1 [13] 0x04`                        | 2 presses                                                  | Verified                            |
| 154 | `R-fx3`          | Right FX 3 button                                                       | `r1 [13] 0x02`                        | 2 presses                                                  | Verified                            |
| 155 | `R-fx4`          | Right FX 4 button                                                       | `r1 [13] 0x01`                        | 2 presses                                                  | Verified                            |
| 156 | `R-on1`          | Right ON 1                                                              | `r1 [14] 0x10`                        | 2 presses                                                  | Verified                            |
| 157 | `R-on2`          | Right ON 2                                                              | `r1 [14] 0x08`                        | 2 presses                                                  | Verified                            |
| 158 | `R-on3`          | Right ON 3                                                              | `r1 [14] 0x04`                        | 2 presses                                                  | Verified                            |
| 159 | `R-on4`          | Right ON 4                                                              | `r1 [14] 0x02`                        | 2 presses                                                  | Verified                            |
| 160 | `R-deck`         | Right DECK                                                              | `r1 [16] 0x04`                        | 2 presses                                                  | Verified                            |
| 161 | `R-flux`         | Right FLUX                                                              | `r1 [16] 0x10`                        | 2 presses                                                  | Verified                            |
| 162 | `R-shift`        | Right SHIFT                                                             | `r1 [17] 0x80`                        | 2 presses                                                  | Verified                            |
| 163 | `R-sync`         | Right SYNC                                                              | `r1 [17] 0x40`                        | 2 presses                                                  | Verified                            |
| 164 | `R-cue`          | Right CUE                                                               | `r1 [17] 0x20`                        | 2 presses                                                  | Verified                            |
| 165 | `R-play`         | Right PLAY                                                              | `r1 [17] 0x10`                        | 2 presses                                                  | Verified                            |
| 166 | `R-hotcue`       | Right HOTCUE                                                            | `r1 [17] 0x08`                        | 2 presses                                                  | Verified                            |
| 167 | `R-loop`         | Right LOOP                                                              | `r1 [17] 0x04`                        | 2 presses                                                  | Verified                            |
| 168 | `R-freeze`       | Right FREEZE/SLICE                                                      | `r1 [17] 0x02`                        | 2 presses                                                  | Verified                            |
| 169 | `R-remix`        | Right REMIX                                                             | `r1 [17] 0x01`                        | 2 presses                                                  | Verified                            |
| 170 | `R-pad-1`        | Right pad 1                                                             | `r1 [16] 0x08`                        | 2 presses                                                  | Verified                            |
| 171 | `R-pad-2`        | Right pad 2                                                             | `r1 [16] 0x01`                        | 2 presses                                                  | Verified                            |
| 172 | `R-pad-3`        | Right pad 3                                                             | `r1 [15] 0x01`                        | 2 presses                                                  | Verified                            |
| 173 | `R-pad-4`        | Right pad 4                                                             | `r1 [15] 0x02`                        | 2 presses                                                  | Verified                            |
| 174 | `R-pad-5`        | Right pad 5                                                             | `r1 [16] 0x20`                        | 2 presses                                                  | Verified                            |
| 175 | `R-pad-6`        | Right pad 6                                                             | `r1 [16] 0x40`                        | 2 presses                                                  | Verified                            |
| 176 | `R-pad-7`        | Right pad 7                                                             | `r1 [16] 0x80`                        | 2 presses                                                  | Verified                            |
| 177 | `R-pad-8`        | Right pad 8                                                             | `r1 [16] 0x02`                        | 2 presses                                                  | Verified                            |
| 178 | `R-fxk-1-touch`  | Right FX knob 1 touch                                                   | `r1 [18] 0x40`                        | touch and release                                          | Verified                            |
| 179 | `R-fxk-1`        | Right FX knob 1                                                         | `r2 [51..52]`                         | 10–4077                                                    | Verified                            |
| 180 | `R-fxk-2-touch`  | Right FX knob 2 touch                                                   | `r1 [18] 0x80`                        | touch and release                                          | Verified                            |
| 181 | `R-fxk-2`        | Right FX knob 2                                                         | `r2 [53..54]`                         | 9–4074                                                     | Verified                            |
| 182 | `R-fxk-3-touch`  | Right FX knob 3 touch                                                   | `r1 [19] 0x10`                        | touch and release                                          | Verified                            |
| 183 | `R-fxk-3`        | Right FX knob 3                                                         | `r2 [55..56]`                         | 10–4077                                                    | Verified                            |
| 184 | `R-fxk-4-touch`  | Right FX knob 4 touch                                                   | `r1 [19] 0x20`                        | touch and release                                          | Verified                            |
| 185 | `R-fxk-4`        | Right FX knob 4                                                         | `r2 [57..58]`                         | 8–4076                                                     | Verified                            |
| 186 | `R-fader-1`      | Right deck fader 1 (touch + position)                                   | `r2 [25..26]`, `r1 [19] 0x01` (touch) | 10–4076; touch bit never changed                           | Position verified; touch unverified |
| 187 | `R-fader-2`      | Right deck fader 2 (touch + position)                                   | `r2 [27..28]`, `r1 [19] 0x02`         | 9–4077; touch and release                                  | Verified                            |
| 188 | `R-fader-3`      | Right deck fader 3 (touch + position)                                   | `r2 [29..30]`, `r1 [19] 0x04`         | 9–4077; touch and release                                  | Verified                            |
| 189 | `R-fader-4`      | Right deck fader 4 (touch + position)                                   | `r2 [31..32]`, `r1 [19] 0x08`         | 8–4077; touch and release                                  | Verified                            |
| 190 | `R-strip-touch`  | Right touchstrip touch                                                  | `r1 [34..35]`                         | 485–495 while touched, 0 on release; no separate touch bit | Verified                            |
| 191 | `R-strip-slide`  | Right touchstrip position                                               | `r1 [34..35]`                         | 1–1024 while touched, 0 on release                         | Verified                            |

### Corrections to the sources

Encdr `encdr/descriptors/ni_kontrol_s8.json` (the same entries exist at the
bridge's pin `5d7a689`), for a later Encdr PR and for the bridge's own S8
descriptor corrections (#12):

| Encdr entry | Encdr says | Observed |
| ----------- | ---------- | -------- |
| `left_touchstrip_touch` | `r1 [16] 0x01` | That bit is right pad 2. The strip has no touch bit; use position ≠ 0 |
| `right_touchstrip_touch` | `r1 [32] 0x01` | Low bit of the counter at `r1 [32..33]`; flips on unrelated reports |
| `left_touchstrip`, `right_touchstrip` | 10 bits, max 1024 | Reaches 1024, which needs 11 bits |
| `xfader_assign_*_b`, `xfader_assign_*_d` | B at `0x08`/`0x04`, D at `0x02`/`0x01` | Swapped: B is `0x02`/`0x01`, D is `0x08`/`0x04` |
| `xfader_assign_left_*`, `xfader_assign_right_*` | Two independent switches per channel | One three-position switch; see the table above for end and middle bits |
| Deck fader `on_touch`, `*_fx_dial_touch_*` | Report 2, bytes 11 and 19 | Those bytes are position data. Touch is in report 1, bytes 10/11 and 18/19 |
| `*_screen_encoder_*` | `wrap16_wide` (wraps at 65536) | Wraps at 1000 |
| `mixer_eq_low_c`, `mixer_eq_low_d` | 4 bits at `[96]` / `[106]` | 12 bits at `[95..96]` / `[105..106]` |
| Slider `max_value` 4078 | One full scale | 4074–4095 depending on the control |

kontrol-s8-protocol `data/HID_INPUT.json`:

| Topic | Dataset says | Observed |
| ----- | ------------ | -------- |
| Touchstrip touch | `[29]` / `[35]` mask `0x01` | Bit 8 of the position, not a touch flag |
| Mixer channel faders | One byte each, range 0–15 | 12-bit at `r2 [33..40]` |
| EQ LOW C and D | One byte at `[96]` / `[106]` | 12-bit at `[95..96]` / `[105..106]` |
| FILTER ON, deck assign, crossfader assign, MIC, latches | Not listed | Present as tabled above |
