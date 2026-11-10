# S8 default Djay mapping

The bridge generates `Kontrol S8 Bridge.djayMidiMapping` for the virtual MIDI
endpoint `Kontrol S8 MIDI Full`. Every address comes from
`src/s8/catalog.rs`; this page lists the default layout that catalog produces.

This is the working default layout. It assumes the S8 is the only controller
Djay uses. A second layout based on Evan's mapping is tracked in
gootecks/S4MK3-HID-MIDI-BRIDGE issue #65. The mapping has no LED or screen
feedback yet (its `outputs` array is empty). Nothing here is hardware-verified
until the attended check.

## Channels and message types

Channels below are one-based, as Djay shows them. The plist stores them
zero-based.

| Scope                 | Channel |
| --------------------- | ------- |
| Decks A-D             | 1-4     |
| Mixer                 | 5       |
| Deck select           | 6       |
| Decks A-D, Shift held | 9-12    |

Notes are message type 1; CCs are type 3. Number is the note or CC number.

Each deck side (left and right) sends on the channel of the deck it currently
controls. The DECK button switches a side between A/C or B/D and sends the
deck-select note. A control that has a Shift row uses it while Shift is held;
a control without one keeps its base row. Shift rows share the number of their
base row and sit on channel + 8.

## Deck controls

Channel is the deck's channel (1-4), or 9-12 for Shift. Both sides send the
same numbers.

### Notes

| Control              | No. | Base                                          | Shift                      |
| -------------------- | --- | --------------------------------------------- | -------------------------- |
| PLAY                 | 0   | `playPause`                                   |                            |
| CUE                  | 1   | `cuePositionOrJumpConsideringPlayState1`      | `gotoZero`                 |
| SYNC                 | 2   | `bpmSync`                                     |                            |
| FLUX                 | 4   | `deckSlipToggle`                              | `reverseHold`              |
| Loop encoder press   | 6   | `autoLoopOnOff`                               | `reloop`                   |
| Touchstrip touch     | 7   | `scratchingMode`                              | `jogSeekMode`              |
| BACK                 | 9   | `musicLibrary.libraryBack`                    | `toggleLibrarySource`      |
| Browse encoder press | 12  | `musicLibrary.load{deck}`                     | `toggleLibraryVisible`     |
| EDIT                 | 13  | `musicLibrary.togglePreview`                  |                            |
| CAPTURE              | 22  | `recordSample`                                |                            |
| FREEZE               | 27  | `padModeSlicer`                               | `slicerLoopModeToggle`     |
| Stem ON 1-4          | 56-59 | `unmixerFourTrackChannel{1-4}Muted`         | `...Channel{1-4}Solo`      |

Unqualified names are `turntable{deck}.` actions. `libraryBack`,
`toggleLibrarySource`, `load{deck}`, `toggleLibraryVisible` and `togglePreview`
are `musicLibrary.` actions; `load{deck}` loads into the sending deck.

### CCs

| Control        | No.   | Type            | Base                                  | Shift         |
| -------------- | ----- | --------------- | ------------------------------------- | ------------- |
| Loop encoder   | 1     | relative        | `autoLoopDurationRotary`              | `skipRotary`  |
| Browse encoder | 3     | relative        | `musicLibrary.libraryRotary` (flipped) | `musicLibrary.sectionRotary` (flipped) |
| Touchstrip     | 5     | rotary-absolute | `scratchingMove` (flipped)            | `jogSeekMove` |
| Stem fader 1-4 | 48-51 | absolute, pickup | `unmixerFourTrackChannel{1-4}Volume` |               |

The touchstrip uses its own sensitivity and acceleration per layer; the tuned
values are in the catalog.

## Pads

The pad layer is chosen by the HOTCUE, LOOP, FREEZE and REMIX buttons, which
the bridge handles itself. Pad numbers 1-8 map to the note numbers shown.
All pad rows are notes on the deck channel.

| Mode    | Pads 1-8 | Base action                                              | Shift action     |
| ------- | -------- | -------------------------------------------------------- | ---------------- |
| HOTCUE  | 40-47    | `cueOrJumpIfAlreadySet1-8` (hot cues)                    | `clearCuePoint1-8` |
| LOOP    | 64-71    | `bounceLoop...BeatInterval`, 1/32 beat (pad 1) to 4 beats (pad 8), held | none |
| FREEZE  | 72-79    | `slicer8Slice1-8` (Slicer)                               | none             |
| REMIX   | 48-55    | `sampler.turntable{bank}.player1-8.playingConsideringHoldSetting` (sampler) | `...paused` |

Shift+FREEZE (note 27 on the Shift channel) toggles slicer loop mode. Decks A
and C use sampler bank 1; decks B and D use bank 2.

## FX units

Each unit targets the deck chosen by its mixer-channel FX buttons, and sends
on that deck's channel. The right unit has its own numbers so both units can
target the same deck. Dial 1 is dry/wet for all three effect slots.

| Control            | Left unit | Right unit | Base                | Shift                |
| ------------------ | --------- | ---------- | ------------------- | -------------------- |
| FX 1 (notes)       | 32        | 36         | `fxActive`          |                      |
| FX 2 (notes)       | 33        | 37         | `fx1Enabled`        | `fx1SelectNext`      |
| FX 3 (notes)       | 34        | 38         | `fx2Enabled`        | `fx2SelectNext`      |
| FX 4 (notes)       | 35        | 39         | `fx3Enabled`        | `fx3SelectNext`      |
| Dial 2 (CC)        | 32        | 40         | `fx1ParameterValue` |                      |
| Dial 3 (CC)        | 33        | 41         | `fx2ParameterValue` |                      |
| Dial 4 (CC)        | 34        | 42         | `fx3ParameterValue` |                      |
| Dial 1 (CC)        | 35, 36, 37 | 43, 44, 45 | `fx1WetDryValue`, `fx2WetDryValue`, `fx3WetDryValue` | |

The three Dial 1 numbers are the same physical dial sent three times, one per
effect slot. All dials use pickup.

## Mixer (channel 5)

### CCs

| Control               | No.   | Action                      |
| --------------------- | ----- | --------------------------- |
| GAIN A-D              | 0-3   | `turntable{1-4}.gain`       |
| EQ HI A-D             | 4-7   | `turntable{1-4}.highEQ`     |
| EQ MID A-D            | 8-11  | `turntable{1-4}.midEQ`      |
| EQ LOW A-D            | 12-15 | `turntable{1-4}.lowEQ`      |
| Channel fader A-D     | 16-19 | `mixer.lineVolume{1-4}`     |
| Crossfader            | 20    | `mixer.crossfade`           |
| FILTER A-D            | 21-24 | `turntable{1-4}.filter`     |
| Tempo encoder         | 26    | `turntableSelected.speed` (relative) |

Mixer CCs do not use pickup.

### Notes

| Control        | No.   | Action                               |
| -------------- | ----- | ------------------------------------ |
| CUE (PFL) A-D  | 0-3   | `mixer.monitorActive{1-4}`           |
| MIC            | 4     | `microphone.active`                  |
| TEMPO button   | 26    | `turntableSelected.resetSpeed`       |

## Crossfader assign (channel 5, notes)

One note per channel and switch position, sent when the switch moves.

| Channel | Left | Middle | Right |
| ------- | ---- | ------ | ----- |
| A       | 80   | 81     | 82    |
| B       | 83   | 84     | 85    |
| C       | 86   | 87     | 88    |
| D       | 89   | 90     | 91    |

Actions are `mixer.crossfadeAssignment{1-4}Left`, `...Through` and `...Right`.

## Deck select (channel 6, notes)

Sent by a DECK button for the deck its side switches to.

| Deck | No. | Action                          |
| ---- | --- | ------------------------------- |
| A    | 0   | `application.turntable1Selected` |
| B    | 1   | `application.turntable2Selected` |
| C    | 2   | `application.turntable3Selected` |
| D    | 3   | `application.turntable4Selected` |

## Hardware-local controls

The bridge handles these itself; they send nothing to Djay.

| Control                       | Function                                                                  |
| ----------------------------- | ------------------------------------------------------------------------- |
| SHIFT                         | selects the Shift layer of its side                                       |
| DECK                          | switches its side between decks A/C or B/D, then sends the deck-select note |
| HOTCUE, LOOP, FREEZE, REMIX   | select the pad layer of their side                                        |
| `mixer_fx_X_1`, `mixer_fx_X_2` | select mixer channel X as the target of the left or right FX unit        |
| `mixer_filter_on_X`           | gates the FILTER knob of mixer channel X                                  |

## Unassigned controls

| Control                                                                        | Reason                                         |
| ------------------------------------------------------------------------------ | ---------------------------------------------- |
| SNAP, QUANTIZE (`mixer_snap`, `mixer_quantize`)                                | deliberately unassigned in this prototype      |
| `mixer_mic_2`                                                                  | Djay has one microphone action                 |
| `deck_assign_X`                                                                | Djay decks are fixed to their mixer channels   |
| FX SELECT (`fx_select`)                                                        | reserved for an on-screen effect picker        |
| `screen_left_2`, `screen_left_3`, `screen_right_1..4`, `screen_encoder_1..4`   | reserved for the app's screen pages            |
| `screen_left_1`, `screen_left_4`                                               | Native Instruments functions                   |
| `*_touch` except `touchstrip_touch`                                            | touch caps are not sent to Djay                |
| `*_latch`                                                                      | toggle states the S8 keeps itself              |
