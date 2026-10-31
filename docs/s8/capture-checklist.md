# S8 agent-stopped input capture: recorder and checklist

Prerequisite capture for #10 (input), and for #12-#15. This is the plan for the
attended pass with `NIHardwareAgent` stopped; results go into
[input-capture.md](input-capture.md). The MIDI-mode pass already verified the
four CUE buttons, PLAY and the 16 display buttons.

## Recorder

`examples/s8-input-diff` opens only the S8 control HID interface (5) in macOS
shared mode and reads input reports. It sends nothing: no output or feature
reports, no control transfers, no bulk or screen traffic, and it never opens
interface 4 or 6. It does not stop or start `NIHardwareAgent`.

Each report is diffed against the previous report with the same ID. For every
changed field it prints one line: time, report ID, byte offset (with report ID,
as in the datasets), mask or byte range, `old->new`, and field names. For
rotary encoders it adds the wrapped delta (`+1 mod 16`).

```text
    1.000s r1 [9] mask 0x10 0->1 E:left_play | P:BtnPlayButtonLeft
    2.600s r1 [1] mask 0x0f 0->1 (+1 mod 16) P:EncLoopKnobLeft
    3.200s r2 [17..18] mask 0x0fff 2050->2624 (+574) E:left_fader_1 | P:FadFader1Left
    3.300s r2 [1..2] 0->65535 (-1 mod 65536) E:left_screen_encoder_1 | P:EndKnob1Left
    2.700s r1 [36] mask 0x80 0x00->0x80 ?
```

Names come from two sources:

- `P:` kontrol-s8-protocol `data/HID_INPUT.json`
  ([gusgustavodj/kontrol-s8-protocol](https://github.com/gusgustavodj/kontrol-s8-protocol),
  CC BY 4.0).
- `E:` Encdr `encdr/descriptors/ni_kontrol_s8.json`.

`?` means a changed bit that neither source describes. Fields that both
sources place at the same bytes print on one line; where they disagree (mask,
width or report) each prints its own line, which is how the discrepancy list
below gets resolved. The 4 heartbeat bytes at `r1 [26..27]` and `[32..33]`
(changing in every report per [input-capture.md](input-capture.md)) are
hidden unless `--show-heartbeat` is given.

Options:

| Option                  | Meaning |
| ----------------------- | ------- |
| `--log <path>`          | Raw JSONL log. Default `/tmp/s8-input-<epoch>.jsonl`; never inside the repo. |
| `--deadband <n>`        | Suppress analog changes of `n` counts or fewer from the last printed value (default 3). Touch bits, buttons and encoders are never suppressed. |
| `--step <label>`        | Label active from the start. Type further labels on stdin. |
| `--seconds <n>`         | Stop after `n` seconds (default: Ctrl-C). |
| `--show-heartbeat`      | Print the heartbeat bytes too. |
| `--list`                | List S8 HID entries and exit; opens nothing. |
| `--replay <log.jsonl>`  | Decode an earlier log offline; no device access. |
| `--protocol`, `--encdr` | Dataset paths (defaults: sibling checkouts `../kontrol-s8-protocol`, `../Encdr`, relative to the working directory). |

The JSONL log holds one record per line: `hid` (every report: `t`, `wall_ms`,
`step`, `id`, `len`, `hex`), `change` (the printed line) and `step` markers.

### Build and run

From the worktree root (the defaults assume the sibling checkouts):

```sh
cargo build --release --example s8-input-diff
./target/release/examples/s8-input-diff --log /tmp/s8-capture-$(date +%Y%m%d-%H%M).jsonl
```

Type a step label and press Enter **before** each control, for example `L-play`,
then perform the motion. Labels are stamped into the log and printed as
`=== step: <label> ===`. Use the step labels in the tables below so the log can
be grouped later. Stop with Ctrl-C. Offline check of any log:

```sh
./target/release/examples/s8-input-diff --replay /tmp/s8-capture-....jsonl --log /tmp/replay.jsonl
```

## Session flow

1. Integrator confirms the S8 is connected in the mode under test and starts
   the recorder; the first line per report is `baseline`. The S8 sends HID
   reports only when a control changes, so nothing prints until the first
   motion.
2. Integrator stops `NIHardwareAgent` (as in
   [agent-stop-validation.md](agent-stop-validation.md)) and restores it
   afterwards. The recorder does neither.
3. Work left deck, then mixer, then right deck, top to bottom. One step label
   per row. Do each motion deliberately, with a 1 s pause before the next row.
4. Per row, write the result in the `Result` column or in
   [input-capture.md](input-capture.md): report, byte, mask or range, direction,
   and **Verified** or **Unverified**.

Motions: "press and release, twice" gives two distinct edges. "3 clicks CW then
3 clicks CCW" shows the per-click delta and direction. "Full sweep" goes from
physical minimum to maximum and back so range and monotonic direction are both
visible; hold at each end for 1 s. Touch caps: touch with a fingertip without
moving the control.

## Discrepancies to settle

The two sources disagree. The session decides which is right on this firmware.

| Topic | Encdr | kontrol-s8-protocol | Rows |
| ----- | ----- | ------------------- | ---- |
| Touchstrip position | Report 1, 10-bit at `[28..29]` (left) and `[34..35]` (right), max 1024 | Position field unresolved | `L-strip-slide`, `R-strip-slide` |
| Touchstrip touch | `[16]` and `[32]`, mask `0x01` | `[29]` and `[35]`, mask `0x01` (inside Encdr's position bytes) | `L-strip-touch`, `R-strip-touch` |
| Heartbeat | `[26..27]`, `[32..33]` called heartbeat | Not listed | hidden by default |
| Deck fader and FX dial touch | Report 2, byte 11 / 19 | Report 1, byte 11 / 19 (faders), byte 10 / 18 (FX knobs, different masks) | `*-fader-n`, `*-fxk-n-touch` |
| Screen encoders | `wrap16_wide`, 16-bit, `[1..16]` | Absolute position, same bytes | `*-senc-n-turn` |
| Browse and loop encoders | Not described | Report 1 `[1]` (left) / `[2]` (right), nibbles, mod 16 | `*-browse-turn`, `*-loop-turn` |
| Mixer channel fader | 12-bit at `[33..40]`, max 4078 | One byte at `[34]`, `[36]`, `[37]`, `[40]`, range 0..15 | `M-*-fader` |
| EQ LOW, channels C and D | 4-bit at `[96]` / `[106]` | One byte at `[96]` / `[106]`, range 0..15 | `M-C-eqlow`, `M-D-eqlow` |
| FILTER ON, deck assign, crossfader assign, MIC | Described | Not listed | marked "Encdr only" |
| Fader full scale | Max 4078 | 4077-4089 observed | all 12-bit rows (S4 uses 4095) |
| Report length | 41 and 109 bytes | Some report-1 rows are labelled length 109 | any `r1` change at a different length prints `baseline` again |

EQ LOW on channels C and D is the likeliest wrong width: both sources give one
byte at `[96]` / `[106]`, but the neighbouring knobs are 16-bit, so the field
probably spans `[95..96]` / `[105..106]`. Changes in `[95]`, `[105]` or the
upper nibble of `[96]` / `[106]` print as `?` during `M-C-eqlow` /
`M-D-eqlow`.

## Recorder verification status

- Opens the connected S8 (interface 5, shared) read-only without error; `--list`
  shows interface 5 (usage page `0xff01`).
- An idle run (20 s, agent running, no presses) received no reports, as in
  [agent-stop-validation.md](agent-stop-validation.md): the S8 reports only on
  change. Reports 1 (41 B) and 2 (109 B) were therefore not re-observed by this
  tool; they were observed by the earlier Swift recorder
  ([input-capture.md](input-capture.md)).
- Name resolution, nibble and 16-bit wrap deltas, deadband, heartbeat hiding
  and step markers were exercised with `--replay` on a synthetic log, not on
  hardware.

## Controls

191 rows. Left deck, mixer, right deck. "Expected field" cites report ID,
byte offset including the report ID and mask. `E` is Encdr, `P` is the
protocol dataset, `E=P` means both give the same location.

### Left deck

| #  | Step label       | Control                                                                | Motion                                                                        | Expected field                                                             | Result |
| -- | ---------------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------ |
| 1  | `L-disp-1`       | Left display button 1 (NI colour settings in MIDI mode)                | press and release, twice                                                      | E=P r1 [4] 0x20                                                            | |
| 2  | `L-disp-2`       | Left display button 2                                                  | press and release, twice                                                      | E=P r1 [4] 0x10                                                            | |
| 3  | `L-disp-3`       | Left display button 3                                                  | press and release, twice                                                      | E=P r1 [4] 0x01                                                            | |
| 4  | `L-disp-4`       | Left display button 4 (Controller Editor template switch in MIDI mode) | press and release, twice                                                      | E=P r1 [6] 0x40                                                            | |
| 5  | `L-disp-5`       | Left display button 5                                                  | press and release, twice                                                      | E=P r1 [5] 0x08                                                            | |
| 6  | `L-disp-6`       | Left display button 6                                                  | press and release, twice                                                      | E=P r1 [5] 0x10                                                            | |
| 7  | `L-disp-7`       | Left display button 7                                                  | press and release, twice                                                      | E=P r1 [5] 0x20                                                            | |
| 8  | `L-disp-8`       | Left display button 8                                                  | press and release, twice                                                      | E=P r1 [5] 0x40                                                            | |
| 9  | `L-senc-1-touch` | Left screen encoder 1 touch                                            | touch the cap, hold 2 s, release                                              | P r1 [10] 0x02                                                             | |
| 10 | `L-senc-1-turn`  | Left screen encoder 1 (wrap16_wide)                                    | 3 clicks CW, pause, 3 clicks CCW; then one slow full turn CW                  | E r2 [1..2] 16-bit; P r2 [1..2] ; confirm delta per click and direction    | |
| 11 | `L-senc-2-touch` | Left screen encoder 2 touch                                            | touch the cap, hold 2 s, release                                              | P r1 [10] 0x04                                                             | |
| 12 | `L-senc-2-turn`  | Left screen encoder 2 (wrap16_wide)                                    | 3 clicks CW, pause, 3 clicks CCW; then one slow full turn CW                  | E r2 [3..4] 16-bit; P r2 [3..4] ; confirm delta per click and direction    | |
| 13 | `L-senc-3-touch` | Left screen encoder 3 touch                                            | touch the cap, hold 2 s, release                                              | P r1 [10] 0x08                                                             | |
| 14 | `L-senc-3-turn`  | Left screen encoder 3 (wrap16_wide)                                    | 3 clicks CW, pause, 3 clicks CCW; then one slow full turn CW                  | E r2 [5..6] 16-bit; P r2 [5..6] ; confirm delta per click and direction    | |
| 15 | `L-senc-4-touch` | Left screen encoder 4 touch                                            | touch the cap, hold 2 s, release                                              | P r1 [10] 0x10                                                             | |
| 16 | `L-senc-4-turn`  | Left screen encoder 4 (wrap16_wide)                                    | 3 clicks CW, pause, 3 clicks CCW; then one slow full turn CW                  | E r2 [7..8] 16-bit; P r2 [7..8] ; confirm delta per click and direction    | |
| 17 | `L-browse-touch` | Left browse encoder touch                                              | touch, hold 2 s, release                                                      | P r1 [10] 0x20                                                             | |
| 18 | `L-browse-push`  | Left browse encoder push                                               | press and release, twice                                                      | P r1 [4] 0x08                                                              | |
| 19 | `L-browse-turn`  | Left browse encoder                                                    | 3 clicks CW, then 3 clicks CCW (protocol: CW decreases)                       | P r1 [1] 0xF0 high nibble, mod 16                                          | |
| 20 | `L-loop-touch`   | Left loop encoder touch                                                | touch, hold 2 s, release                                                      | P r1 [10] 0x01                                                             | |
| 21 | `L-loop-push`    | Left loop encoder push                                                 | press and release, twice                                                      | P r1 [7] 0x10                                                              | |
| 22 | `L-loop-turn`    | Left loop encoder                                                      | 3 clicks CW, then 3 clicks CCW (protocol: CW increases)                       | P r1 [1] 0x0F low nibble, mod 16                                           | |
| 23 | `L-back`         | Left BACK                                                              | press and release, twice                                                      | E=P r1 [6] 0x80                                                            | |
| 24 | `L-edit`         | Left EDIT                                                              | press and release, twice                                                      | E=P r1 [6] 0x01                                                            | |
| 25 | `L-capture`      | Left CAPTURE                                                           | press and release, twice                                                      | E=P r1 [6] 0x20                                                            | |
| 26 | `L-fxselect`     | Left FX SELECT                                                         | press and release, twice                                                      | E=P r1 [4] 0x40                                                            | |
| 27 | `L-fx1`          | Left FX 1 button                                                       | press and release, twice                                                      | E=P r1 [4] 0x80                                                            | |
| 28 | `L-fx2`          | Left FX 2 button                                                       | press and release, twice                                                      | E=P r1 [5] 0x04                                                            | |
| 29 | `L-fx3`          | Left FX 3 button                                                       | press and release, twice                                                      | E=P r1 [5] 0x02                                                            | |
| 30 | `L-fx4`          | Left FX 4 button                                                       | press and release, twice                                                      | E=P r1 [5] 0x01                                                            | |
| 31 | `L-on1`          | Left ON 1                                                              | press and release, twice                                                      | E=P r1 [6] 0x10                                                            | |
| 32 | `L-on2`          | Left ON 2                                                              | press and release, twice                                                      | E=P r1 [6] 0x08                                                            | |
| 33 | `L-on3`          | Left ON 3                                                              | press and release, twice                                                      | E=P r1 [6] 0x04                                                            | |
| 34 | `L-on4`          | Left ON 4                                                              | press and release, twice                                                      | E=P r1 [6] 0x02                                                            | |
| 35 | `L-deck`         | Left DECK                                                              | press and release, twice                                                      | E=P r1 [8] 0x04                                                            | |
| 36 | `L-flux`         | Left FLUX                                                              | press and release, twice                                                      | E=P r1 [8] 0x10                                                            | |
| 37 | `L-shift`        | Left SHIFT                                                             | press and release, twice                                                      | E=P r1 [9] 0x80                                                            | |
| 38 | `L-sync`         | Left SYNC                                                              | press and release, twice                                                      | E=P r1 [9] 0x40                                                            | |
| 39 | `L-cue`          | Left CUE                                                               | press and release, twice                                                      | E=P r1 [9] 0x20                                                            | |
| 40 | `L-play`         | Left PLAY                                                              | press and release, twice                                                      | E=P r1 [9] 0x10                                                            | |
| 41 | `L-hotcue`       | Left HOTCUE                                                            | press and release, twice                                                      | E=P r1 [9] 0x08                                                            | |
| 42 | `L-loop`         | Left LOOP                                                              | press and release, twice                                                      | E=P r1 [9] 0x04                                                            | |
| 43 | `L-freeze`       | Left FREEZE/SLICE                                                      | press and release, twice                                                      | E=P r1 [9] 0x02                                                            | |
| 44 | `L-remix`        | Left REMIX                                                             | press and release, twice                                                      | E=P r1 [9] 0x01                                                            | |
| 45 | `L-pad-1`        | Left pad 1                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [8] 0x08                                                            | |
| 46 | `L-pad-2`        | Left pad 2                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [8] 0x01                                                            | |
| 47 | `L-pad-3`        | Left pad 3                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [7] 0x01                                                            | |
| 48 | `L-pad-4`        | Left pad 4                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [7] 0x02                                                            | |
| 49 | `L-pad-5`        | Left pad 5                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [8] 0x20                                                            | |
| 50 | `L-pad-6`        | Left pad 6                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [8] 0x40                                                            | |
| 51 | `L-pad-7`        | Left pad 7                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [8] 0x80                                                            | |
| 52 | `L-pad-8`        | Left pad 8                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [8] 0x02                                                            | |
| 53 | `L-fxk-1-touch`  | Left FX knob 1 touch                                                   | touch, hold 2 s, release                                                      | E r2 [11] 0x10; P r1 [10] 0x40                                             | |
| 54 | `L-fxk-1`        | Left FX knob 1                                                         | full sweep min → max → min                                                    | E r2 [43..44] 12-bit; P r2 [43..44]                                        | |
| 55 | `L-fxk-2-touch`  | Left FX knob 2 touch                                                   | touch, hold 2 s, release                                                      | E r2 [11] 0x20; P r1 [10] 0x80                                             | |
| 56 | `L-fxk-2`        | Left FX knob 2                                                         | full sweep min → max → min                                                    | E r2 [45..46] 12-bit; P r2 [45..46]                                        | |
| 57 | `L-fxk-3-touch`  | Left FX knob 3 touch                                                   | touch, hold 2 s, release                                                      | E r2 [11] 0x40; P r1 [11] 0x10                                             | |
| 58 | `L-fxk-3`        | Left FX knob 3                                                         | full sweep min → max → min                                                    | E r2 [47..48] 12-bit; P r2 [47..48]                                        | |
| 59 | `L-fxk-4-touch`  | Left FX knob 4 touch                                                   | touch, hold 2 s, release                                                      | E r2 [11] 0x80; P r1 [11] 0x20                                             | |
| 60 | `L-fxk-4`        | Left FX knob 4                                                         | full sweep min → max → min                                                    | E r2 [49..50] 12-bit; P r2 [49..50]                                        | |
| 61 | `L-fader-1`      | Left deck fader 1 (touch + position)                                   | touch, full sweep min → max → min, release                                    | E r2 [17..18] 12-bit; P r2 [17..18]; touch E r2 [11] 0x1 vs P r1 [11] 0x01 | |
| 62 | `L-fader-2`      | Left deck fader 2 (touch + position)                                   | touch, full sweep min → max → min, release                                    | E r2 [19..20] 12-bit; P r2 [19..20]; touch E r2 [11] 0x2 vs P r1 [11] 0x02 | |
| 63 | `L-fader-3`      | Left deck fader 3 (touch + position)                                   | touch, full sweep min → max → min, release                                    | E r2 [21..22] 12-bit; P r2 [21..22]; touch E r2 [11] 0x4 vs P r1 [11] 0x04 | |
| 64 | `L-fader-4`      | Left deck fader 4 (touch + position)                                   | touch, full sweep min → max → min, release                                    | E r2 [23..24] 12-bit; P r2 [23..24]; touch E r2 [11] 0x8 vs P r1 [11] 0x08 | |
| 65 | `L-strip-touch`  | Left touchstrip touch                                                  | touch mid-strip, hold 2 s, release                                            | E r1 [16] 0x01; P r1 [29] 0x01; compare Encdr touch byte with P touch byte | |
| 66 | `L-strip-slide`  | Left touchstrip position                                               | touch at bottom, slide to top and back, release; then tap bottom, middle, top | E r1 [28..29] 10-bit (E: 10-bit, max 1024; P: position unresolved)         | |

### Mixer

| #   | Step label       | Control                                                | Motion                                                                          | Expected field                                                                                        | Result |
| --- | ---------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ------ |
| 67  | `M-A-gain`       | Channel A GAIN                                         | full sweep min → max → min                                                      | E r2 [69..70] 12-bit; P r2 [69..70]                                                                   | |
| 68  | `M-A-eqhi`       | Channel A EQ HI                                        | full sweep min → max → min, pause at centre detent                              | E r2 [71..72] 12-bit; P r2 [71..72]                                                                   | |
| 69  | `M-A-eqmid`      | Channel A EQ MID                                       | full sweep min → max → min, pause at centre detent                              | E r2 [73..74] 12-bit; P r2 [73..74]                                                                   | |
| 70  | `M-A-eqlow`      | Channel A EQ LOW                                       | full sweep min → max → min, pause at centre detent                              | E r2 [75..76] 12-bit; P r2 [75..76]                                                                   | |
| 71  | `M-A-filter`     | Channel A FILTER                                       | full sweep min → max → min, pause at centre detent                              | E r2 [77..78] 12-bit; P r2 [77..78]                                                                   | |
| 72  | `M-A-filteron`   | Channel A FILTER ON                                    | press and release, twice                                                        | E r1 [20] 0x02 (Encdr only)                                                                           | |
| 73  | `M-A-fx1`        | Channel A FX assign 1                                  | press and release, twice                                                        | E=P r1 [22] 0x20                                                                                      | |
| 74  | `M-A-fx2`        | Channel A FX assign 2                                  | press and release, twice                                                        | E=P r1 [22] 0x40                                                                                      | |
| 75  | `M-A-deckassign` | Channel A deck assign (left or right deck pair button) | press and release, twice                                                        | E r1 [22] 0x02 (Encdr only)                                                                           | |
| 76  | `M-A-cue`        | Channel A CUE (PFL)                                    | press and release; note latch bit `[39]` toggles (already verified, re-confirm) | E=P r1 [20] 0x08                                                                                      | |
| 77  | `M-A-xf-left`    | Channel A crossfader assign LEFT                       | press each position of the switch, return to centre                             | E r1 [24] 0x20 (Encdr only)                                                                           | |
| 78  | `M-A-xf-right`   | Channel A crossfader assign RIGHT                      | same switch, RIGHT position                                                     | E r1 [24] 0x10 (Encdr only)                                                                           | |
| 79  | `M-A-fader`      | Channel A fader                                        | full sweep bottom → top → bottom                                                | E r2 [33..34] 12-bit; P r2 [34] ; P has only one byte, 0..15 observed: confirm 12-bit at [33..40]     | |
| 80  | `M-B-gain`       | Channel B GAIN                                         | full sweep min → max → min                                                      | E r2 [79..80] 12-bit; P r2 [79..80]                                                                   | |
| 81  | `M-B-eqhi`       | Channel B EQ HI                                        | full sweep min → max → min, pause at centre detent                              | E r2 [81..82] 12-bit; P r2 [81..82]                                                                   | |
| 82  | `M-B-eqmid`      | Channel B EQ MID                                       | full sweep min → max → min, pause at centre detent                              | E r2 [83..84] 12-bit; P r2 [83..84]                                                                   | |
| 83  | `M-B-eqlow`      | Channel B EQ LOW                                       | full sweep min → max → min, pause at centre detent                              | E r2 [85..86] 12-bit; P r2 [85..86]                                                                   | |
| 84  | `M-B-filter`     | Channel B FILTER                                       | full sweep min → max → min, pause at centre detent                              | E r2 [87..88] 12-bit; P r2 [87..88]                                                                   | |
| 85  | `M-B-filteron`   | Channel B FILTER ON                                    | press and release, twice                                                        | E r1 [23] 0x08 (Encdr only)                                                                           | |
| 86  | `M-B-fx1`        | Channel B FX assign 1                                  | press and release, twice                                                        | E=P r1 [25] 0x10                                                                                      | |
| 87  | `M-B-fx2`        | Channel B FX assign 2                                  | press and release, twice                                                        | E=P r1 [25] 0x20                                                                                      | |
| 88  | `M-B-deckassign` | Channel B deck assign (left or right deck pair button) | press and release, twice                                                        | E r1 [25] 0x02 (Encdr only)                                                                           | |
| 89  | `M-B-cue`        | Channel B CUE (PFL)                                    | press and release; note latch bit `[39]` toggles (already verified, re-confirm) | E=P r1 [23] 0x20                                                                                      | |
| 90  | `M-B-xf-left`    | Channel B crossfader assign LEFT                       | press each position of the switch, return to centre                             | E r1 [24] 0x08 (Encdr only)                                                                           | |
| 91  | `M-B-xf-right`   | Channel B crossfader assign RIGHT                      | same switch, RIGHT position                                                     | E r1 [24] 0x04 (Encdr only)                                                                           | |
| 92  | `M-B-fader`      | Channel B fader                                        | full sweep bottom → top → bottom                                                | E r2 [35..36] 12-bit; P r2 [36] ; P has only one byte, 0..15 observed: confirm 12-bit at [33..40]     | |
| 93  | `M-C-gain`       | Channel C GAIN                                         | full sweep min → max → min                                                      | E r2 [89..90] 12-bit; P r2 [89..90]                                                                   | |
| 94  | `M-C-eqhi`       | Channel C EQ HI                                        | full sweep min → max → min, pause at centre detent                              | E r2 [91..92] 12-bit; P r2 [91..92]                                                                   | |
| 95  | `M-C-eqmid`      | Channel C EQ MID                                       | full sweep min → max → min, pause at centre detent                              | E r2 [93..94] 12-bit; P r2 [93..94]                                                                   | |
| 96  | `M-C-eqlow`      | Channel C EQ LOW                                       | full sweep min → max → min, pause at centre detent                              | E r2 [96] 4-bit; P r2 [96] ; check bytes 95/96 (C) and 105/106 (D)                                    | |
| 97  | `M-C-filter`     | Channel C FILTER                                       | full sweep min → max → min, pause at centre detent                              | E r2 [97..98] 12-bit; P r2 [97..98]                                                                   | |
| 98  | `M-C-filteron`   | Channel C FILTER ON                                    | press and release, twice                                                        | E r1 [20] 0x01 (Encdr only)                                                                           | |
| 99  | `M-C-fx1`        | Channel C FX assign 1                                  | press and release, twice                                                        | E=P r1 [22] 0x08                                                                                      | |
| 100 | `M-C-fx2`        | Channel C FX assign 2                                  | press and release, twice                                                        | E=P r1 [22] 0x10                                                                                      | |
| 101 | `M-C-deckassign` | Channel C deck assign (left or right deck pair button) | press and release, twice                                                        | E r1 [22] 0x01 (Encdr only)                                                                           | |
| 102 | `M-C-cue`        | Channel C CUE (PFL)                                    | press and release; note latch bit `[39]` toggles (already verified, re-confirm) | E=P r1 [20] 0x04                                                                                      | |
| 103 | `M-C-xf-left`    | Channel C crossfader assign LEFT                       | press each position of the switch, return to centre                             | E r1 [24] 0x80 (Encdr only)                                                                           | |
| 104 | `M-C-xf-right`   | Channel C crossfader assign RIGHT                      | same switch, RIGHT position                                                     | E r1 [24] 0x40 (Encdr only)                                                                           | |
| 105 | `M-C-fader`      | Channel C fader                                        | full sweep bottom → top → bottom                                                | E r2 [37..38] 12-bit; P r2 [37..38] ; P has only one byte, 0..15 observed: confirm 12-bit at [33..40] | |
| 106 | `M-D-gain`       | Channel D GAIN                                         | full sweep min → max → min                                                      | E r2 [99..100] 12-bit; P r2 [99..100]                                                                 | |
| 107 | `M-D-eqhi`       | Channel D EQ HI                                        | full sweep min → max → min, pause at centre detent                              | E r2 [101..102] 12-bit; P r2 [101..102]                                                               | |
| 108 | `M-D-eqmid`      | Channel D EQ MID                                       | full sweep min → max → min, pause at centre detent                              | E r2 [103..104] 12-bit; P r2 [103..104]                                                               | |
| 109 | `M-D-eqlow`      | Channel D EQ LOW                                       | full sweep min → max → min, pause at centre detent                              | E r2 [106] 4-bit; P r2 [106] ; check bytes 95/96 (C) and 105/106 (D)                                  | |
| 110 | `M-D-filter`     | Channel D FILTER                                       | full sweep min → max → min, pause at centre detent                              | E r2 [107..108] 12-bit; P r2 [107..108]                                                               | |
| 111 | `M-D-filteron`   | Channel D FILTER ON                                    | press and release, twice                                                        | E r1 [23] 0x10 (Encdr only)                                                                           | |
| 112 | `M-D-fx1`        | Channel D FX assign 1                                  | press and release, twice                                                        | E=P r1 [25] 0x40                                                                                      | |
| 113 | `M-D-fx2`        | Channel D FX assign 2                                  | press and release, twice                                                        | E=P r1 [25] 0x80                                                                                      | |
| 114 | `M-D-deckassign` | Channel D deck assign (left or right deck pair button) | press and release, twice                                                        | E r1 [25] 0x04 (Encdr only)                                                                           | |
| 115 | `M-D-cue`        | Channel D CUE (PFL)                                    | press and release; note latch bit `[39]` toggles (already verified, re-confirm) | E=P r1 [23] 0x40                                                                                      | |
| 116 | `M-D-xf-left`    | Channel D crossfader assign LEFT                       | press each position of the switch, return to centre                             | E r1 [24] 0x02 (Encdr only)                                                                           | |
| 117 | `M-D-xf-right`   | Channel D crossfader assign RIGHT                      | same switch, RIGHT position                                                     | E r1 [24] 0x01 (Encdr only)                                                                           | |
| 118 | `M-D-fader`      | Channel D fader                                        | full sweep bottom → top → bottom                                                | E r2 [39..40] 12-bit; P r2 [40] ; P has only one byte, 0..15 observed: confirm 12-bit at [33..40]     | |
| 119 | `M-xfader`       | Crossfader                                             | full sweep left → right → left                                                  | E r2 [41..42] 12-bit; P r2 [41..42]                                                                   | |
| 120 | `M-snap`         | SNAP                                                   | press and release, twice                                                        | E=P r1 [22] 0x80                                                                                      | |
| 121 | `M-quantize`     | QUANTIZE                                               | press and release, twice                                                        | E=P r1 [25] 0x08                                                                                      | |
| 122 | `M-tempo-push`   | TEMPO encoder push                                     | press and release, twice                                                        | E=P r1 [23] 0x04                                                                                      | |
| 123 | `M-tempo-turn`   | TEMPO encoder                                          | 3 clicks CW, then 3 clicks CCW (P raw codes 247 CW / 248 CCW)                   | E r1 [3] low 4 bits wrap16; P r1 [3]                                                                  | |
| 124 | `M-mic1`         | MIC 1 button                                           | press and release, twice                                                        | E r1 [22] 0x04 (Encdr only)                                                                           | |
| 125 | `M-mic2`         | MIC 2 button                                           | press and release, twice                                                        | E r1 [25] 0x01 (Encdr only)                                                                           | |

### Right deck

| #   | Step label       | Control                                                                 | Motion                                                                        | Expected field                                                              | Result |
| --- | ---------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------------- | --------------------------------------------------------------------------- | ------ |
| 126 | `R-disp-1`       | Right display button 1 (NI colour settings in MIDI mode)                | press and release, twice                                                      | E=P r1 [12] 0x20                                                            | |
| 127 | `R-disp-2`       | Right display button 2                                                  | press and release, twice                                                      | E=P r1 [12] 0x10                                                            | |
| 128 | `R-disp-3`       | Right display button 3                                                  | press and release, twice                                                      | E=P r1 [12] 0x01                                                            | |
| 129 | `R-disp-4`       | Right display button 4 (Controller Editor template switch in MIDI mode) | press and release, twice                                                      | E=P r1 [14] 0x40                                                            | |
| 130 | `R-disp-5`       | Right display button 5                                                  | press and release, twice                                                      | E=P r1 [13] 0x08                                                            | |
| 131 | `R-disp-6`       | Right display button 6                                                  | press and release, twice                                                      | E=P r1 [13] 0x10                                                            | |
| 132 | `R-disp-7`       | Right display button 7                                                  | press and release, twice                                                      | E=P r1 [13] 0x20                                                            | |
| 133 | `R-disp-8`       | Right display button 8                                                  | press and release, twice                                                      | E=P r1 [13] 0x40                                                            | |
| 134 | `R-senc-1-touch` | Right screen encoder 1 touch                                            | touch the cap, hold 2 s, release                                              | P r1 [18] 0x02                                                              | |
| 135 | `R-senc-1-turn`  | Right screen encoder 1 (wrap16_wide)                                    | 3 clicks CW, pause, 3 clicks CCW; then one slow full turn CW                  | E r2 [9..10] 16-bit; P r2 [9..10] ; confirm delta per click and direction   | |
| 136 | `R-senc-2-touch` | Right screen encoder 2 touch                                            | touch the cap, hold 2 s, release                                              | P r1 [18] 0x04                                                              | |
| 137 | `R-senc-2-turn`  | Right screen encoder 2 (wrap16_wide)                                    | 3 clicks CW, pause, 3 clicks CCW; then one slow full turn CW                  | E r2 [11..12] 16-bit; P r2 [11..12] ; confirm delta per click and direction | |
| 138 | `R-senc-3-touch` | Right screen encoder 3 touch                                            | touch the cap, hold 2 s, release                                              | P r1 [18] 0x08                                                              | |
| 139 | `R-senc-3-turn`  | Right screen encoder 3 (wrap16_wide)                                    | 3 clicks CW, pause, 3 clicks CCW; then one slow full turn CW                  | E r2 [13..14] 16-bit; P r2 [13..14] ; confirm delta per click and direction | |
| 140 | `R-senc-4-touch` | Right screen encoder 4 touch                                            | touch the cap, hold 2 s, release                                              | P r1 [18] 0x10                                                              | |
| 141 | `R-senc-4-turn`  | Right screen encoder 4 (wrap16_wide)                                    | 3 clicks CW, pause, 3 clicks CCW; then one slow full turn CW                  | E r2 [15..16] 16-bit; P r2 [15..16] ; confirm delta per click and direction | |
| 142 | `R-browse-touch` | Right browse encoder touch                                              | touch, hold 2 s, release                                                      | P r1 [18] 0x20                                                              | |
| 143 | `R-browse-push`  | Right browse encoder push                                               | press and release, twice                                                      | P r1 [12] 0x08                                                              | |
| 144 | `R-browse-turn`  | Right browse encoder                                                    | 3 clicks CW, then 3 clicks CCW (protocol: CW decreases)                       | P r1 [2] 0xF0 high nibble, mod 16                                           | |
| 145 | `R-loop-touch`   | Right loop encoder touch                                                | touch, hold 2 s, release                                                      | P r1 [18] 0x01                                                              | |
| 146 | `R-loop-push`    | Right loop encoder push                                                 | press and release, twice                                                      | P r1 [15] 0x10                                                              | |
| 147 | `R-loop-turn`    | Right loop encoder                                                      | 3 clicks CW, then 3 clicks CCW (protocol: CW increases)                       | P r1 [2] 0x0F low nibble, mod 16                                            | |
| 148 | `R-back`         | Right BACK                                                              | press and release, twice                                                      | E=P r1 [14] 0x80                                                            | |
| 149 | `R-edit`         | Right EDIT                                                              | press and release, twice                                                      | E=P r1 [14] 0x01                                                            | |
| 150 | `R-capture`      | Right CAPTURE                                                           | press and release, twice                                                      | E=P r1 [14] 0x20                                                            | |
| 151 | `R-fxselect`     | Right FX SELECT                                                         | press and release, twice                                                      | E=P r1 [12] 0x40                                                            | |
| 152 | `R-fx1`          | Right FX 1 button                                                       | press and release, twice                                                      | E=P r1 [12] 0x80                                                            | |
| 153 | `R-fx2`          | Right FX 2 button                                                       | press and release, twice                                                      | E=P r1 [13] 0x04                                                            | |
| 154 | `R-fx3`          | Right FX 3 button                                                       | press and release, twice                                                      | E=P r1 [13] 0x02                                                            | |
| 155 | `R-fx4`          | Right FX 4 button                                                       | press and release, twice                                                      | E=P r1 [13] 0x01                                                            | |
| 156 | `R-on1`          | Right ON 1                                                              | press and release, twice                                                      | E=P r1 [14] 0x10                                                            | |
| 157 | `R-on2`          | Right ON 2                                                              | press and release, twice                                                      | E=P r1 [14] 0x08                                                            | |
| 158 | `R-on3`          | Right ON 3                                                              | press and release, twice                                                      | E=P r1 [14] 0x04                                                            | |
| 159 | `R-on4`          | Right ON 4                                                              | press and release, twice                                                      | E=P r1 [14] 0x02                                                            | |
| 160 | `R-deck`         | Right DECK                                                              | press and release, twice                                                      | E=P r1 [16] 0x04                                                            | |
| 161 | `R-flux`         | Right FLUX                                                              | press and release, twice                                                      | E=P r1 [16] 0x10                                                            | |
| 162 | `R-shift`        | Right SHIFT                                                             | press and release, twice                                                      | E=P r1 [17] 0x80                                                            | |
| 163 | `R-sync`         | Right SYNC                                                              | press and release, twice                                                      | E=P r1 [17] 0x40                                                            | |
| 164 | `R-cue`          | Right CUE                                                               | press and release, twice                                                      | E=P r1 [17] 0x20                                                            | |
| 165 | `R-play`         | Right PLAY                                                              | press and release, twice                                                      | E=P r1 [17] 0x10                                                            | |
| 166 | `R-hotcue`       | Right HOTCUE                                                            | press and release, twice                                                      | E=P r1 [17] 0x08                                                            | |
| 167 | `R-loop`         | Right LOOP                                                              | press and release, twice                                                      | E=P r1 [17] 0x04                                                            | |
| 168 | `R-freeze`       | Right FREEZE/SLICE                                                      | press and release, twice                                                      | E=P r1 [17] 0x02                                                            | |
| 169 | `R-remix`        | Right REMIX                                                             | press and release, twice                                                      | E=P r1 [17] 0x01                                                            | |
| 170 | `R-pad-1`        | Right pad 1                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [16] 0x08                                                            | |
| 171 | `R-pad-2`        | Right pad 2                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [16] 0x01                                                            | |
| 172 | `R-pad-3`        | Right pad 3                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [15] 0x01                                                            | |
| 173 | `R-pad-4`        | Right pad 4                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [15] 0x02                                                            | |
| 174 | `R-pad-5`        | Right pad 5                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [16] 0x20                                                            | |
| 175 | `R-pad-6`        | Right pad 6                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [16] 0x40                                                            | |
| 176 | `R-pad-7`        | Right pad 7                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [16] 0x80                                                            | |
| 177 | `R-pad-8`        | Right pad 8                                                             | press firmly, then lightly; release (note any pressure bytes as `?`)          | E=P r1 [16] 0x02                                                            | |
| 178 | `R-fxk-1-touch`  | Right FX knob 1 touch                                                   | touch, hold 2 s, release                                                      | E r2 [19] 0x10; P r1 [18] 0x40                                              | |
| 179 | `R-fxk-1`        | Right FX knob 1                                                         | full sweep min → max → min                                                    | E r2 [51..52] 12-bit; P r2 [51..52]                                         | |
| 180 | `R-fxk-2-touch`  | Right FX knob 2 touch                                                   | touch, hold 2 s, release                                                      | E r2 [19] 0x20; P r1 [18] 0x80                                              | |
| 181 | `R-fxk-2`        | Right FX knob 2                                                         | full sweep min → max → min                                                    | E r2 [53..54] 12-bit; P r2 [53..54]                                         | |
| 182 | `R-fxk-3-touch`  | Right FX knob 3 touch                                                   | touch, hold 2 s, release                                                      | E r2 [19] 0x40; P r1 [19] 0x10                                              | |
| 183 | `R-fxk-3`        | Right FX knob 3                                                         | full sweep min → max → min                                                    | E r2 [55..56] 12-bit; P r2 [55..56]                                         | |
| 184 | `R-fxk-4-touch`  | Right FX knob 4 touch                                                   | touch, hold 2 s, release                                                      | E r2 [19] 0x80; P r1 [19] 0x20                                              | |
| 185 | `R-fxk-4`        | Right FX knob 4                                                         | full sweep min → max → min                                                    | E r2 [57..58] 12-bit; P r2 [57..58]                                         | |
| 186 | `R-fader-1`      | Right deck fader 1 (touch + position)                                   | touch, full sweep min → max → min, release                                    | E r2 [25..26] 12-bit; P r2 [25..26]; touch E r2 [19] 0x1 vs P r1 [19] 0x01  | |
| 187 | `R-fader-2`      | Right deck fader 2 (touch + position)                                   | touch, full sweep min → max → min, release                                    | E r2 [27..28] 12-bit; P r2 [27..28]; touch E r2 [19] 0x2 vs P r1 [19] 0x02  | |
| 188 | `R-fader-3`      | Right deck fader 3 (touch + position)                                   | touch, full sweep min → max → min, release                                    | E r2 [29..30] 12-bit; P r2 [29..30]; touch E r2 [19] 0x4 vs P r1 [19] 0x04  | |
| 189 | `R-fader-4`      | Right deck fader 4 (touch + position)                                   | touch, full sweep min → max → min, release                                    | E r2 [31..32] 12-bit; P r2 [31..32]; touch E r2 [19] 0x8 vs P r1 [19] 0x08  | |
| 190 | `R-strip-touch`  | Right touchstrip touch                                                  | touch mid-strip, hold 2 s, release                                            | E r1 [32] 0x01; P r1 [35] 0x01; compare Encdr touch byte with P touch byte  | |
| 191 | `R-strip-slide`  | Right touchstrip position                                               | touch at bottom, slide to top and back, release; then tap bottom, middle, top | E r1 [34..35] 10-bit (E: 10-bit, max 1024; P: position unresolved)          | |

## Done when

- Every row has a Result with report, byte, mask or range, and Verified or
  Unverified.
- The discrepancy table has a decision per row, with the log excerpt as
  evidence.
- [input-capture.md](input-capture.md) and [hardware-notes.md](hardware-notes.md)
  carry the verified field map; Encdr differences go in the upstream issue
  list.
- The agent is restored and the Djay mapping works again.
