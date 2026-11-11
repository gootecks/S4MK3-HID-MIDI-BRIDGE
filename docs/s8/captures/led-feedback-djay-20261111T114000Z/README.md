---
summary: "Djay's feedback on the virtual port with mapping r8 at 11:39Z on 2026-11-11, three minutes after LED feedback run 2, with a picture of djay's window."
type: capture
issues: [22, 23]
---

# Djay feedback after LED feedback run 2

Taken with the `feedback-capture` example on the port `Kontrol S8 MIDI Full`, mapping
`Kontrol S8 Bridge r8`. The S8 was not opened and NIHardwareAgent was running. Nothing
was driven in djay: the capture holds what djay sent when the port appeared, and what it
sent on its own for 45 seconds after that.

- `feedback.txt.gz`: the raw capture, 11:39:25Z to 11:40:09Z.
- `djay-window.png`: djay's window at the same time, in its Two Decks view.

What it shows:

- Decks A and B had a track loaded; decks C and D had none (loaded notes 2 and 3 on
  channel 7 were 0, and all their cue tokens were 0).
- Deck A sent tokens 1-8 for cues 1-8. Deck B sent the same, with 0 for its unset cue 5.
- In the picture the cue points of both decks are, from 1 to 8: red, orange, blue,
  yellow, green, magenta, cyan, purple.

Used in [led-feedback-causes.md](../../led-feedback-causes.md#findings-after-run-2).
