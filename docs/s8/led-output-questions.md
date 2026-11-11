---
summary: "S8 deck LED open questions Q1-18 and Q34-39 with the answers the 2026-11-11 probe run and two djay-driven LED checks gave."
type: reference
issues: [21, 22, 23]
---

# Kontrol S8 deck LED questions

Answers are recorded where the plan ran, in
[led-output-validation.md](led-output-validation.md). The two djay-driven LED
checks of 2026-11-11 (run 1 with mapping r7, run 2 with mapping r8) are
recorded in [led-feedback-validation.md](led-feedback-validation.md). An
answer from them names its source as "djay-driven LED check, run N, row Lx".
The map and sources are in [led-output.md](led-output.md). Questions 19-33 and
40-41 are in the [mixer note](led-output-mixer.md#open-questions-for-the-hardware-run).

Answered questions keep their text and carry the step id and the observation.
An answer is what the user reported, quoted as typed; a question the answers do
not fully cover stays open and says what is missing.

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
  white and blue, for the outer decks A/B and C/D? (yes/no) Open in part. T7f
  lit Deck and Shift, both white, with 53-55 at once; no blue reported.
  Answered for decks A and C, djay-driven LED check, run 1, row L15: "A is blue
  and C is white". Missing: the DECK colour of decks B and D. Run 2, row L16
  answered the right side as a whole, without a colour (see Q39), and run 1,
  row L15 did not name the offset written.
- **Q17.** Does `0x00` extinguish a 0x14-class LED (FREEZE, REMIX, LOOP, SYNC)?
  (yes/no) Open. No deck step in the probe run sent `0x14` (Q36).
  - In the djay-driven LED checks the bridge sent it. Row L22 is an observation
    after the bridge stopped, not a test of one `0x00` write.
  - Run 1, row L22: "everything is off except the FX 1-2 of all decks, snap,
    quantize, mic 1 mic 2, all 4 cue buttons and all 4 filter buttons".
  - Run 2, row L22: "FX 1-2 of all decks, mic 1 and mic 2, snap, quantize, all 4
    CUEs, all 4 filter buttons".
  - FX 1-2 are on the plan's dim list (row L1) and stayed lit in both runs. The
    rows neither show nor exclude that a zero clears a dim LED.
  - Missing: a step that writes `0x00` to a LED shown dim, with the bytes logged,
    and the cause for FX 1-2.
- **Q18.** After zeros and an agent restart, do all LEDs return without further
  action? (yes/no) Answered, both runs: yes, panel labels and LEDs back.
- **Q34.** Inside each lit group (24-28, 29-36, 37-43, 44-51, 53-55, 60-67),
  which offset lights which LED? (offset per LED) Open in part: the probe run
  wrote only whole groups (T7a-T7h). The djay-driven LED checks drove single
  LEDs of some groups; none of them reports an offset, only whether the LED the
  bridge meant lit.
  - 24-28, FX 1-4: row L13, run 1: "FX 1-3 turn on, but FX4 goes on and off when
    any of them are turned on or off". FX 1-3 each lit alone and FX 4 followed
    the unit, as the plan expects. FX Select is not reported alone.
  - 37-43, ON 1-4: row L14, run 1: "ON LED is dim when the stem is on and bright
    when the stem is muted". One answer for the four buttons; no LED is named,
    BACK, CAPTURE and EDIT have no row. Not covered.
  - 44-51, mode buttons: HOTCUE and LOOP are told apart. Run 1, row L1: "hotcue
    is brightest, loop, freeze, and remix are dim"; row L10: "shift + hot cue
    also turns the LOOP button brighter but has no effect on the HOTCUE button".
    Run 2, row L9: "shift + hotcue turns the pads green and makes LOOP bright
    blue but hotcue stays dim". FREEZE and REMIX are never reported as a button
    LED, only as pad colours. Not covered.
  - 53-55, DECK and SHIFT: row L15, run 1: "A is blue and C is white"; row L21,
    run 1: "yes nothing else changes" (left SHIFT lit while held). Both lit
    alone as expected. The DECK pair is as in Q16.
  - 60-67, loop ring: row L8, run 1: "the ring is blue on deck A and white on
    deck C". No segment is reported alone. Not covered.
  - 29-36 display buttons: no deck row drives them. Not covered.
- **Q35.** Which single-byte offsets of 44-51 and 53-54 give white and which
  blue? (offset per colour) Open in part: T7e and T7f showed white only. Colours
  seen since, djay-driven LED check: run 1, row L15: "A is blue and C is white"
  (DECK); run 2, row L9: "shift + hotcue turns the pads green and makes LOOP
  bright blue" (LOOP button). No row reports which byte was written, so no offset
  is tied to a colour. Missing: that pairing, and the colour of HOTCUE, FREEZE
  and REMIX when lit.
- **Q36.** Is `0x14` a visible dim level on deck LEDs? (yes/no) Answered,
  djay-driven LED check, run 1, rows L1, L6 and L8: yes, dim LEDs were seen and
  differ from full.
  - L1: "hotcue is brightest, loop, freeze, and remix are dim".
  - L6: "pressing sync makes the sync button go from green to bright green".
  - L8: "it seems like there's only two settings, bright and dim".
  - Run 2, row L9: "hotcue is dim".
  - The notes do not name the byte; `0x14` is `DIM` in `src/s8/leds/map.rs`.
    Not tested: other levels.
- **Q37.** Why did T12 (`68-117=7f`) light only the top row of the left
  touchstrip, in purple, with the bottom row off? (cause) Open. Seen since,
  djay-driven LED check, only the top row ever lit:
  - Run 1, row L5: "the top row of the touchstrip is blue, the bottom row is
    off". Row L18: "the top bar is all the way on the bottom bar is off".
    Row L20: "the dot is purple and it follows my finger".
  - Run 2, row L17: "top row". Row L20: "orange dot appears when i touch the
    touchstrip" and "top row only" and "bottom row has yet to come on ever".
    Rows L17-L20 show a blue progress bar and an orange finger dot, both on
    the top row.
  - Run 1 saw a purple dot, run 2 an orange one. Between the runs commit
    `6225f68` of PR #121 made the bridge blank the blue LED under the orange
    dot. From the code: in run 1 both LEDs of that cell were lit.
  - The user's remark after run 2, a belief about the hardware and not a
    measurement:

    ```text
    if there's a bottom row of lights for the touchstrip i've never seen them turn on.  but i don't think there are.  i think there's just the top row with blue or orange
    ```

  Missing: a test of the bottom row, or the hardware fact whether it exists; the
  cause of purple in T12 is not shown.

- **Q38.** Which byte, if any, controls the deck CUE, filter and arrow LEDs
  that stayed lit after the all-zero writes of T0? (offset, report or none)
  Open.
- **Q39.** Do the right deck offsets other than PLAY (59) match the left?
  (yes/no) Answered, djay-driven LED check, run 2, row L16: yes, "yes works as
  expected". One answer for all right-side controls the row listed (PLAY, CUE,
  SYNC, FLUX, each mode button, the pads, each FX button, DECK) and for the left
  staying unchanged; no control is reported alone. Run 1, row L16 had said "yes
  only play on the right is established and it's independent of the left".
