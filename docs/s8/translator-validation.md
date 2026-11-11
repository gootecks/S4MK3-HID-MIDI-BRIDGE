---
summary: "What the input-to-MIDI translator tests cover, their limits, and the attended checks of 2026-11-10."
type: validation
issues: [15]
---

# S8 input-to-MIDI translator validation

Validation record for #15. `--device s8 --midi full` reads S8 HID input on
interface 5 and publishes the catalog's MIDI ([mapping.md](mapping.md)) on the
virtual source `Kontrol S8 MIDI Full`. The bridge sends nothing to the S8: no
LED report, feature request, control transfer or screen traffic.

<?toc?>

- [Status](#status)
- [What the unit tests cover](#what-the-unit-tests-cover)
- [Known limits](#known-limits)
- [Attended check](#attended-check)
  - [Setup](#setup)
  - [Steps](#steps)
- [Results](#results)
  - [First attended check, 2026-11-10](#first-attended-check-2026-11-10)
  - [Second attended check, 2026-11-10](#second-attended-check-2026-11-10)
  - [Third attended check, 2026-11-10](#third-attended-check-2026-11-10)
  - [Fourth attended check, 2026-11-10](#fourth-attended-check-2026-11-10)
  - [User statement after the fourth check, 2026-11-10](#user-statement-after-the-fourth-check-2026-11-10)
  - [Touchstrip](#touchstrip)
    - [Comparison with NIHardwareAgent, 2026-11-10](#comparison-with-nihardwareagent-2026-11-10)
    - [Fifth attended check, 2026-11-10](#fifth-attended-check-2026-11-10)
  - [S8 dark at start](#s8-dark-at-start)

<?/toc?>

## Status

| Part                                                                         | Result                                                                                                                                                                                        |
| ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Translator against the catalog                                               | Unit tests, including a replay of the captured reports through the real decoder                                                                                                               |
| Startup with the S8 connected                                                | Run once, unattended: MIDI source created, HID opened, no input in 3 s                                                                                                                        |
| MIDI from physical controls                                                  | Five attended checks, 2026-11-10. The user went down the checklist and states that every row not reported as broken works; see [Results](#results)                                            |
| Djay reaction with the generated mapping                                     | As above. Open: CAPTURE                                                                                                                                                                       |
| Touchstrip                                                                   | Reported coarse after the fourth check; reported working in the fifth, including a small back-and-forth motion on the left strip, with no change to the bridge. See [Touchstrip](#touchstrip) |
| Shift + touchstrip                                                           | Reported working for broad motion in the earlier checks; not repeated in the fifth                                                                                                            |
| CAPTURE                                                                      | Triggers "Record Sample" with no visible result in the sampler (first check). Deferred                                                                                                        |
| Pad-mode notes, FX slots 1 and 2, FX buttons, dial 4, BACK, FX SELECT, TEMPO | Reported working in the second check                                                                                                                                                          |
| FX dial 3                                                                    | Reported working in the third check, after a full sweep                                                                                                                                       |
| REMIX toggling the sampler                                                   | Reported working in the third check                                                                                                                                                           |
| CUE VOL, CUE MIX                                                             | Reported working in Djay after the third check                                                                                                                                                |
| BOOTH                                                                        | Reported visible in Djay's MIDI configuration; **audio not checked**                                                                                                                          |
| LOOP, Shift + HOTCUE, QUANTIZE                                               | Reported working in the fourth check                                                                                                                                                          |
| SNAP, MIC 2                                                                  | Reported visible in Djay's MIDI configuration in the fourth check                                                                                                                             |
| MAIN                                                                         | Reported working in Djay with the mapping regenerated after the fourth check                                                                                                                  |
| Behaviour with NIHardwareAgent running                                       | **Not checked**                                                                                                                                                                               |

Each attended check ran against the layout of its time, and the layout
changed after each in response. The rows changed after a check were reported
working in a later one, except CAPTURE.

## What the unit tests cover

- Buttons send note on and note off on the deck channel, and on channel + 8
  while Shift is held. A note is released on the address it was pressed on,
  even when Shift, the deck or the pad mode changed in between.
- DECK switches a side between A/C or B/D and sends the deck-select note.
- HOTCUE, Shift + HOTCUE, FREEZE and REMIX change the pad layer of their side
  and send their pad-mode note. LOOP sends its note and leaves the pad layer.
- MIC 2 and SNAP send their own notes, which the generated mapping leaves
  unbound.
- A control bound to several Djay actions sends one message.
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
- SNAP and MIC 2 send notes that the generated mapping leaves unbound.
- With a crossfader assign switch away from the middle, moving the crossfader
  made Djay show its "Crossfader assignment changed" dialog in the third
  check. In the fourth it appeared on the first button press after start,
  when the bridge sends the switch positions. The cause is not established;
  see [mapping.md](mapping.md).
- The Looper's pad triggers are not mapped; LOOP only shows or hides it.
- There is no LED or screen feedback; the mapping has no outputs.
- `--midi` without `full`, `--led-config`, `--tension-config`, `--jog-config`,
  `--screens` and `--screen-control-stdin` are refused for the S8.
- Started once with Controller Editor closed, the S8 was dark and the bridge
  was of no use until Controller Editor was opened again; see
  [S8 dark at start](#s8-dark-at-start).

## Attended check

Needs the S8, Djay Pro and a person at the controller. Reading HID input is
passive; this check sends nothing to the S8.

`mise run s8:midi` (`translator-check.sh`) does setup steps 1 to 3 and
restores NIHardwareAgent when the bridge stops. It keeps an existing
`Kontrol S8 Bridge` mapping, and writes the bridge log to
`docs/s8/captures/translator-<UTC time>/`. The manual steps are:

Open Controller Editor first and check that both panels show its labels.
If the S8 is dark and the panels ask for MIDI mode with SHIFT + BACK, end the
check with Ctrl-C, open Controller Editor and start again
([S8 dark at start](#s8-dark-at-start)).

### Setup

1. Generate the mapping into Djay's mapping folder:

       cargo run --locked -- --device s8 \
           --generate-mapping ~/Music/djay/"MIDI Mappings"

   This writes `Kontrol S8 Bridge.djayMidiMapping`. An existing file with that
   name is not overwritten.

2. Stop NIHardwareAgent. While it runs, it also turns S8 input into MIDI.
   That MIDI is for the Controller Editor template. Djay would then receive
   two sets of commands. Restore the agent afterwards with
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

| #   | Action                                                                                    | Expected in Djay                                                                                                                                                            |
| --- | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | PLAY, CUE, SYNC on decks A and B                                                          | Play, cue and sync on the matching deck                                                                                                                                     |
| 2   | DECK on each side, then PLAY                                                              | The side controls C or D; PLAY acts on that deck                                                                                                                            |
| 3   | Shift + a control with a Shift row                                                        | The Shift action in [mapping.md](mapping.md)                                                                                                                                |
| 4   | HOTCUE, then pads 1-8                                                                     | Djay shows hot cue pads; hot cues set and jump                                                                                                                              |
| 5   | Shift + HOTCUE, then pads 1-8; LOOP twice                                                 | Djay shows bounce loop pads and loops play while held; LOOP shows and hides the Looper without changing the pads                                                            |
| 6   | FREEZE, then pads 1-8; then HOTCUE                                                        | Slicer opens and pads play slices; HOTCUE leaves the Slicer                                                                                                                 |
| 7   | Shift + FREEZE                                                                            | Slicer loop mode toggles                                                                                                                                                    |
| 8   | REMIX, then pads 1-8; REMIX again                                                         | Sampler pads play and the sampler opens; the second press closes it                                                                                                         |
| 9   | Hold a pad, change pad mode, release                                                      | The held pad's action ends; no stuck note                                                                                                                                   |
| 10  | Loop encoder turn and press; Shift + turn and press                                       | Loop length changes and the loop toggles; Shift skips and reloops                                                                                                           |
| 11  | Browse encoder turn and press; BACK, Shift + BACK, EDIT, FX SELECT                        | The library scrolls and the track loads into that side's deck; BACK switches the library table, Shift + BACK goes back; preview; FX SELECT expands or collapses the library |
| 12  | FLUX, CAPTURE                                                                             | Slip toggles; a sample records                                                                                                                                              |
| 13  | Touchstrip: touch, slide, release                                                         | Scratch motion while touched; nothing after release                                                                                                                         |
| 14  | Shift + touchstrip                                                                        | Seek                                                                                                                                                                        |
| 15  | TEMPO encoder and button                                                                  | The selected deck's tempo changes and resets                                                                                                                                |
| 16  | Channel faders, crossfader, EQ, gain, filter on all four channels                         | The matching mixer control follows                                                                                                                                          |
| 17  | FILTER ON off, turn the filter knob, FILTER ON on                                         | Filter stays neutral while off, then jumps to the knob                                                                                                                      |
| 18  | Crossfader assign switches, three positions per channel                                   | Channel assigned left, through or right                                                                                                                                     |
| 19  | CUE on each mixer channel; MIC                                                            | Pre-listen toggles on that channel; the microphone toggles                                                                                                                  |
| 20  | FX buttons and dials 1-3, button and dial 4, then a mixer FX button, then the dials again | Slots 1-3 switch on and follow their dial; button 4 toggles FX, dial 4 moves all three dry/wet; the FX of the newly chosen deck follows                                     |
| 21  | Stem faders and ON buttons, Shift + ON                                                    | Stem volume, mute and solo                                                                                                                                                  |
| 22  | QUANTIZE; SNAP, MIC 2; display buttons, screen encoders                                   | Quantize toggles; SNAP and MIC 2 appear in Djay's MIDI configuration and do nothing; no reaction to the rest                                                                |
| 23  | Ctrl-C                                                                                    | The bridge stops; the `Kontrol S8 MIDI Full` source disappears                                                                                                              |

After the check, restore NIHardwareAgent. Confirm that the panels show
Controller Editor labels. Confirm that PLAY works with the existing setup.

## Results

### First attended check, 2026-11-10

Run by the user with `mise run s8:midi` against the earlier layout, in which
HOTCUE, LOOP and REMIX sent nothing, FX SELECT was unassigned, BACK was
`libraryBack`, FX button 1 was `fxActive` and dial 1 was dry/wet. The user
reported that most controls worked and commented on the rows below. Rows not
listed were not reported individually.

| #   | Reported                                                                                                                 | Follow-up                                                                          |
| --- | ------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 7   | Works. The Slicer view cannot be closed from the S8                                                                      | HOTCUE and LOOP now send their pad-mode note                                       |
| 8   | Sampler pads work. The sampler does not open. REMIX, HOTCUE, LOOP and FREEZE do not show up in Djay's MIDI configuration | Pad-mode buttons now send notes; REMIX also sends `sampler.showSampler`            |
| 11  | Works. BACK is wanted as "Switch Library Table"; the user rebound it in Djay                                             | BACK and Shift + BACK swapped                                                      |
| 12  | Slip toggles. CAPTURE triggers "Record Sample" with no visible result in the sampler                                     | Deferred, low priority                                                             |
| 15  | Not run: the bridge log has no TEMPO encoder event                                                                       |                                                                                    |
| 20  | Button 1 was FX Active and dial 1 moved only FX 3 dry/wet; button and dial 2 were FX 1                                   | Slots moved to controls 1-3; dial 4 now sends one CC for all three dry/wet actions |
| 22  | FX SELECT did nothing, as designed then                                                                                  | FX SELECT now toggles the library                                                  |

Dial 1 moved only FX 3 because the translator sent the last of the dial's
three CC numbers instead of all three.

### Second attended check, 2026-11-10

Run by the user with `mise run s8:midi` and the revised mapping, in which
REMIX sent `padModeSampler` and `sampler.showSampler` and Shift + REMIX sent
`toggleSamplerShown`. Rows not listed were not reported.

| #   | Reported                                                                                                                                | Follow-up                                                                                |
| --- | --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| 6   | Works: HOTCUE and LOOP leave the Slicer. The user still sees nothing in Djay's MIDI configuration when the pad-mode buttons are pressed | Not explained                                                                            |
| 8   | The sampler does not open. The bridge log has REMIX presses but none with Shift held, so `toggleSamplerShown` was not exercised         | REMIX now sends `toggleSamplerShown` itself; `showSampler` and the Shift row are removed |
| 11  | Works                                                                                                                                   |                                                                                          |
| 15  | Works                                                                                                                                   |                                                                                          |
| 20  | Buttons 1-3 and dials 1 and 2 work. Dial 3 does nothing. Dial 4 moves all three dry/wet. Button 4 toggles FX; accepted as is            | Dial 3: see below                                                                        |
| 22  | FX SELECT works                                                                                                                         |                                                                                          |

The bridge read FX dial 3 throughout (337 reports, positions 0.50 to 0.96) and
the mapping row for it matches dials 1 and 2. The dial never went below its
centre in this run. The FX dials use Djay's pickup mode, so a parameter below
centre in Djay would not have been picked up; this is a suspicion, not a
finding. Sweep the dial through its full range in the next check.

CUE VOL, CUE MIX and BOOTH were raised as missing. The pinned Encdr S8
descriptor has no entry for them and no capture had moved them.

The bridge log of this check was written to `target/s8-captures/` and was
lost with the build cache, so the dial 3 figures above cannot be rechecked.

### Third attended check, 2026-11-10

Run by the user with the mapping regenerated after the second check. Rows
not listed were not reported.

| #   | Reported                                          | Follow-up                         |
| --- | ------------------------------------------------- | --------------------------------- |
| 8   | REMIX opens and closes the sampler                |                                   |
| 20  | Dial 3 works after a sweep through its full range | Consistent with pickup; no change |

The user then turned CUE VOL, CUE MIX and BOOTH in that order under
`--device s8 --raw`. Report 2 changed at `[65..66]`, `[63..64]` and
`[61..62]` in turn, each sweeping 11-22 to 4082-4091. No other field
moved. The assignment of fields to knobs rests on that order. The log is
`captures/2026-11-10-centre-knobs/`.

The three knobs are now decoded and mapped to `mixer.monitorLevel`,
`mixer.monitorMix` and `mixer.boothLevel`. The user then reported that CUE
VOL and CUE MIX work in Djay. BOOTH shows up in Djay's MIDI configuration.
The booth output was not listened to. Whether the S8 also applies the knobs
to its own outputs is not checked.

In the same check the user reported that moving the crossfader with any
crossfader assign switch away from the middle brings up Djay's "Crossfader
assignment changed" dialog. It does not with all four in the middle.

### Fourth attended check, 2026-11-10

Run by the user with `mise run s8:midi` and the mapping regenerated after
the third check. The bridge log is
`captures/translator-20261110T111803Z/bridge.log`. Rows not listed were not
reported.

| #   | Reported                                                                                                       | Follow-up                                                                                                                       |
| --- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| 5   | Shift + HOTCUE works. LOOP works, but its first press brought up Djay's "Crossfader assignment changed" dialog | The log shows that press in the first buttons report, together with the four crossfader assign positions, all middle. No change |
| 22  | QUANTIZE works. SNAP and MIC 2 show up in Djay's MIDI configuration                                            |                                                                                                                                 |

The user then turned MAIN under `--device s8 --raw`
(`captures/2026-11-10-main-knob/`). Report 2 `[59..60]` swept 13 to 4082.
The log also has two left PLAY presses and a short channel A fader move
before the sweep; no other field moved. MAIN is now decoded and mapped to
`mixer.masterLevel`. The user then reported that MAIN works in Djay. Whether
the S8 also applies the knob to its own main output is not checked.

### User statement after the fourth check, 2026-11-10

The user stated that in the checks they went down the list and tried
everything quickly. To the best of their knowledge, every row not reported
as broken works. On that statement all rows of the checklist are recorded
as checked and working in Djay, with these exceptions:

| #      | State                                                                                               |
| ------ | --------------------------------------------------------------------------------------------------- |
| 12     | CAPTURE: "Record Sample" triggers with no visible result. Deferred, unchanged since the first check |
| 13, 14 | Touchstrip: works, but only for broad scrubbing. See below                                          |

The rows were tried quickly and not logged one by one, so this is a general
statement by the user, not a per-row record. It does not cover the items
outside the checklist: the booth output, whether the S8 applies BOOTH and
MAIN to its own outputs, and behaviour with NIHardwareAgent running.

### Touchstrip

After the fourth check the user reported that the touchstrip scratches and
seeks, but that the motion is coarse: a broad slide scrubs, and jiggling a
finger back and forth in place does little or nothing. The fifth check below
did not reproduce this, and its cause is not established.

The generated mapping uses values close to Evan's mapping for this control:

| Mapping                | Action           | Type              | Flipped | Sensitivity | Acceleration |
| ---------------------- | ---------------- | ----------------- | ------- | ----------- | ------------ |
| Evan 1.1, decks 1-4    | `scratchingMove` | `rotary-absolute` | yes     | 16-20       | 9-14         |
| Bridge                 | `scratchingMove` | `rotary-absolute` | yes     | 18          | 11           |
| Evan 1.1, second layer | `jogSeekMove`    | `rotary-absolute` | no      | 7.1-7.7     | 0            |
| Bridge, Shift          | `jogSeekMove`    | `rotary-absolute` | no      | 7.4         | 0            |

An earlier version of this record explained the report by the 7-bit value
the bridge sends, `(position - 1) >> 3`: one step is 8 units of the strip,
so a motion within 8 units sends nothing. It rested on 29 strip events in
the fourth check's log. The comparison below shows that NIHardwareAgent
sends the same value, so this does not separate the bridge from the usual
setup.

#### Comparison with NIHardwareAgent, 2026-11-10

Read-only, with NIHardwareAgent running, Controller Editor open and Evan's
mapping selected in Djay. The user states that an in-place jiggle scratches
in this setup. `examples/midi-monitor.rs` listened on the agent's virtual
source `Traktor Kontrol S8 Input` while `--device s8 --raw` logged the strip
from interface 5 in shared mode. The user slid the left strip from end to
end and then jiggled in place. The logs are in
`captures/2026-11-10-agent-strip-midi/`.

- The agent sends the strip as an absolute 7-bit value on CC 2, channel 1,
  and note 2 while touched: velocity 127 at the touch, a note-on with
  velocity 0 at the release.
- The value equals `(position - 1) >> 3` of the last logged position in 844
  of 926 messages and differs by one in the rest.
- The translator's rule, applied to the decoded events of the raw log, gives
  934 messages against the agent's 934: the same eight touch notes, and a
  value sequence that differs in 54 places by an added, dropped or
  off-by-one value. This was computed from the log, not by running the
  bridge.
- The S8 reported the strip only after it had moved 3 units or more, with
  one move of 2, about every 13 ms during motion.

For the same motion the bridge sends Djay what the agent sends. A relative
encoding is not needed to match the usual setup, and none was built.

#### Fifth attended check, 2026-11-10

Run by the user with `mise run s8:midi` and Controller Editor quit. Two
mappings were installed: r6, and `Kontrol S8 Bridge r7`. r7 is a copy of r6
with the `scratchingMove` sensitivity and acceleration of Evan's mapping on
each deck (20 / 9, 18 / 10, 16 / 14, 19 / 11) and no other change. The user
selected r7 first. They reported that the in-place jiggle worked and felt
like Evan's mapping. They then selected r6 and reported that it felt the
same: "both of them work fine now".

The bridge log is `captures/translator-20261110T125820Z/bridge.log.gz`. It
has 15 touches of the left strip and no DECK or Shift press, so the motion
went to deck A on the base layer. Strip reports came about every 12.5 ms
during motion, as with the agent running. The shortest touches span 55 to
144 units of the strip, 7 to 13 values. The log does not mark the change of
mapping, so a touch cannot be assigned to r6 or r7.

Not covered by this check: the right strip, decks B to D and Shift +
touchstrip.

The catalog keeps the r6 values. r7 was made for this comparison only and
is not kept in `mappings/`.

### S8 dark at start

Reported by the user on 2026-11-10, after the fifth check: `mise run s8:midi`
started the bridge, but the S8 went dark and the displays said to start MIDI
mode with SHIFT + BACK. That did not work. The user pressed Ctrl-C, opened
Controller Editor again, and then it worked.

The fifth check had been run with Controller Editor quit, as its
instructions asked. It ended at 13:06 UTC with NIHardwareAgent restored by
the script. The bridge logs of the three starts that followed are in
`captures/`:

| Log                           | What it shows                                                                                                                       |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `translator-20261110T131032Z` | HID opened. The first input is Shift, then BACK, 46 s after start; one second later the bridge ends with a HID read error           |
| `translator-20261110T131159Z` | HID opened, no input in 39 s, ended with Ctrl-C. The running Controller Editor process was started during this run, at 13:12:23 UTC |
| `translator-20261110T131306Z` | HID opened, 747 input events in a minute                                                                                            |

The cause is not established. The logs do not show whether the S8 was
already dark before the script stopped the agent, or what SHIFT + BACK did
to the S8 apart from ending the bridge's HID read. The fifth check itself
ran with Controller Editor quit and received input throughout.

`translator-check.sh` now prints a warning when Controller Editor is not
running. It does not refuse to start.
