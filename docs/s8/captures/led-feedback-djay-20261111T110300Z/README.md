---
summary: "Feedback capture of djay on the virtual port only, mapping r8, 2026-11-11: cue tokens and progress steps."
type: capture
issues: [22, 23]
---

# S8 LED feedback: djay capture with mapping r8

Taken on 2026-11-11 with the `feedback-capture` example on the virtual port
`Kontrol S8 MIDI Full` only. The S8 was not opened and NIHardwareAgent kept running.
The mapping was `Kontrol S8 Bridge r8`. osascript drove djay through its Deck 1 and
Deck 2 menus. The record is
[led-feedback-causes.md](../../led-feedback-causes.md#check-against-djay-without-the-s8).

- `feedback.txt.gz`: the raw capture.
- `actions-marks.txt`: the UTC times of the scripted menu actions (Go to Beginning and
  Play / Pause on deck 1 and deck 2).

Results:

- Notes 40-47 carry tokens 1-8 for set cue points on all four decks, and 0 for the
  unset cue 5 of deck B.
- `songProgress`: deck 1 playing from the start for 25 s sent 0, 1, 3, 4 ... 22 in
  steps; deck 2 sent 61-66 over 8 s.

This does not show anything on the S8.
