---
summary: "S8 deck LED open questions Q1-18 and Q34-39 with the answers the 2026-11-11 run gave."
type: reference
issues: [21, 22, 23]
---

# Kontrol S8 deck LED questions

Answers are recorded where the plan ran, in
[led-output-validation.md](led-output-validation.md). The map and sources are in
[led-output.md](led-output.md). Questions 19-33 and 40-41 are in the [mixer
note](led-output-mixer.md#open-questions-for-the-hardware-run). Answered
questions keep their text and carry the step id and the observation.

- **Q1.** Does `[80]` plus 118 zeros return 119 from a hidapi write? (yes/no)
  Answered, T0: yes, 119 returned.
- **Q2.** Does that write change any LED? (yes/no) Answered, T0: yes, most
  deck buttons went dark.
- **Q3.** After PLAY-only then CUE-only reports, is PLAY dark? (yes/no)
  Answered, T2: yes.
- **Q4.** Does a write to `0x80` change anything on the right deck or mixer?
  (yes/no) Open. T4 showed a `0x81` write changing nothing on the left; the
  `0x80` direction was not asked.
- **Q5.** Is offset 57 SYNC lit red or green? (red/green) Answered, T5: red.
- **Q6.** Is offset 56 a different LED from 57? (yes/no) Answered, T5 and T6:
  57 lit SYNC red with no green; 56 lit the left SYNC green. Different colour
  LEDs; whether they are one button was not asked.
- **Q7.** For pad 1, which colour does byte 0, byte 1 and byte 2 give?
  (red/green/blue) Answered, T8a-T8c: red, green, blue.
- **Q8.** Does `0xFF` on a pad or strip LED look different from `0x7F`?
  (yes/no) Answered for the pad, T9b: same. Strips not tested.
- **Q9.** Which end of the strip does offset 68 light? (left/right) Answered,
  T10a: left.
- **Q10.** Is offset 80 the middle strip LED? (yes/no) Answered, T10b: yes,
  middle.
- **Q11.** What colour is the 68-92 bank, and what colour is 93-117? (one
  word each) Answered, T10a-T11b: blue and orange. Written together, T12:
  the top row of the left touchstrip is purple and the bottom row off.
- **Q12.** Do offsets 29-36 each light a different display-side button?
  (yes/no) Open. T7b lit all eight together: 4 on the left, 4 on the right
  of the left deck display.
- **Q13.** Is 29-32 the left cluster? (yes/no) Open: T7b wrote 29-36 together.
- **Q14.** Is offset 25 the FX-unit on button or the first FX slot? (on/slot)
  Open. T7a lit FX Select and the buttons below FX1-FX4 as a group.
- **Q15.** Do offsets 44, 46, 48 and 50 (white) and 45, 47, 49 and 51 (blue)
  light separate LEDs? (yes/no) Open. T7e lit hotcue, loop, freeze and remix,
  all white, with all eight offsets at once.
- **Q16.** Do `0x80` and `0x81` offsets 53 and 54 light the deck selector LEDs,
  white and blue, for the outer decks A/B and C/D? (yes/no) Open. T7f lit
  Deck and Shift, both white, with 53-55 at once; no blue reported.
- **Q17.** Does `0x00` extinguish a 0x14-class LED (FREEZE, REMIX, LOOP, SYNC)?
  (yes/no) Open: no deck step sent `0x14` (Q36).
- **Q18.** After zeros and an agent restart, do all LEDs return without further
  action? (yes/no) Answered, both runs: yes, panel labels and LEDs back.
- **Q34.** Inside each lit group (24-28, 29-36, 37-43, 44-51, 53-55, 60-67),
  which offset lights which LED? (offset per LED) Open: only whole groups
  were written (T7a-T7h).
- **Q35.** Which single-byte offsets of 44-51 and 53-54 give white and which
  blue? (offset per colour) Open: T7e and T7f showed white only.
- **Q36.** Is `0x14` a visible dim level on deck LEDs? (yes/no) Open: no deck
  step sent it.
- **Q37.** Why did T12 (`68-117=7f`) light only the top row of the left
  touchstrip, in purple, with the bottom row off? (cause) Open.
- **Q38.** Which byte, if any, controls the deck CUE, filter and arrow LEDs
  that stayed lit after the all-zero writes of T0? (offset, report or none)
  Open.
- **Q39.** Do the right deck offsets other than PLAY (59) match the left?
  (yes/no) Open: T4 wrote only `0x81` offset 59.
