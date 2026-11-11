---
summary: "Attended check plan and record for the S8 deck LEDs driven by --s8-leds from djay feedback (led-feedback-check.sh); not run yet."
type: validation
issues: [22, 23]
---

# S8 LED feedback validation

Validation record for #22 and #23. `--device s8 --midi full --s8-leds` lights
the deck, pad and touchstrip LEDs of the S8 from djay's MIDI feedback and from
the bridge's own control state. What each LED should show is in
[mapping.md](mapping.md#led-feedback). The report layout was probed on its
own in the LED output check; this check is the first run driven from djay.

## Status

Not run. Nothing from `--s8-leds` has been sent to the S8. The unit tests pass.
The rows below start as `Not run` and take the user's answers unchanged.

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
6. It asks the rows one at a time. Each answer goes unchanged into
   `notes.txt` behind the row id.
7. It stops the bridge with SIGINT, which clears the LEDs, and asks L22.
8. It restores NIHardwareAgent and asks L23 and L24.

The log lines of interest are `S8_LEDS_ENABLED` at start, `S8_LEDS_DISABLED`
after a failed or short write, and `S8_LEDS_CLEAR_FAILED` after a failed clear.
The bridge does not log a count of LED writes. The runner counts those three
markers after the bridge stops and notes them.

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

| Row | Action                                                                                                                                             | Expected                                                                                                                                                  | Answer  |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- |
| L1  | Nothing is loaded on decks A to D. Look at both sides and touch nothing.                                                                           | PLAY, CUE, FLUX off; DECK full; HOTCUE full; the other mode buttons, SYNC, ring, FX and ON dim; pads and strips dark. Dim level open in the output check. | Not run |
| L2  | Look at the LEDs the bridge does not write: filter on and the arrows at the top of each deck, the mixer CUE buttons, MIC 1, MIC 2, SNAP, QUANTIZE. | They stay as the firmware leaves them. The bridge writes only `0x80` and `0x81`, so it does not change them.                                              | Not run |
| L3  | In djay load one test track on deck A and one on deck B. Do not play.                                                                              | Left shows A, right shows B. PLAY off; strip blue bank dark at progress 0; pads dark unless the track has saved cues, which light in their cue colours.   | Not run |
| L4  | Press left PLAY to play deck A, then again to pause it.                                                                                            | PLAY full while A plays, off when paused (root `playing` output).                                                                                         | Not run |
| L5  | Pause deck A at its start. Hold left CUE, then release it. Do not hold Shift.                                                                      | CUE full while it is held, from the local state and from djay's CUE note. After the release it follows djay's CUE feedback.                               | Not run |
| L6  | Play deck B, play deck A, press left SYNC to switch sync on, then again to switch it off.                                                          | SYNC (green LED) full with sync on, dim with sync off. The red SYNC LED stays off. Open in the output check: the dim level.                               | Not run |
| L7  | Press left FLUX, then again.                                                                                                                       | FLUX full while slip is on, off otherwise.                                                                                                                | Not run |
| L8  | Press the left loop encoder to start a loop on deck A, then again to end it.                                                                       | All four ring segments full while the loop is active, dim otherwise: blue on decks A and B, white on C and D.                                             | Not run |
| L9  | On the left side press HOTCUE, FREEZE, REMIX, then Shift + HOTCUE.                                                                                 | The button of the active pad mode is full and the other three are dim. Shift + HOTCUE makes LOOP the full one.                                            | Not run |
| L10 | On the left side select FREEZE, then Shift + HOTCUE, then REMIX, and look at the eight pads each time. Do not press a pad.                         | FREEZE: all pads dim blue. Shift + HOTCUE: all pads dim green. REMIX: all pads dim orange. Dim is the lit colour divided by 6.                            | Not run |
| L11 | Select HOTCUE. On deck A set a hot cue on pad 1 and pad 2, then delete the cue of pad 2 in djay.                                                   | Set pads light in djay's cue colour (tokens 1-8 from the S4 bridge's default colours, 9 white); unset pads are dark; the deleted pad goes dark.           | Not run |
| L12 | Select REMIX. Press sampler pad 1 to play its sample, then stop it. The slot needs a sample.                                                       | The pad is orange while its sample plays and dim orange when it stops. Deck A and deck C read the same sampler bank, B and D the other.                   | Not run |
| L13 | Press left FX 1, 2, 3 and 4 on, then off.                                                                                                          | Each FX LED is full while its effect is on, dim while off. The side shows the unit's target deck, chosen with the mixer FX buttons.                       | Not run |
| L14 | Needs a stem track on deck A. Press left ON 1, 2, 3 and 4 to mute and unmute stems.                                                                | Each ON LED is full when djay reports that stem muted, dim otherwise.                                                                                     | Not run |
| L15 | Press left DECK to switch to deck C, look at the left side, then press DECK again.                                                                 | On C the DECK LED is white; PLAY off, pads dark, ring dim. Back on A it shows A again. Group order open in the output check.                              | Not run |
| L16 | Repeat on the right side with deck B: play, hold CUE, SYNC, loop, FLUX, FX 1, a pad-mode button and a hot cue pad.                                 | The right side behaves as the left with its own report `0x81`, independent of the left. Only PLAY on the right is established.                            | Not run |
| L17 | Put deck A at the very start of its track (progress 0) and look at the left touchstrip.                                                            | The blue bank is dark at progress 0; one LED once progress is above 0. Open in the output check: which row of the strip the blue bank lights.             | Not run |
| L18 | Seek deck A to about half of the track and look at the left strip.                                                                                 | A blue bar from the physical left end, about 12 or 13 of 25 LEDs.                                                                                         | Not run |
| L19 | Seek deck A to its last seconds, then let it play on.                                                                                              | The bar grows to 24 or 25 LEDs near the end. Open in the output check: whether progress covers the whole track.                                           | Not run |
| L20 | Touch the left strip at its left end without moving, slide to the middle and to the right end, then release.                                       | No orange dot while still; one orange dot at the finger once it moves; out on release. Blue bar stays. Bottom row open in the output check.               | Not run |
| L21 | Hold left Shift, then release it.                                                                                                                  | SHIFT full while held, dim otherwise. Nothing else on that side changes.                                                                                  | Not run |
| L22 | The bridge has stopped with Ctrl-C. NIHardwareAgent is still stopped. Look at both sides.                                                          | Every LED the bridge owns is dark on both sides: buttons, pads, loop ring, strips. The LEDs of L2 are as before.                                          | Not run |
| L23 | NIHardwareAgent has been started again. Look at both panels and the LEDs.                                                                          | Controller Editor labels on both panels and the template's LEDs are back.                                                                                 | Not run |
| L24 | Press left PLAY once, then display button 1 or 4.                                                                                                  | Djay reacts to PLAY through the Controller Editor template; the NI action of the display button works.                                                    | Not run |

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

Empty until the check is run. Copy each answer unchanged from `notes.txt`
into the table. Interpretation goes in the lists below and is worded as
inference where it is one.

### Established

Only what the answers show directly. Nothing yet.

### Not settled

Nothing yet. Carry over, as they are answered or not: the dim level (L1, L6,
L8, L10), the order inside multi-byte groups (L15), the bottom row of the
touchstrip (L17, L20) and the right-side offsets (L16).
