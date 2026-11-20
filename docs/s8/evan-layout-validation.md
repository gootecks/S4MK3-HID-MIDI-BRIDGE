---
summary: "Checks of the Evan S8 layout: scripted djay check and the attended S8 run of 2026-11-20, with the changes they asked for."
type: validation
issues: [65]
---

# Evan layout validation

The layout is described in [mapping-evan.md](mapping-evan.md).

## Scripted djay check, 2026-11-20

No S8 and no bridge: a virtual source sent the bridge's Evan addresses to djay with the
mapping installed as `Kontrol S8 Bridge Evan r1`. Notes and pictures:
`docs/s8/captures/evan-layout-djay-20261120/`.

- Passed: browse and load, hot cue 1, PLAY, Shift + pad clears a cue, mixer FX buttons
  nudge tempo, stem ON 4 resets tempo, bounce loop 1 beat with slip, Shift + FREEZE
  slicer view and slice 1, Shift + HOTCUE back to hot cues.
- Inconclusive: browse scroll direction; whether Pitch Play changed the pitch.

## Attended run, 2026-11-20

The bridge at commit `005ecb2` ran with
`--device s8 --midi full --s8-layout evan --s8-leds`, NIHardwareAgent stopped and djay
set to mapping `Kontrol S8 Bridge Evan r2` on `Kontrol S8 MIDI Full`. Log:
`docs/s8/captures/evan-layout-attended-20261120T133527Z/bridge.log.gz`. Rows were on
deck A unless named. User answers:

| Row  | Check                                                 | Answer                                                                               |
| ---- | ----------------------------------------------------- | ------------------------------------------------------------------------------------ |
| E1   | Browse encoder turn, press loads and expands          | Yes                                                                                  |
| E2   | PLAY, CUE, SYNC and their LEDs                        | Yes, except the CUE LED does not get brighter when pressed                           |
| E3   | Loop encoder turn, press, Shift + turn                | Yes                                                                                  |
| E4   | Hot cue pads set, jump, Shift clears, colours         | Yes                                                                                  |
| E5   | HOTCUE, LOOP, FREEZE, REMIX held bounce loops         | Yes                                                                                  |
| E6   | Shift + FREEZE slicer and slices, Shift + HOTCUE back | Yes; Shift + FREEZE should also switch back                                          |
| E7   | FLUX + pad Pitch Play                                 | Yes                                                                                  |
| E8   | Stem faders with pickup                               | Yes                                                                                  |
| E9   | Stem ON 1 / 2 PFL with mixer CUE LED, ON 3 / 4 resets | Yes; ON 1 / 2 should do something else now that CUE does PFL                         |
| E10  | FLUX + stem ON 1-4 bounce loops                       | Yes                                                                                  |
| E11  | Left FX buttons and dials                             | FX 1 drags FX 2 and FX 3 with it; wants FX 1-3 for slots 1-3 and FX 4 for FX 1's job |
| E12  | Mixer FX 1 / 2 tempo down and up                      | Yes                                                                                  |
| E13  | Touchstrip scratch, Shift + touchstrip seek           | Yes                                                                                  |
| E14  | Deck C on the left, deck B on the right               | Yes                                                                                  |
| E15  | Mixer CUE A-D LEDs follow djay                        | Yes                                                                                  |
| Exit | LEDs after Ctrl-C                                     | Back to the default state                                                            |

The log has 44 `S8_CUE_LEDS` writes and no `S8_LEDS_DISABLED` or
`S8_LEDS_CLEAR_FAILED`. NIHardwareAgent was restored.

Requested changes for the next revision:

- Shift + FREEZE toggles back to hot cues.
- FX buttons 1-3 select slots 1-3, and FX 4 takes the Master FX action.
- Stem ON 1 / 2 become Key + / Key -.
- The CUE LED gets brighter while pressed.
