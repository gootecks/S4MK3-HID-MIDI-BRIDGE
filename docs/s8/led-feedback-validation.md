---
summary: "Plan and record of the attended run of 2026-11-11 for the S8 deck LEDs driven by --s8-leds from djay feedback (led-feedback-check.sh)."
type: validation
issues: [22, 23]
---

# S8 LED feedback validation

Validation record for #22 and #23. `--device s8 --midi full --s8-leds` lights
the deck, pad and touchstrip LEDs of the S8 from djay's MIDI feedback and from
the bridge's own control state. What each LED should show is in
[mapping.md](mapping.md#led-feedback). The report layout was probed on its
own in the LED output check; this check is the first run driven from djay.
The answers of the run of 2026-11-11 are under [Answers](#answers).

## Status

Run 1 on 2026-11-11 with the mapping `Kontrol S8 Bridge r7` and the debug build of this
branch at commit `b2eafc2`. All 24 rows were answered. The run folder is
[led-feedback-20261111T101037Z](captures/led-feedback-20261111T101037Z/README.md).
The bridge exited with status 0, with `S8_LEDS_DISABLED` 0 and
`S8_LEDS_CLEAR_FAILED` 0. Its closing counts were `input_events=1249`,
`led_writes=10886`, `midi_messages=1055` and `feedback_messages=30587`.

Several rows did not match the expected result; see [Results](#results) and
[Causes](#causes). Fixes for the causes are in the worktree (mapping r8 and the bridge).
None of them has been seen on the S8 yet: they were checked against djay without the S8
only. The second run on the S8 is not run yet; see [Second run](#second-run).
Issues #22 and #23 stay open.

An earlier attempt, `led-feedback-20261111T095724Z`, was ended with Ctrl-C at the
mapping prompt, before any row. The bridge exited with status 0 and NIHardwareAgent
was restored.

## Scope

What the bridge writes to the S8:

- Output reports `0x80` (left) and `0x81` (right) only, 119 bytes each: the
  report ID and 118 payload bytes. A write replaces the whole side.
- A report is written only when it differs from the last one written for that
  side, and at most once every 10 ms per side (`MIN_INTERVAL` in
  `src/s8/led_output.rs`).
- While `--s8-leds` is on, the HID read waits 5 ms instead of 100 ms so that
  feedback is applied while the controls are idle.
- At a normal stop or Ctrl-C (SIGINT) both sides are written all zero.

What is not touched:

- Interface 4 and the screens.
- Feature reports, including `0xF3` and `0xF4`.
- The mixer report `0x82`. Its LEDs, and the ones that no `0x80` or `0x81`
  byte controls, stay as the firmware leaves them.

The runner `led-feedback-check.sh` writes nothing to the S8 itself. It builds
the bridge, installs a mapping, stops NIHardwareAgent, runs the bridge and
asks the rows.

## Prerequisites

- One S8 connected, Traktor, Maschine and Komplete Kontrol quit, and no other
  bridge, `S8Host` or `S8Preview` running. The runner refuses otherwise.
- djay running, with decks A to D empty, two test tracks in the library to
  load on A and B, and a sampler slot with a sample for L12. A stem track is
  needed for L14.
- Controller Editor open with both panels showing their labels. A run started
  with it closed once left the S8 dark; see
  [translator-validation.md](translator-validation.md#s8-dark-at-start).
- `docs/s8/captures/` writable. The run folder is `led-feedback-<UTC>/`.

## How to run

`mise run s8:leds` runs the plan from L1. `mise run s8:leds L12` starts at L12.
`mise run s8:leds only L3 L7` runs just those rows, plus the exit rows.
The runner does this in order:

1. It checks for one S8, one genuine NIHardwareAgent and no rival bridge.
2. It builds the bridge with `cargo build --locked` so that no build runs while
   the agent is stopped.
3. It generates the mapping into a temporary folder and installs it as
   `Kontrol S8 Bridge rN.djayMidiMapping` in `~/Music/djay/MIDI Mappings/`, with
   N one above the highest installed N. It refuses to overwrite a file. When
   the highest installed revision is the same file, it uses that one instead.
4. It asks for a left PLAY press, stops NIHardwareAgent with SIGKILL and asks
   again.
5. It starts the bridge with `--device s8 --midi full --s8-leds --until-stopped`.
   Select the new mapping for `Kontrol S8 MIDI Full` in djay, then press Enter.
6. It asks the rows one at a time, printed as a block with DO and EXPECT. Each answer
   goes unchanged into `notes.txt` behind the row id.
7. It stops the bridge with SIGINT, which clears the LEDs, and asks L22.
8. It restores NIHardwareAgent and asks L23 and L24.

The log lines of interest are `S8_LEDS_ENABLED` at start, `S8_LEDS_DISABLED`
after a failed or short write, and `S8_LEDS_CLEAR_FAILED` after a failed clear.
The bridge logs `led_writes` and `feedback_messages` in its closing line. The runner
counts the three markers after the bridge stops and notes them with the counts as
`Bridge counts`. The bridge log is written to `bridge.log` in the run folder only.

## Stop, abort and recovery

- Answer `stop` at any row to skip to the exit rows.
- The runner stops the rows when the bridge exits, or when the log shows
  `S8_LEDS_DISABLED` or `S8_LEDS_CLEAR_FAILED`. The controls keep sending MIDI
  after `S8_LEDS_DISABLED`; the LEDs stay off until the bridge is restarted.
- Ctrl-C and SIGTERM to the runner send SIGINT to the bridge and then restore
  NIHardwareAgent. The bridge clears the LEDs on that SIGINT.
- A closed terminal also restores NIHardwareAgent, but it can end the bridge
  before the clear, and the LEDs then stay as last written.
- If the S8 is dark afterwards, open Controller Editor and start
  NIHardwareAgent again with
  `open -g "/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"`.
- The installed mapping stays in djay's folder. Delete it by hand if it is not
  wanted.

## Plan

Rows L1 to L21 run while the bridge runs, L22 after it stops and L23 and L24
after NIHardwareAgent is back. Points that the output check left unsettled
are marked "open in the output check": the order inside multi-byte groups, the
dim level on deck LEDs and the bottom row of the touchstrip. Those questions
ask for a description.

| Row | Action                                                                                                                                             | Expected                                                                                                                                                  | Result          |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| L1  | Nothing is loaded on decks A to D. Look at both sides and touch nothing.                                                                           | PLAY, CUE, FLUX off; DECK full; HOTCUE full; the other mode buttons, SYNC, ring, FX and ON dim; pads and strips dark. Dim level open in the output check. | Not settled     |
| L2  | Look at the LEDs the bridge does not write: filter on and the arrows at the top of each deck, the mixer CUE buttons, MIC 1, MIC 2, SNAP, QUANTIZE. | They stay as the firmware leaves them. The bridge writes only `0x80` and `0x81`, so it does not change them.                                              | Not settled     |
| L3  | In djay load one test track on deck A and one on deck B. Do not play.                                                                              | Left shows A, right shows B. PLAY off; strip blue bank dark at progress 0; pads dark unless the track has saved cues, which light in their cue colours.   | Differs         |
| L4  | Press left PLAY to play deck A, then again to pause it.                                                                                            | PLAY full while A plays, off when paused (root `playing` output).                                                                                         | Matches         |
| L5  | Pause deck A at its start. Hold left CUE, then release it. Do not hold Shift.                                                                      | CUE full while it is held, from the local state and from djay's CUE note. After the release it follows djay's CUE feedback.                               | Differs         |
| L6  | Play deck B, play deck A, press left SYNC to switch sync on, then again to switch it off.                                                          | SYNC (green LED) full with sync on, dim with sync off. The red SYNC LED stays off. Open in the output check: the dim level.                               | Matches in part |
| L7  | Press left FLUX, then again.                                                                                                                       | FLUX full while slip is on, off otherwise.                                                                                                                | Differs         |
| L8  | Press the left loop encoder to start a loop on deck A, then again to end it.                                                                       | All four ring segments full while the loop is active, dim otherwise: blue on decks A and B, white on C and D.                                             | Matches         |
| L9  | On the left side press HOTCUE, FREEZE, REMIX, then Shift + HOTCUE.                                                                                 | The button of the active pad mode is full and the other three are dim. Shift + HOTCUE makes LOOP the full one.                                            | Not settled     |
| L10 | On the left side select FREEZE, then Shift + HOTCUE, then REMIX, and look at the eight pads each time. Do not press a pad.                         | FREEZE: all pads dim blue. Shift + HOTCUE: all pads dim green. REMIX: all pads dim orange. Dim is the lit colour divided by 6.                            | Not settled     |
| L11 | Select HOTCUE. On deck A set a hot cue on pad 1 and pad 2, then delete the cue of pad 2 in djay.                                                   | Set pads light in djay's cue colour (tokens 1-8 from the S4 bridge's default colours, 9 white); unset pads are dark; the deleted pad goes dark.           | Differs         |
| L12 | Select REMIX. Press sampler pad 1 to play its sample, then stop it. The slot needs a sample.                                                       | The pad is orange while its sample plays and dim orange when it stops. Deck A and deck C read the same sampler bank, B and D the other.                   | Matches         |
| L13 | Press left FX 1, 2, 3 and 4 on, then off.                                                                                                          | Each FX LED is full while its effect is on, dim while off. The side shows the unit's target deck, chosen with the mixer FX buttons.                       | Differs         |
| L14 | Needs a stem track on deck A. Press left ON 1, 2, 3 and 4 to mute and unmute stems.                                                                | Each ON LED is full when djay reports that stem muted, dim otherwise.                                                                                     | Matches         |
| L15 | Press left DECK to switch to deck C, look at the left side, then press DECK again.                                                                 | On C the DECK LED is white; PLAY off, pads dark, ring dim. Back on A it shows A again. Group order open in the output check.                              | Not settled     |
| L16 | Repeat on the right side with deck B: play, hold CUE, SYNC, loop, FLUX, FX 1, a pad-mode button and a hot cue pad.                                 | The right side behaves as the left with its own report `0x81`, independent of the left. Only PLAY on the right is established.                            | Not settled     |
| L17 | Put deck A at the very start of its track (progress 0) and look at the left touchstrip.                                                            | The blue bank is dark at progress 0; one LED once progress is above 0. Open in the output check: which row of the strip the blue bank lights.             | Not settled     |
| L18 | Seek deck A to about half of the track and look at the left strip.                                                                                 | A blue bar from the physical left end, about 12 or 13 of 25 LEDs.                                                                                         | Differs         |
| L19 | Seek deck A to its last seconds, then let it play on.                                                                                              | The bar grows to 24 or 25 LEDs near the end. Open in the output check: whether progress covers the whole track.                                           | Not settled     |
| L20 | Touch the left strip at its left end without moving, slide to the middle and to the right end, then release.                                       | No orange dot while still; one orange dot at the finger once it moves; out on release. Blue bar stays. Bottom row open in the output check.               | Differs         |
| L21 | Hold left Shift, then release it.                                                                                                                  | SHIFT full while held, dim otherwise. Nothing else on that side changes.                                                                                  | Matches         |
| L22 | The bridge has stopped with Ctrl-C. NIHardwareAgent is still stopped. Look at both sides.                                                          | Every LED the bridge owns is dark on both sides: buttons, pads, loop ring, strips. The LEDs of L2 are as before.                                          | Not settled     |
| L23 | NIHardwareAgent has been started again. Look at both panels and the LEDs.                                                                          | Controller Editor labels on both panels and the template's LEDs are back.                                                                                 | Not settled     |
| L24 | Press left PLAY once, then display button 1 or 4.                                                                                                  | Djay reacts to PLAY through the Controller Editor template; the NI action of the display button works.                                                    | Matches         |

### Answers

The answers as the runner wrote them to `notes.txt`, unchanged. The Result column of the
table above sorts each one; [Results](#results) says why.

```text
L1: hotcue is brightest, loop, freeze, and remix are dim.  cue, play and hotcues are off.  deck is pretty bright.  everything else is dimmer
L2: they're all lit
L3: all the hot cues are off.  right deck cue is flashing blue, but the left deck's cue is solid blue.  the controller editor labels show 8 hot cues for deck A but only 5 for deck b but deck b has 7 hot cues and only #5 is empty.
L4: yes
L5: the top row of the touchstrip is blue, the bottom row is off.  cue is solid blue and when i hold it, the green play button comes on and then turns off when i let go
L6: play button is green on both decks.  pressing sync makes the sync button go from green to bright green.  pressing play to pause turns off the play button and makes the cue button flash blue.  sync never turns red
L7: yes it flashes orange when slip is on
L8: the ring is blue on deck A and white on deck C.  enabling the loop makes it brighter, disabling it makes it dimmer, but it seems like there's only two settings, bright and dim
L9: hotcue - all hotcues are off.  loop - looper opens but no change in color on the pads.  freeze - hotcues turn teal.  remix - sampler toggles on and pads turn orange.  freeze also enables slicer mode
L10: shift + hotcue - pads turn green.  remix turns the pads orange.  shift + hot cue also turns the LOOP button brighter but has no effect on the HOTCUE button.
L11: none of the pads come on.  deleting a hot cue and re-adding it doesn't make the pads come on either
L12: yes the sample plays, the pad lights up, then it dims. A and C have the same samples and B and D also have the same samples.  switching to deck A and Deck b turns the top row of the touchstrip blue and then it turns off when you switch decks
L13: FX 1-3 turn on, but FX4 goes on and off when any of them are turned on or off.  so fx4 is on when any of the other ones are on and then turns off when any of them turn off.
L14: ON LED is dim when the stem is on and bright when the stem is muted
L15: play doesn't come on, but A is blue and C is white.  sync stays green the whole time
L16: yes only play on the right is established and it's independent of the left
L17: yes it's off when progress is 0 and turns blue when it's above 0
L18: the top bar is all the way on the bottom bar is off.  it starts on the left but goes all the way across
L19: it's all the way on and then when the track finishes it turns off.
L20: the dot is purple and it follows my finger.  when i release, it goes back to blue and stays on
L21: yes nothing else changes
L22: everything is off except the FX 1-2 of all decks, snap, quantize, mic 1 mic 2, all 4 cue buttons and all 4 filter buttons.  everything else is off
L23: the Controller Editor labels have  evan's hotcues s8 template on both.  but on the right deck, only 5 hot cues are on but the track has 7 total (hotcue 5 is off on the pad, but on on the display)
L24: yes buttons 1 and 4 both work as expected to switch colors or switch templates
```

The question the runner asks for each row is in `plan()` in
`led-feedback-check.sh`.

## Code behind each row

Line numbers are those of this worktree.

| Row | Source                                                                                                                                                                |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| L1  | `src/s8/led_state.rs` 132-176 (`present`: levels, DECK, mode buttons, FX SELECT/BACK/CAPTURE/EDIT dim, display off); `src/s8/leds/map.rs` 4-6 (`ON` 0x7f, `DIM` 0x14) |
| L2  | `src/s8/leds.rs` 139-179 (`render` writes only the `0x80`/`0x81` offsets); `docs/s8/mapping.md` "Not verified on the S8"                                              |
| L3  | `src/s8/feedback.rs` 70-98 (loaded, playing and progress per deck); `led_state.rs` 85-98 (hot cue pads), 120-129 (strip)                                              |
| L4  | `led_state.rs` 168-172; `feedback.rs` 70-81, 111-113                                                                                                                  |
| L5  | `led_state.rs` 163-167; `src/s8/translator/mod.rs` 161 (`cue_held`)                                                                                                   |
| L6  | `led_state.rs` 162, 67-69, `leds/map.rs` 26 (`SYNC_GREEN`); the red SYNC offset 57 is never set                                                                       |
| L7  | `led_state.rs` 159 (note 4)                                                                                                                                           |
| L8  | `led_state.rs` 145 (`ring`), 173; `leds.rs` 139-179 (`RING_WHITE`, `RING_BLUE`); `led_state.rs` 71-83 (`dual`)                                                        |
| L9  | `led_state.rs` 134-143, 155-158; `src/s8/translator/lookup.rs` 17-22 (Shift + HOTCUE is Loop)                                                                         |
| L10 | `led_state.rs` 26-38 (`mode_colours`, divisor 6), 85-98 (`pad`)                                                                                                       |
| L11 | `led_state.rs` 91-94; `src/s8/led_palette.rs` 40-47                                                                                                                   |
| L12 | `led_state.rs` 95; `feedback.rs` (sampler notes shared by A/C and B/D)                                                                                                |
| L13 | `led_state.rs` 100-111 (`fx_level`, reads `fx_target`)                                                                                                                |
| L14 | `led_state.rs` 113-118 (`slot_level`, notes 56-59)                                                                                                                    |
| L15 | `led_state.rs` 40-65 (`Local`), 71-83 (`dual`, white from deck C), 160                                                                                                |
| L16 | `leds.rs` 139-179 and `led_output.rs` 55-74 (side index selects `0x80`/`0x81`)                                                                                        |
| L17 | `src/s8/leds/strip.rs` 20-31 (`bar`: dark at 0, at least one LED above 0); `led_state.rs` 120-129                                                                     |
| L18 | `strip.rs` 7-10, 20-31 (progress 0-127 scaled to 25)                                                                                                                  |
| L19 | `strip.rs` 20-31; `feedback.rs` 82-85 (`songProgress`)                                                                                                                |
| L20 | `led_state.rs` 120-123; `strip.rs` 12-18 (`dot`); `translator/mod.rs` 155-158 (position only while touched and moving)                                                |
| L21 | `led_state.rs` 161 (`level(local.shifted)`)                                                                                                                           |
| L22 | `led_output.rs` 76-88 (`finish`); `src/s8/bridge.rs` 160-165 (clear at exit, `S8_LEDS_CLEAR_FAILED`)                                                                  |
| L23 | `docs/s8/agent-stop-validation.md`; `led-feedback-check.sh` restore path                                                                                              |
| L24 | as L23                                                                                                                                                                |

Not in the plan because the code has no such behaviour: FX SELECT, BACK,
CAPTURE and EDIT never change (always dim), the display buttons are always off,
and Shift changes only its own LED. No LED reads the touch of the touchstrip
other than the orange dot, so there is no separate touched row.

## Results

Rows are sorted by what the words of the answer say
against the Expected column. No cause is given here.

### Answer matches the expected result

- L4: "yes".
- L8: ring blue on deck A, white on deck C; brighter in the loop, dimmer outside.
- L12: the pad lights while the sample plays, then dims.
- L14: ON is "dim when the stem is on and bright when the stem is muted".
- L21: "yes nothing else changes".
- L24: "yes", display buttons 1 and 4 work.

For L6, SYNC going "from green to bright green" and "sync never turns red" match;
the CUE flash in L6 is listed below.

### Answer differs from the expected result

- L3: expected pads lit for saved cues; the answer says "all the hot cues are off" on
  a deck that has 7 hot cues, and says the right deck CUE is "flashing blue".
- L5: the answer says "when i hold it, the green play button comes on".
- L6: pausing "makes the cue button flash blue" on a paused deck.
- L7: FLUX "flashes orange when slip is on", where the expected result is full.
- L11: "none of the pads come on".
- L13: "FX4 goes on and off when any of them are turned on or off".
- L18: the bar "goes all the way across" at about half of the track.
- L20: "the dot is purple", where the expected dot is orange.

### Not settled

- L1: DECK is "pretty bright" where full is expected, and FLUX is not named.
- L2: "they're all lit"; whether any changed when the bridge started was not answered.
- L9: which mode button is full was not answered.
- L10: whether the dim pads are visible was not answered. The answer "no effect on the
  HOTCUE button" fits the expected result: HOTCUE was already dim when the question was
  asked, because FREEZE came first in the step sequence.
- L15: pads, ring and strip on C and A were not described.
- L16: the answer repeats the expected text: "yes only play on the right is
  established and it's independent of the left". The right-side controls that
  `bridge.log` shows were operated are `right_cue`, `right_deck`, `right_fader_1` to
  `right_fader_4`, `right_fx_dial_1` to `right_fx_dial_4`,
  `right_loop_encoder_touch`, `right_pad_1`, `right_play`, `right_remix` and
  `right_touchstrip`. Of those, `right_play` is 4 events, `right_cue` 12 and
  `right_pad_1` 6. SYNC, FLUX, FX 1 and pads other than pad 1 do not appear.
- L17: "yes it's off when progress is 0 and turns blue when it's above 0"; how many LEDs
  and which row were not stated there.
- L19: "it's all the way on and then when the track finishes it turns off"; whether the
  bar grows with the playhead was not stated.
- L22: "everything is off except the FX 1-2 of all decks, snap, quantize, mic 1 mic 2,
  all 4 cue buttons and all 4 filter buttons"; whether the FX 1-2 are on the side
  panels or the mixer was not stated.
- L23: the labels are back; the template's LEDs were not described.
- The user's remark after the run: "deck A has 8 hot cues and all the pads are bright.
  deck B has 5 bright pads but there are 7 hot cues total. deck A's stem mute 1 and 2
  are both on but pressing them doesn't do anything". It is not known whether this
  describes the run or the state after NIHardwareAgent and the Controller Editor
  template were back.

### Points marked open in the output check

- Dim level visible: L1 "everything else is dimmer"; L8 "it seems like there's only two
  settings, bright and dim"; L12 "the pad lights up, then it dims". The dim pads of L10
  were not answered.
- Blue bank row: L5 "the top row of the touchstrip is blue, the bottom row is off";
  L18 "the top bar is all the way on the bottom bar is off".
- DECK colours: L15 "A is blue and C is white".
- Bottom row of the strip: L5 and L18 as above, "off"; L20 does not name a row.
- Order inside multi-byte groups (L15): not answered.

### Other observations in the answers

- L9: "loop - looper opens but no change in color on the pads"; "freeze - hotcues turn
  teal"; "remix - sampler toggles on and pads turn orange"; "freeze also enables slicer
  mode".
- L12: "switching to deck A and Deck b turns the top row of the touchstrip blue and then
  it turns off when you switch decks".
- L3 and L23: the Controller Editor labels show 8 hot cues for deck A and 5 for deck B
  while deck B has 7; on the right deck "hotcue 5 is off on the pad, but on on the
  display".
- L23: the labels are "evan's hotcues s8 template".
- L6: "play button is green on both decks".
- L15: "sync stays green the whole time".

## Causes

The causes of the run 1 rows, with the evidence level of each. No fix below has been seen
on the S8 yet. Evidence levels: capture, code and test, checked against djay without the
S8, inference, unexplained.

- Hot cue pads dark (L3, L9, L11). Evidence: capture. With mapping r7 djay sent value
  127 on notes 40-47 (channels 1-4) for a set cue point and 0 for an unset one. The
  bridge reads a colour token 1-9 there, so 127 lit nothing. r7 had an empty `userInfo`.
  Fix: mapping r8 carries `userInfo` with `padColorOff` 0, `padColorWhiteActive` 9 and
  `padColorsActive` 1-8. Checked against djay without the S8: notes 40-47 carry tokens
  1-8 for set cue points on all four decks and 0 for the unset cue 5 of deck B.
- Strip bar full at any progress (L5, L17, L18, L19). Evidence: capture. With r7 djay
  sent `songProgress` (CC 8-11 on channel 7) as 0 or 127 only. Fix: r8 marks these
  outputs `controlType: control`. Checked against djay without the S8: deck 1 playing
  from the start for 25 s sent 0, 1, 3, 4 ... 22 in steps; deck 2 sent 61-66 over 8 s.
- Purple dot (L20). Evidence: code and test. The orange and blue LED banks share the
  top row of the strip, and lit together they look purple. Fix: the bridge blanks the
  blue LED under the orange dot, with a unit test. "Stays on after release" was the
  full bar of the cause above.
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

## Second run

Not run yet. Command, in the worktree, with mapping `Kontrol S8 Bridge r8`:

```text
mise run s8:leds only L3 L7 L9 L11 L15 L16 L17 L18 L19 L20
```

The exit rows L22-L24 are always asked. The rows and why each is in it:

- L3, L9, L11: the hot cue colours, after the `userInfo` fix.
- L7: FLUX "flashes" where the capture shows a steady 127; unexplained.
- L15: pads, ring and strip on deck C and on A were not described; this also re-checks
  the DECK button colours.
- L16: the right side was not settled.
- L17, L18, L19: the progress bar, after the `controlType: control` fix.
- L20: the purple dot, after the blanking of the blue LED under the orange dot.
