---
summary: "Evan's Kontrol S8 layout on the bridge: select it, find each address and see what differs."
type: reference
issues: [65]
---

# Evan layout for the Kontrol S8

The Evan layout follows Evan's Hotcues and Slicer Controller Editor templates and his
`1.1 DynColor` Djay mapping. The bridge uses its own MIDI addresses and the virtual device
`Kontrol S8 MIDI Full`, not Controller Editor's `Traktor Kontrol S8` port. An
[earlier revision was checked on the S8](evan-layout-validation.md); the changes below
to Shift + FREEZE, FX and stem ON 1 / 2 have not yet been checked on the S8.

## Choose the layout

- In the [S8 kit](kit.md), choose Evan (`e`). In Djay's MIDI settings, select
  `Kontrol S8 Bridge Evan` on `Kontrol S8 MIDI Full`.
- To export it yourself, point the bridge at an existing directory:
  `s4mk3-hid-midi-bridge --device s8 --s8-layout evan --generate-mapping <directory>`.
  The export writes `Kontrol S8 Bridge Evan.djayMidiMapping` without overwriting.
- Start the bridge with `--device s8 --midi full --s8-layout evan`, or launch through
  `S8Host --bridge <path> --layout evan`. The mapping and bridge must use the same layout.
  Leave the layout option out to use the [default layout](mapping.md).

## Deck controls and pads

Channel numbers are one-based. Decks A–D use channels 1–4; the alternate layer uses
channels 9–12. Numbers are bridge note or CC numbers, not the template's MIDI addresses.

| Physical control               | Evan action                                                                  | Bridge address                         |
| ------------------------------ | ---------------------------------------------------------------------------- | -------------------------------------- |
| PLAY / CUE / SYNC              | Play/pause, deck cue, BPM sync                                               | Notes 0 / 1 / 2                        |
| Loop encoder                   | Turn: loop length; Shift turn: beat skip; press: auto loop on/off            | CC 1 (base or Shift); note 6           |
| BACK                           | Switch library table on A/B; switch back on C/D                              | Note 9                                 |
| Browse encoder                 | Scroll (flipped); press to load and toggle library expanded                  | CC 3; note 12                          |
| Shift + browse press           | Switch back on A/B; same as base on C/D                                      | Shift note 12 on A/B                   |
| FX SELECT                      | Toggle library expanded                                                      | Note 14                                |
| CAPTURE                        | Set downbeat on B only                                                       | B note 22                              |
| HOTCUE / LOOP / FREEZE / REMIX | Hold bounce loop: 1 / ½ / ¼ / ⅛ beat                                         | Notes 25–28                            |
| Shift + HOTCUE / FREEZE        | Select Hotcues / toggle between Hotcues and Slicer                           | Shift note 25 (Hotcues) or 27 (Slicer) |
| Hotcues pads 1–8               | Set/jump to cue and select it for Pitch Play; Shift clears cue               | Notes 40–47 (base or Shift)            |
| Slicer pads 1–8                | Trigger slices 1–8                                                           | Notes 72–79                            |
| FLUX or EDIT + pad             | Pitch Play actions 1–8                                                       | Alternate notes 64–71                  |
| Stem faders 1–4                | Set stem volumes with pickup                                                 | CCs 48–51                              |
| Stem ON 1 / 2                  | Key + / Key − on the active deck                                             | Notes 56 / 57                          |
| Stem ON 3 / 4                  | Reset pitch / tempo on A, C, D; move B to deck 3 / 4 on B                    | Notes 58 / 59                          |
| FLUX or EDIT + stem ON 1–4     | A/B: bounce loop ½ to ¹⁄₁₆ beat; C: auto loop in those lengths; D: unchanged | Alternate notes 56–59 on A–C           |
| Touchstrip                     | Scratch; Shift: seek                                                         | CC 5 and touch note 7 (base or Shift)  |
| DECK                           | Switch A/C or B/D (bridge behavior)                                          | Channel 6 notes 0–3                    |

Hold FLUX or EDIT on either side to activate the modifier. Holding physical Shift takes
precedence over it for pads (in Hotcues, Shift clears a cue). Physical Shift alone does
not change stem ON actions. A held note releases on its original channel even if you
change decks, layers or pad modes before letting go.

## FX and mixer

| Control              | Action                    | Bridge address                        |
| -------------------- | ------------------------- | ------------------------------------- |
| FX buttons 1–3       | Toggle slots 1–3          | Ch. 1 notes 32–34; ch. 2 notes 36–38  |
| FX button 4          | All off / all on          | Sends the slot notes above as needed  |
| FX dials 1–3         | Slot parameters 1–3       | Ch. 1 CCs 32–34; ch. 2 CCs 40–42      |
| FX dial 4            | Dry/wet for all slots     | Ch. 1 CC 35; ch. 2 CC 43              |
| Mixer FX 1 / 2       | Tempo minus / plus        | Ch. 5 notes 92–99, A through D        |
| Other mixer controls | Levels, EQ, filters, etc. | [Default](mapping.md#mixer-channel-5) |

FX 4 turns off the enabled slots if any are on; otherwise it turns on all three. Its LED
is bright when at least one slot is on and dim otherwise. The bridge uses the latest Djay
feedback, treating slots with no feedback as off. Give Djay time to report the new state
before pressing FX 4 again.

## Differences from the Controller Editor setup

- Shift + FREEZE switches between Hotcues and Slicer on that side. Shift + HOTCUE
  always selects Hotcues. Display buttons 1 and 4 stay reserved for Native
  Instruments' color settings and template switching; the bridge sends no MIDI
  for them. Evan's layout has no separate Sampler or stem-mute pad bank.
- The modifier is bridge-local: no Controller Editor or Djay modifier state is required.
  Alternate pitch-play and slicer bindings from the original stem/screen controls and
  mode buttons are not duplicated there; use the pads instead. Deck switch and Shift
  retain their bridge roles.
- Touch and position use separate MIDI messages. The bridge normalizes the seek target
  to the active deck and uses its seek tuning for C/D where the source did not supply a
  usable per-deck assignment. Scratch tuning follows the source by deck.
- FX controls apply to A/C on the left and B/D on the right, regardless of deck selection.
  The LEDs and FX 4 follow A or B only. If you change a slot on just one deck in Djay,
  the two decks can differ: toggling that slot flips it on both decks.
  Mixer FX buttons change tempo rather than reassigning either FX unit.
- S8Host reserves the display encoders for the panel views. The original two monitor-level
  controls, continuous pitch and tempo on display knobs 3 and 4, and duplicate pitch-play
  and slicer triggers on the display encoders are not mapped there. Mixer headphone mix
  remains available; it is not the display-encoder monitor-level control.
- Source rows with only a bare application, mixer or deck target (e.g. on FX 1, A's
  CAPTURE, shifted B PLAY, modified A FX+, quantize/snap and the touchstrips) do not name
  actions and were not copied. CAPTURE has no action on A/C/D. Mic 2 and Snap remain
  unassigned.
- The bridge retains mixer PFL, crossfader assign, mic, quantize, headphone mix and booth/
  master mappings; these are not extra Controller Editor assignments.

LEDs use the bridge's [feedback catalog](mapping.md#led-feedback), including deck
transport, hotcue colors and strip progress. As with Default, deck CUE is fully lit
when pressed or when Djay's CUE feedback is on; it cannot get brighter if already lit.
Slicer has local bank lighting, and Pitch Play and Key + / Key − have no LED feedback.
