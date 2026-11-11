---
summary: "Second attended run of the S8 deck LEDs driven by --s8-leds on 2026-11-11, rows L3 L7 L9 L11 L15-L20 and L22-L24: notes, events, snapshots and bridge log."
type: capture
issues: [22, 23]
---

# S8 LED feedback: second attended run

Taken on 2026-11-11 with the mapping `Kontrol S8 Bridge r8` and the debug build at
`d49a291`. Rows asked: L3, L7, L9, L11, L15, L16, L17, L18, L19 and L20, then the exit
rows L22 to L24. The bridge exited with status 0 with `input_events=1450`,
`led_writes=7678`, `midi_messages=1109` and `feedback_messages=17498`. The record is
[led-feedback-validation.md](../../led-feedback-validation.md#second-run).

- `notes.txt`: every question and the user's answer, unchanged.
- `events.txt`: runner marks.
- `snapshots.txt`: process snapshots.
- `bridge.log.gz`: INFO log of the bridge. Feedback messages are logged at debug level
  and are not in it.
