---
summary: "Causes of the rows that differed in the attended LED feedback runs of 2026-11-11, with the evidence for each, and the check of the fixes against djay without the S8."
type: validation
issues: [22, 23]
---

# S8 LED feedback: causes

The rows and the user's answers are in
[led-feedback-validation.md](led-feedback-validation.md). This note holds what was found
about them afterwards.

## Causes

The causes of the run 1 rows, with the evidence level of each. Run 2 on the S8 confirmed
the fixes for the cue colour, progress bar and purple dot entries. Evidence levels:
capture, code and test, checked against djay without the S8, inference, unexplained.

- Hot cue pads dark (L3, L9, L11). Evidence: capture. With mapping r7 djay sent value
  127 on notes 40-47 (channels 1-4) for a set cue point and 0 for an unset one. The
  bridge reads a colour token 1-9 there, so 127 lit nothing. r7 had an empty `userInfo`.
  Fix: mapping r8 carries `userInfo` with `padColorOff` 0, `padColorWhiteActive` 9 and
  `padColorsActive` 1-8. Checked against djay without the S8: notes 40-47 carry tokens
  1-8 for set cue points on all four decks and 0 for the unset cue 5 of deck B.
  Run 2 confirmed the fix on the S8: L11 "yes they're the expected colors and then off
  when you delete the hotcue"; L3 "te pads are the right color except hotcue 8".
- Strip bar full at any progress (L5, L17, L18, L19). Evidence: capture. With r7 djay
  sent `songProgress` (CC 8-11 on channel 7) as 0 or 127 only. Fix: r8 marks these
  outputs `controlType: control`. Checked against djay without the S8: deck 1 playing
  from the start for 25 s sent 0, 1, 3, 4 ... 22 in steps; deck 2 sent 61-66 over 8 s.
  Run 2 confirmed the fix on the S8: L18 "about half are on when it's at halfway"; L19 "24
  out of 25, it grows as the playhead moves"; L17 "0 LEDs are lit at progress 0".
- Purple dot (L20). Evidence: code and test. The orange and blue LED banks share the
  top row of the strip, and lit together they look purple. Fix: the bridge blanks the
  blue LED under the orange dot, with a unit test. "Stays on after release" was the
  full bar of the cause above.
  Run 2 confirmed the fix on the S8: L20 "orange dot appears when i touch the
  touchstrip. it follows my finger and goes out on release. top row only."
- FX 4 on with FX 1-3 (L13). Evidence: capture. This is not a fault. FX 4 is bound to
  `turntable{deck}.fxActive`, which is on while any of FX 1-3 of that deck is on
  (catalog, [mapping.md](mapping.md)); the capture shows it following. The Expected text
  of L13 in the plan was wrong; FX 4 behaved as mapped. Fix: none; the Expected text
  stays as written above as a record of the plan.
- HOTCUE button after Shift + HOTCUE (L10). Evidence: inference from the step order.
  The sequence is FREEZE, then Shift + HOTCUE, then REMIX, so HOTCUE was already dim
  when the question was asked. "No effect on the HOTCUE button" fits correct
  behaviour. L10 is Not settled. Fix: none.
- FREEZE pads "teal" (L9, L10). Evidence: code. The bridge sends `Rgb(0, 80, 127)`;
  the plan's "dim blue" was loose wording. Fix: none.
- CUE flashing on a paused deck (L3, L6). Evidence: capture. djay toggles note 1 about
  every 500 ms while a deck is paused away from its cue point. This is djay behaviour,
  not a bridge fault. Fix: none.
- FLUX "flashes" (L7). Evidence: unexplained. The capture shows a steady 127. Fix: none.
- L16 right side. Evidence: unexplained, not settled. The bridge log of run 1 shows only
  `right_play`, `right_cue`, `right_pad_1`, `right_deck`, `right_remix`, the strip,
  faders and dials on the right. Fix: none; the second run asks it again.

All four djay decks had a track loaded during the djay check (loaded notes 0-3 on
channel 7 all 127). Evan's S8 mapping (`Evan's Traktor Kontrol S8 1.1.djayMidiMapping`),
offered as a source for hot cue colours, has no `userInfo` and no outputs, so it carries
no colour settings.

## Check against djay without the S8

Mapping r8 was selected in djay for the `Kontrol S8 MIDI Full` port. The
`feedback-capture` example held the virtual port only: the S8 was not opened and
NIHardwareAgent kept running. osascript drove djay through its Deck 1 and Deck 2 menus
(Go to Beginning, Play / Pause). Results are in [Causes](#causes): cue tokens 1-8 on
notes 40-47 for set cues and 0 for the unset one, and progress that rises in steps. The
capture is
[led-feedback-djay-20261111T110300Z](captures/led-feedback-djay-20261111T110300Z/README.md).

This does not make any row checked on the S8. A row counts as checked only from what
the user reports on the S8. Run 1, with mapping r7, was captured the same way on the
virtual port at about 10:54Z in
[led-feedback-debug-20261111T120000Z](captures/led-feedback-debug-20261111T120000Z/README.md).

## Findings after run 2

Looked at after run 2, from its bridge log, the code and a capture of djay's feedback
taken on the virtual port at 11:39Z, three minutes after the run
([led-feedback-djay-20261111T114000Z](captures/led-feedback-djay-20261111T114000Z/README.md)).
Nothing here was asked again on the S8.

- Mode buttons (L9). Evidence: code, and the button presses in the bridge log. No fault
  was found and no code changed. HOTCUE, FREEZE and REMIX select their pad mode. Shift +
  HOTCUE selects the loop pads: LOOP is then the full button, HOTCUE is dim and the pads
  rest dim green. That is what the answer reports. Plain LOOP is not a pad-mode button:
  it sends note 26, `looper.toggleLooperShown`, and changes no LED. The log shows Shift
  held only around two HOTCUE presses; each plain HOTCUE press selected the hot cue pads.
  Not settled: the answer does not say which button was full in the HOTCUE, FREEZE and
  REMIX modes, nor whether LOOP opened djay's Looper view. The runner's L9 text now says
  what LOOP does and asks for the full button.
- Pads off on deck C (L15). Evidence: inference. In the capture at 11:39Z decks C and D
  had no track loaded (loaded notes 2 and 3 on channel 7 were 0, and so were all their
  cue tokens), and djay was in its Two Decks view. Pads off and a CUE button that does
  not blink fit an empty deck. The capture is from after the run, not from the moment of
  the row. The answer does not give the DECK colour; run 1 did (L15).
- Hot cue 8 (L3). Evidence: capture and the user's answer after run 2. At 11:08Z djay sent
  token 6 for cue 8 on decks A, B and D, the same token as cue 6. At 11:39Z, after the
  user had changed the colour to purple, it sent token 8 on decks A and B, and
  `djay-window.png` shows cue 8 purple on both. The colours djay showed then, cue 1 to 8:
  red, orange, blue, yellow, green, magenta, cyan, purple. The answer: "pad 8 was magenta
  like pad 6 in both. but the tracks' hot cues were set that way." So the pad showed the
  colour the cue had in djay, and the bridge needs no change.
- LEDs dim after the stop (L22). Evidence: inference from the code. Both runs list the same
  controls. Snap, quantize, the mic buttons, the four channel CUE buttons and the four
  filter buttons are on the mixer; FX 1-2 would be too if the answer means the channels'
  FX assign buttons, which it does not say. The bridge writes only the deck reports `0x80`
  and `0x81` (`leds.rs`, `report_id`), so its clear cannot reach a mixer LED. Whether the
  S8 lights those LEDs on its own without NIHardwareAgent was not tested. The mixer report
  is research in #24.
