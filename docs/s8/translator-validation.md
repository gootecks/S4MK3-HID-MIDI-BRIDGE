# S8 input-to-MIDI translator validation

Validation record for #15. `--device s8 --midi full` reads S8 HID input on
interface 5 and publishes the catalog's MIDI ([mapping.md](mapping.md)) on the
virtual source `Kontrol S8 MIDI Full`. The bridge sends nothing to the S8: no
LED report, feature request, control transfer or screen traffic.

## Status

| Part | Result |
| ---- | ------ |
| Translator against the catalog | Unit tests, including a replay of the captured reports through the real decoder |
| Startup with the S8 connected | Run once, unattended: MIDI source created, HID opened, no input in 3 s |
| MIDI from physical controls | **Not checked** |
| Djay reaction with the generated mapping | **Not checked** |
| Behaviour with NIHardwareAgent running | **Not checked** |

No control has been checked on hardware. The attended check below is still to
be run.

## What the unit tests cover

- Buttons send note on and note off on the deck channel, and on channel + 8
  while Shift is held. A note is released on the address it was pressed on,
  even when Shift, the deck or the pad mode changed in between.
- DECK switches a side between A/C or B/D and sends the deck-select note.
- HOTCUE, LOOP, FREEZE and REMIX change the pad layer of their side.
- The mixer FX buttons retarget the FX unit; FX controls then send on the
  target deck's channel.
- FILTER ON is a bridge toggle. Off sends the centre value; on sends the knob
  position again.
- Crossfader assign sends one note pulse per channel whose switch position
  changed in a report.
- Step encoders send relative values around 64. Faders and knobs send 7-bit
  values, once per change.
- The touchstrip sends the touch note and the position while touched, on the
  deck and layer in force when the touch began.
- The captured reports in `tests/fixtures/s8-input-reports.jsonl` replay
  through the Encdr parser into the translator.

## Known limits

- The S8 reports only when a control moves, and the bridge does not request
  the current state. The first report of each report ID after start therefore
  synchronises positions: mixer faders, knobs and crossfader assign are sent,
  stem faders and FX dials are remembered, and a button or encoder change in
  that report is not sent. Move any control once after starting the bridge.
- Crossfader assign is sent once a switch reports left or middle. With all
  four switches right, nothing is sent until one moves.
- Encoder steps larger than 63 in one report are clamped.
- SNAP and QUANTIZE are unassigned. FX SELECT is unassigned.
- There is no LED or screen feedback; the mapping has no outputs.
- `--midi` without `full`, `--led-config`, `--tension-config`, `--jog-config`,
  `--screens` and `--screen-control-stdin` are refused for the S8.

## Attended check

Needs the S8, Djay Pro and a person at the controller. Reading HID input is
passive; this check sends nothing to the S8.

### Setup

1. Generate the mapping into Djay's mapping folder:

       cargo run --locked -- --device s8 \
           --generate-mapping ~/Music/djay/"MIDI Mappings"

   This writes `Kontrol S8 Bridge.djayMidiMapping`. An existing file with that
   name is not overwritten.
2. Stop NIHardwareAgent. While it runs, it also turns S8 input into MIDI for
   the Controller Editor template, so Djay would receive two sets of commands.
   Restore it afterwards with
   `open -g "/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"`
   ([agent-stop-validation.md](agent-stop-validation.md)).
3. Start the bridge:

       cargo run --locked -- --device s8 --midi full --until-stopped

   Expect `MIDI_FULL_READY port="Kontrol S8 MIDI Full"` and `S8_HID_OPEN`.
4. In Djay, open the MIDI settings, choose `Kontrol S8 MIDI Full` and confirm
   that it uses `Kontrol S8 Bridge`.
5. Load a track on every deck. Move one control on each side and one mixer
   control so the bridge has seen both reports.

### Steps

Record pass, fail or not run for each row, per side where it applies.

| # | Action | Expected in Djay |
| - | ------ | ---------------- |
| 1 | PLAY, CUE, SYNC on decks A and B | Play, cue and sync on the matching deck |
| 2 | DECK on each side, then PLAY | The side controls C or D; PLAY acts on that deck |
| 3 | Shift + a control with a Shift row | The Shift action in [mapping.md](mapping.md) |
| 4 | HOTCUE, then pads 1-8 | Hot cues set and jump |
| 5 | LOOP, then pads 1-8 | Bounce loops while held |
| 6 | FREEZE, then pads 1-8 | Slicer opens; pads play slices |
| 7 | Shift + FREEZE | Slicer loop mode toggles |
| 8 | REMIX, then pads 1-8 | Sampler pads play |
| 9 | Hold a pad, change pad mode, release | The held pad's action ends; no stuck note |
| 10 | Loop encoder turn and press; Shift + turn and press | Loop length changes and the loop toggles; Shift skips and reloops |
| 11 | Browse encoder turn and press; BACK, EDIT | The library scrolls and the track loads into that side's deck; back and preview |
| 12 | FLUX, CAPTURE | Slip toggles; a sample records |
| 13 | Touchstrip: touch, slide, release | Scratch motion while touched; nothing after release |
| 14 | Shift + touchstrip | Seek |
| 15 | TEMPO encoder and button | The selected deck's tempo changes and resets |
| 16 | Channel faders, crossfader, EQ, gain, filter on all four channels | The matching mixer control follows |
| 17 | FILTER ON off, turn the filter knob, FILTER ON on | Filter stays neutral while off, then jumps to the knob |
| 18 | Crossfader assign switches, three positions per channel | Channel assigned left, through or right |
| 19 | CUE on each mixer channel; MIC | Pre-listen toggles on that channel; the microphone toggles |
| 20 | FX unit dials and buttons, then a mixer FX button, then the dials again | The FX of the newly chosen deck follows |
| 21 | Stem faders and ON buttons, Shift + ON | Stem volume, mute and solo |
| 22 | SNAP, QUANTIZE, FX SELECT, display buttons, screen encoders | No Djay reaction |
| 23 | Ctrl-C | The bridge stops; the `Kontrol S8 MIDI Full` source disappears |

After the check, restore NIHardwareAgent and confirm that the panels show
Controller Editor labels and that PLAY works with the existing setup.

## Results

None yet.
