---
summary: "Per-control results of the agent-stopped S8 input capture: field, observed range and status for all 191 checklist rows."
type: validation
issues: [10]
---

# S8 input capture: results per control

Per-control results of the agent-stopped capture described in
[input-capture.md](input-capture.md#agent-stopped-capture-2026-11-09), one row per row of
[capture-checklist.md](capture-checklist.md). Offsets include the report ID: `r1` is
report 1 (41 bytes) and `r2` is report 2 (109 bytes). The four mixer rows without a number
come from a separate raw capture on 2026-11-10.

<?toc
max-level: 2
?>

- [Rows](#rows)

<?/toc?>

## Rows

### Left deck: displays and encoders

| #   | Step             | Control                                                                | Field          | Observed                                   | Status   |
| --- | ---------------- | ---------------------------------------------------------------------- | -------------- | ------------------------------------------ | -------- |
| 1   | `L-disp-1`       | Left display button 1 (NI colour settings in MIDI mode)                | `r1 [4] 0x20`  | 2 presses                                  | Verified |
| 2   | `L-disp-2`       | Left display button 2                                                  | `r1 [4] 0x10`  | 2 presses                                  | Verified |
| 3   | `L-disp-3`       | Left display button 3                                                  | `r1 [4] 0x01`  | 2 presses                                  | Verified |
| 4   | `L-disp-4`       | Left display button 4 (Controller Editor template switch in MIDI mode) | `r1 [6] 0x40`  | 2 presses                                  | Verified |
| 5   | `L-disp-5`       | Left display button 5                                                  | `r1 [5] 0x08`  | 2 presses                                  | Verified |
| 6   | `L-disp-6`       | Left display button 6                                                  | `r1 [5] 0x10`  | 2 presses                                  | Verified |
| 7   | `L-disp-7`       | Left display button 7                                                  | `r1 [5] 0x20`  | 2 presses                                  | Verified |
| 8   | `L-disp-8`       | Left display button 8                                                  | `r1 [5] 0x40`  | 2 presses                                  | Verified |
| 9   | `L-senc-1-touch` | Left screen encoder 1 touch                                            | `r1 [10] 0x02` | touch and release                          | Verified |
| 10  | `L-senc-1-turn`  | Left screen encoder 1 (wrap16_wide)                                    | `r2 [1..2]`    | 0–996, wraps past 999; clockwise counts up | Verified |
| 11  | `L-senc-2-touch` | Left screen encoder 2 touch                                            | `r1 [10] 0x04` | touch and release                          | Verified |
| 12  | `L-senc-2-turn`  | Left screen encoder 2 (wrap16_wide)                                    | `r2 [3..4]`    | 0–997, wraps past 999; clockwise counts up | Verified |
| 13  | `L-senc-3-touch` | Left screen encoder 3 touch                                            | `r1 [10] 0x08` | touch and release                          | Verified |
| 14  | `L-senc-3-turn`  | Left screen encoder 3 (wrap16_wide)                                    | `r2 [5..6]`    | 0–999, wraps past 999; clockwise counts up | Verified |
| 15  | `L-senc-4-touch` | Left screen encoder 4 touch                                            | `r1 [10] 0x10` | touch and release                          | Verified |
| 16  | `L-senc-4-turn`  | Left screen encoder 4 (wrap16_wide)                                    | `r2 [7..8]`    | 0–995, wraps past 999; clockwise counts up | Verified |
| 17  | `L-browse-touch` | Left browse encoder touch                                              | `r1 [10] 0x20` | touch and release                          | Verified |
| 18  | `L-browse-push`  | Left browse encoder push                                               | `r1 [4] 0x08`  | 2 presses                                  | Verified |
| 19  | `L-browse-turn`  | Left browse encoder                                                    | `r1 [1] 0xf0`  | clockwise +3, anticlockwise −3, mod 16     | Verified |
| 20  | `L-loop-touch`   | Left loop encoder touch                                                | `r1 [10] 0x01` | touch and release                          | Verified |
| 21  | `L-loop-push`    | Left loop encoder push                                                 | `r1 [7] 0x10`  | 2 presses                                  | Verified |
| 22  | `L-loop-turn`    | Left loop encoder                                                      | `r1 [1] 0x0f`  | clockwise +3, anticlockwise −3, mod 16     | Verified |

### Left deck: buttons and pads

| #   | Step         | Control           | Field         | Observed  | Status   |
| --- | ------------ | ----------------- | ------------- | --------- | -------- |
| 23  | `L-back`     | Left BACK         | `r1 [6] 0x80` | 2 presses | Verified |
| 24  | `L-edit`     | Left EDIT         | `r1 [6] 0x01` | 2 presses | Verified |
| 25  | `L-capture`  | Left CAPTURE      | `r1 [6] 0x20` | 2 presses | Verified |
| 26  | `L-fxselect` | Left FX SELECT    | `r1 [4] 0x40` | 2 presses | Verified |
| 27  | `L-fx1`      | Left FX 1 button  | `r1 [4] 0x80` | 2 presses | Verified |
| 28  | `L-fx2`      | Left FX 2 button  | `r1 [5] 0x04` | 2 presses | Verified |
| 29  | `L-fx3`      | Left FX 3 button  | `r1 [5] 0x02` | 2 presses | Verified |
| 30  | `L-fx4`      | Left FX 4 button  | `r1 [5] 0x01` | 2 presses | Verified |
| 31  | `L-on1`      | Left ON 1         | `r1 [6] 0x10` | 2 presses | Verified |
| 32  | `L-on2`      | Left ON 2         | `r1 [6] 0x08` | 2 presses | Verified |
| 33  | `L-on3`      | Left ON 3         | `r1 [6] 0x04` | 2 presses | Verified |
| 34  | `L-on4`      | Left ON 4         | `r1 [6] 0x02` | 2 presses | Verified |
| 35  | `L-deck`     | Left DECK         | `r1 [8] 0x04` | 2 presses | Verified |
| 36  | `L-flux`     | Left FLUX         | `r1 [8] 0x10` | 2 presses | Verified |
| 37  | `L-shift`    | Left SHIFT        | `r1 [9] 0x80` | 2 presses | Verified |
| 38  | `L-sync`     | Left SYNC         | `r1 [9] 0x40` | 2 presses | Verified |
| 39  | `L-cue`      | Left CUE          | `r1 [9] 0x20` | 2 presses | Verified |
| 40  | `L-play`     | Left PLAY         | `r1 [9] 0x10` | 2 presses | Verified |
| 41  | `L-hotcue`   | Left HOTCUE       | `r1 [9] 0x08` | 2 presses | Verified |
| 42  | `L-loop`     | Left LOOP         | `r1 [9] 0x04` | 2 presses | Verified |
| 43  | `L-freeze`   | Left FREEZE/SLICE | `r1 [9] 0x02` | 2 presses | Verified |
| 44  | `L-remix`    | Left REMIX        | `r1 [9] 0x01` | 2 presses | Verified |
| 45  | `L-pad-1`    | Left pad 1        | `r1 [8] 0x08` | 2 presses | Verified |
| 46  | `L-pad-2`    | Left pad 2        | `r1 [8] 0x01` | 2 presses | Verified |
| 47  | `L-pad-3`    | Left pad 3        | `r1 [7] 0x01` | 2 presses | Verified |
| 48  | `L-pad-4`    | Left pad 4        | `r1 [7] 0x02` | 2 presses | Verified |
| 49  | `L-pad-5`    | Left pad 5        | `r1 [8] 0x20` | 2 presses | Verified |
| 50  | `L-pad-6`    | Left pad 6        | `r1 [8] 0x40` | 2 presses | Verified |
| 51  | `L-pad-7`    | Left pad 7        | `r1 [8] 0x80` | 2 presses | Verified |
| 52  | `L-pad-8`    | Left pad 8        | `r1 [8] 0x02` | 2 presses | Verified |

### Left deck: knobs, faders and touchstrip

| #   | Step            | Control                              | Field                         | Observed                                                    | Status   |
| --- | --------------- | ------------------------------------ | ----------------------------- | ----------------------------------------------------------- | -------- |
| 53  | `L-fxk-1-touch` | Left FX knob 1 touch                 | `r1 [10] 0x40`                | touch and release                                           | Verified |
| 54  | `L-fxk-1`       | Left FX knob 1                       | `r2 [43..44]`                 | 11–4088                                                     | Verified |
| 55  | `L-fxk-2-touch` | Left FX knob 2 touch                 | `r1 [10] 0x80`                | touch and release                                           | Verified |
| 56  | `L-fxk-2`       | Left FX knob 2                       | `r2 [45..46]`                 | 12–4084                                                     | Verified |
| 57  | `L-fxk-3-touch` | Left FX knob 3 touch                 | `r1 [11] 0x10`                | touch and release                                           | Verified |
| 58  | `L-fxk-3`       | Left FX knob 3                       | `r2 [47..48]`                 | 15–4088                                                     | Verified |
| 59  | `L-fxk-4-touch` | Left FX knob 4 touch                 | `r1 [11] 0x20`                | touch and release                                           | Verified |
| 60  | `L-fxk-4`       | Left FX knob 4                       | `r2 [49..50]`                 | 17–4088                                                     | Verified |
| 61  | `L-fader-1`     | Left deck fader 1 (touch + position) | `r2 [17..18]`, `r1 [11] 0x01` | 12–4088; touch and release                                  | Verified |
| 62  | `L-fader-2`     | Left deck fader 2 (touch + position) | `r2 [19..20]`, `r1 [11] 0x02` | 13–4087; touch release only (touched before the step began) | Verified |
| 63  | `L-fader-3`     | Left deck fader 3 (touch + position) | `r2 [21..22]`, `r1 [11] 0x04` | 16–4088; touch and release                                  | Verified |
| 64  | `L-fader-4`     | Left deck fader 4 (touch + position) | `r2 [23..24]`, `r1 [11] 0x08` | 13–4088; touch and release                                  | Verified |
| 65  | `L-strip-touch` | Left touchstrip touch                | `r1 [28..29]`                 | 495–509 while touched, 0 on release; no separate touch bit  | Verified |
| 66  | `L-strip-slide` | Left touchstrip position             | `r1 [28..29]`                 | 1–1024 while touched, 0 on release                          | Verified |

### Mixer: channel A

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

### Mixer: channel B

| #   | Step             | Control                                                | Field                  | Observed                                        | Status   |
| --- | ---------------- | ------------------------------------------------------ | ---------------------- | ----------------------------------------------- | -------- |
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

### Mixer: channel C

| #   | Step             | Control                                                | Field                  | Observed                                        | Status   |
| --- | ---------------- | ------------------------------------------------------ | ---------------------- | ----------------------------------------------- | -------- |
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

### Mixer: channel D

| #   | Step             | Control                                                | Field                  | Observed                                        | Status   |
| --- | ---------------- | ------------------------------------------------------ | ---------------------- | ----------------------------------------------- | -------- |
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

### Mixer: master section

| #   | Step           | Control                                    | Field          | Observed                                        | Status                        |
| --- | -------------- | ------------------------------------------ | -------------- | ----------------------------------------------- | ----------------------------- |
| 119 | `M-xfader`     | Crossfader                                 | `r2 [41..42]`  | 22–4090                                         | Verified                      |
| 120 | `M-snap`       | SNAP                                       | `r1 [22] 0x80` | 2 presses                                       | Verified                      |
| 121 | `M-quantize`   | QUANTIZE                                   | `r1 [25] 0x08` | 2 presses                                       | Verified                      |
| 122 | `M-tempo-push` | TEMPO encoder push                         | `r1 [23] 0x04` | 2 presses                                       | Verified                      |
| 123 | `M-tempo-turn` | TEMPO encoder                              | `r1 [3] 0x0f`  | clockwise +3, anticlockwise −3, mod 16          | Verified                      |
| 124 | `M-mic1`       | MIC 1 button                               | `r1 [22] 0x04` | 2 presses; latch `r1 [39] 0x10` flips per press | Verified                      |
| 125 | `M-mic2`       | MIC 2 button                               | `r1 [25] 0x01` | 2 presses; latch `r1 [39] 0x20` flips per press | Verified                      |
|     | `M-cue-vol`    | CUE VOL (2026-11-10, separate raw capture) | `r2 [65..66]`  | 19–4091                                         | Verified by order moved       |
|     | `M-cue-mix`    | CUE MIX (2026-11-10, separate raw capture) | `r2 [63..64]`  | 20–4090                                         | Verified by order moved       |
|     | `M-booth`      | BOOTH (2026-11-10, separate raw capture)   | `r2 [61..62]`  | 11–4082                                         | Verified by order moved       |
|     | `M-main`       | MAIN (2026-11-10, separate raw capture)    | `r2 [59..60]`  | 13–4082                                         | Verified, only field sweeping |

### Right deck: displays and encoders

| #   | Step             | Control                                                                 | Field          | Observed                                   | Status   |
| --- | ---------------- | ----------------------------------------------------------------------- | -------------- | ------------------------------------------ | -------- |
| 126 | `R-disp-1`       | Right display button 1 (NI colour settings in MIDI mode)                | `r1 [12] 0x20` | 2 presses                                  | Verified |
| 127 | `R-disp-2`       | Right display button 2                                                  | `r1 [12] 0x10` | 2 presses                                  | Verified |
| 128 | `R-disp-3`       | Right display button 3                                                  | `r1 [12] 0x01` | 2 presses                                  | Verified |
| 129 | `R-disp-4`       | Right display button 4 (Controller Editor template switch in MIDI mode) | `r1 [14] 0x40` | 2 presses                                  | Verified |
| 130 | `R-disp-5`       | Right display button 5                                                  | `r1 [13] 0x08` | 2 presses                                  | Verified |
| 131 | `R-disp-6`       | Right display button 6                                                  | `r1 [13] 0x10` | 2 presses                                  | Verified |
| 132 | `R-disp-7`       | Right display button 7                                                  | `r1 [13] 0x20` | 2 presses                                  | Verified |
| 133 | `R-disp-8`       | Right display button 8                                                  | `r1 [13] 0x40` | 2 presses                                  | Verified |
| 134 | `R-senc-1-touch` | Right screen encoder 1 touch                                            | `r1 [18] 0x02` | touch and release                          | Verified |
| 135 | `R-senc-1-turn`  | Right screen encoder 1 (wrap16_wide)                                    | `r2 [9..10]`   | 1–997, wraps past 999; clockwise counts up | Verified |
| 136 | `R-senc-2-touch` | Right screen encoder 2 touch                                            | `r1 [18] 0x04` | touch and release                          | Verified |
| 137 | `R-senc-2-turn`  | Right screen encoder 2 (wrap16_wide)                                    | `r2 [11..12]`  | 1–994, wraps past 999; clockwise counts up | Verified |
| 138 | `R-senc-3-touch` | Right screen encoder 3 touch                                            | `r1 [18] 0x08` | touch and release                          | Verified |
| 139 | `R-senc-3-turn`  | Right screen encoder 3 (wrap16_wide)                                    | `r2 [13..14]`  | 2–998, wraps past 999; clockwise counts up | Verified |
| 140 | `R-senc-4-touch` | Right screen encoder 4 touch                                            | `r1 [18] 0x10` | touch and release                          | Verified |
| 141 | `R-senc-4-turn`  | Right screen encoder 4 (wrap16_wide)                                    | `r2 [15..16]`  | 0–999, wraps past 999; clockwise counts up | Verified |
| 142 | `R-browse-touch` | Right browse encoder touch                                              | `r1 [18] 0x20` | 2 touches                                  | Verified |
| 143 | `R-browse-push`  | Right browse encoder push                                               | `r1 [12] 0x08` | 2 presses                                  | Verified |
| 144 | `R-browse-turn`  | Right browse encoder                                                    | `r1 [2] 0xf0`  | clockwise +3, anticlockwise −3, mod 16     | Verified |
| 145 | `R-loop-touch`   | Right loop encoder touch                                                | `r1 [18] 0x01` | touch and release                          | Verified |
| 146 | `R-loop-push`    | Right loop encoder push                                                 | `r1 [15] 0x10` | 2 presses                                  | Verified |
| 147 | `R-loop-turn`    | Right loop encoder                                                      | `r1 [2] 0x0f`  | clockwise +3, anticlockwise −3, mod 16     | Verified |

### Right deck: buttons and pads

| #   | Step         | Control            | Field          | Observed  | Status   |
| --- | ------------ | ------------------ | -------------- | --------- | -------- |
| 148 | `R-back`     | Right BACK         | `r1 [14] 0x80` | 2 presses | Verified |
| 149 | `R-edit`     | Right EDIT         | `r1 [14] 0x01` | 2 presses | Verified |
| 150 | `R-capture`  | Right CAPTURE      | `r1 [14] 0x20` | 2 presses | Verified |
| 151 | `R-fxselect` | Right FX SELECT    | `r1 [12] 0x40` | 2 presses | Verified |
| 152 | `R-fx1`      | Right FX 1 button  | `r1 [12] 0x80` | 2 presses | Verified |
| 153 | `R-fx2`      | Right FX 2 button  | `r1 [13] 0x04` | 2 presses | Verified |
| 154 | `R-fx3`      | Right FX 3 button  | `r1 [13] 0x02` | 2 presses | Verified |
| 155 | `R-fx4`      | Right FX 4 button  | `r1 [13] 0x01` | 2 presses | Verified |
| 156 | `R-on1`      | Right ON 1         | `r1 [14] 0x10` | 2 presses | Verified |
| 157 | `R-on2`      | Right ON 2         | `r1 [14] 0x08` | 2 presses | Verified |
| 158 | `R-on3`      | Right ON 3         | `r1 [14] 0x04` | 2 presses | Verified |
| 159 | `R-on4`      | Right ON 4         | `r1 [14] 0x02` | 2 presses | Verified |
| 160 | `R-deck`     | Right DECK         | `r1 [16] 0x04` | 2 presses | Verified |
| 161 | `R-flux`     | Right FLUX         | `r1 [16] 0x10` | 2 presses | Verified |
| 162 | `R-shift`    | Right SHIFT        | `r1 [17] 0x80` | 2 presses | Verified |
| 163 | `R-sync`     | Right SYNC         | `r1 [17] 0x40` | 2 presses | Verified |
| 164 | `R-cue`      | Right CUE          | `r1 [17] 0x20` | 2 presses | Verified |
| 165 | `R-play`     | Right PLAY         | `r1 [17] 0x10` | 2 presses | Verified |
| 166 | `R-hotcue`   | Right HOTCUE       | `r1 [17] 0x08` | 2 presses | Verified |
| 167 | `R-loop`     | Right LOOP         | `r1 [17] 0x04` | 2 presses | Verified |
| 168 | `R-freeze`   | Right FREEZE/SLICE | `r1 [17] 0x02` | 2 presses | Verified |
| 169 | `R-remix`    | Right REMIX        | `r1 [17] 0x01` | 2 presses | Verified |
| 170 | `R-pad-1`    | Right pad 1        | `r1 [16] 0x08` | 2 presses | Verified |
| 171 | `R-pad-2`    | Right pad 2        | `r1 [16] 0x01` | 2 presses | Verified |
| 172 | `R-pad-3`    | Right pad 3        | `r1 [15] 0x01` | 2 presses | Verified |
| 173 | `R-pad-4`    | Right pad 4        | `r1 [15] 0x02` | 2 presses | Verified |
| 174 | `R-pad-5`    | Right pad 5        | `r1 [16] 0x20` | 2 presses | Verified |
| 175 | `R-pad-6`    | Right pad 6        | `r1 [16] 0x40` | 2 presses | Verified |
| 176 | `R-pad-7`    | Right pad 7        | `r1 [16] 0x80` | 2 presses | Verified |
| 177 | `R-pad-8`    | Right pad 8        | `r1 [16] 0x02` | 2 presses | Verified |

### Right deck: knobs, faders and touchstrip

| #   | Step            | Control                               | Field                                 | Observed                                                   | Status                              |
| --- | --------------- | ------------------------------------- | ------------------------------------- | ---------------------------------------------------------- | ----------------------------------- |
| 178 | `R-fxk-1-touch` | Right FX knob 1 touch                 | `r1 [18] 0x40`                        | touch and release                                          | Verified                            |
| 179 | `R-fxk-1`       | Right FX knob 1                       | `r2 [51..52]`                         | 10–4077                                                    | Verified                            |
| 180 | `R-fxk-2-touch` | Right FX knob 2 touch                 | `r1 [18] 0x80`                        | touch and release                                          | Verified                            |
| 181 | `R-fxk-2`       | Right FX knob 2                       | `r2 [53..54]`                         | 9–4074                                                     | Verified                            |
| 182 | `R-fxk-3-touch` | Right FX knob 3 touch                 | `r1 [19] 0x10`                        | touch and release                                          | Verified                            |
| 183 | `R-fxk-3`       | Right FX knob 3                       | `r2 [55..56]`                         | 10–4077                                                    | Verified                            |
| 184 | `R-fxk-4-touch` | Right FX knob 4 touch                 | `r1 [19] 0x20`                        | touch and release                                          | Verified                            |
| 185 | `R-fxk-4`       | Right FX knob 4                       | `r2 [57..58]`                         | 8–4076                                                     | Verified                            |
| 186 | `R-fader-1`     | Right deck fader 1 (touch + position) | `r2 [25..26]`, `r1 [19] 0x01` (touch) | 10–4076; touch bit never changed                           | Position verified; touch unverified |
| 187 | `R-fader-2`     | Right deck fader 2 (touch + position) | `r2 [27..28]`, `r1 [19] 0x02`         | 9–4077; touch and release                                  | Verified                            |
| 188 | `R-fader-3`     | Right deck fader 3 (touch + position) | `r2 [29..30]`, `r1 [19] 0x04`         | 9–4077; touch and release                                  | Verified                            |
| 189 | `R-fader-4`     | Right deck fader 4 (touch + position) | `r2 [31..32]`, `r1 [19] 0x08`         | 8–4077; touch and release                                  | Verified                            |
| 190 | `R-strip-touch` | Right touchstrip touch                | `r1 [34..35]`                         | 485–495 while touched, 0 on release; no separate touch bit | Verified                            |
| 191 | `R-strip-slide` | Right touchstrip position             | `r1 [34..35]`                         | 1–1024 while touched, 0 on release                         | Verified                            |
