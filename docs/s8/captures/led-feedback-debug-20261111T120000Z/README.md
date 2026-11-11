---
summary: "Feedback capture of djay on the virtual port only, mapping r7, 2026-11-11 about 10:54Z: no S8 opened."
type: capture
issues: [22, 23]
---

# S8 LED feedback: djay capture with mapping r7

Taken on 2026-11-11 with the `feedback-capture` example on the virtual port
`Kontrol S8 MIDI Full` only. The S8 was not opened. The mapping was
`Kontrol S8 Bridge r7`. The actions ran at about 10:54Z. The time stamp in the folder
name is not the real capture time. The record is
[led-feedback-validation.md](../../led-feedback-validation.md#causes).

- `initial.txt.gz`: cargo output and `DJAY_FEEDBACK` lines from the start, at 10:52Z, with
  the state djay had then.
- `actions.txt.gz`: `DJAY_FEEDBACK` lines while the actions were taken.
- `actions-marks.txt`: the times (UTC, `HH:MM:SS`) of the actions: `fx1on`, `fx2on`,
  `fx1off`, `fx2off`, `slipon`, `slipoff`, `play`, `pause` and `gotobeginning`.

It shows the causes of the hot cue pads (127 on notes 40-47), the progress bar
(`songProgress` as 0 or 127 only), the FX 4 follow of FX 1-3 and the CUE toggling.
