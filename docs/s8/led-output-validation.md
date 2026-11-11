---
summary: "Attended LED output check on the S8: the deck and mixer step plans and, once run, the answers (led-check.sh)."
type: validation
issues: [21, 24]
---

# S8 LED output check

Validation record for #21 and #24. The byte maps and the open questions are
in [led-output.md](led-output.md) and [led-output-mixer.md](led-output-mixer.md).

## Status

Run on 2026-11-11 by the user, at the S8. The deck plan is in
`captures/led-check-20261111T090918Z/`, run on branch commit `8e17946`. The
mixer plan is in `captures/led-check-mixer-20261111T092527Z/`, run on
`e855b5e`, which differs from `8e17946` only in the runner's terminal
styling. Before the rebase onto #119, which changed no code, these commits
were `59c2bf3` and `5c42d0f`.
NIHardwareAgent was stopped for each run and restored at its end:
both final answers to "Are the panel labels and LEDs back?" are `yes`.
The optional steps M16-54 to M16-71 were not run.

## What is sent

- Output reports `0x80` and `0x81` in the deck plan, 119 bytes each: the
  report ID and 118 payload bytes.
- Output report `0x82` in the mixer plan, 74 bytes: the report ID and 73
  payload bytes. The tool refuses `0x82` without `--allow-mixer`.
- Each report is one `write` to the HID collection on interface 5, opened in
  shared mode. A step with two writes waits 500 ms between them.
- No feature report is sent. The tool has no code path for `0xF3`, `0xF4`
  or any other feature report.
- Nothing is sent on interface 4 or to the screens. No panel picture
  changes because of this check, so there are no expected pictures.

Every byte is at most `0x7F`, the descriptor's logical maximum. The one
exception is step T9b, which sends `0xFF` to a single pad byte.

## How it runs

`mise run s8:led-check` runs the deck plan and `mise run s8:led-check mixer`
the mixer plan, both through `led-check.sh`. Step ids after the command
select steps, optional ones included. `mise run s8:led-check-plan` prints
every step with its full wire bytes and opens no device.

The runner does this in order:

1. It checks for one S8, one genuine NIHardwareAgent and no Traktor, Maschine
   or Komplete Kontrol.
2. It starts the passive recorder and asks for a left PLAY press.
3. It stops NIHardwareAgent with SIGKILL and asks for PLAY again.
4. For each step it sends the writes with `examples/s8-led-probe/`, prints
   the result of each write and asks the step's question.
5. On every exit it sends all-zero reports and starts NIHardwareAgent again.

Answering `stop` at any question skips to the last step. A failed write
does the same. Logs go to `docs/s8/captures/led-check-<UTC>/`.

While the agent is stopped, djay gets no MIDI from the Controller Editor
setup. The panels keep their last picture until the agent is back.

## Deck plan

Steps T0 to T13 answer the framing questions of #21:

- whether separate 119-byte writes work;
- whether a write replaces the whole side;
- the pad byte order and the brightness range;
- which LED each offset lights, and the strip direction.

| Step | Report         | Bytes set, others zero             | Expected                                                                            | Question                                                                   | Answer                                                                                                                                                                                                                              |
| ---- | -------------- | ---------------------------------- | ----------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| T0   | `0x80`, `0x81` | all zero then all zero             | Both writes return 119. Deck LEDs that NI left lit go dark; the mixer is unchanged. | Did any deck LEDs change? Did anything on the mixer change?                | most of the buttons went dark except the CUE for each deck, filter on each deck, and the left and right arrows at the top of each deck. also the mic 1 and mic 2 and snap and quantize buttons are still on. everything else is off |
| T1   | `0x80`         | `59=7f`                            | Left PLAY lit.                                                                      | Is left PLAY lit, and only it?                                             | yes                                                                                                                                                                                                                                 |
| T2   | `0x80`         | `58=7f`                            | CUE lit, PLAY dark.                                                                 | Is CUE lit and PLAY now dark?                                              | yes                                                                                                                                                                                                                                 |
| T3   | `0x80`         | `52=7f`                            | FLUX lit.                                                                           | Which button lit?                                                          | flux                                                                                                                                                                                                                                |
| T4   | `0x81`         | `59=7f`                            | Right PLAY lit.                                                                     | Is right PLAY lit? Did anything on the left change?                        | yes. no flux is still on                                                                                                                                                                                                            |
| T5   | `0x80`         | `52=7f`, `57=7f`                   | FLUX and SYNC lit.                                                                  | What colour is SYNC? Is a second SYNC LED (green) also lit?                | red. no green SYNC                                                                                                                                                                                                                  |
| T6   | `0x80`         | `56=7f`                            | SYNC (green?) or nothing.                                                           | Which LED, what colour?                                                    | left SYNC is green                                                                                                                                                                                                                  |
| T7a  | `0x80`         | `24-28=7f`                         | Left deck LEDs at offsets 24-28 lit, all others dark.                               | Which physical LEDs lit (offsets 24-28), left to right? Colours?           | FX Select and the buttons below FX1-FX4                                                                                                                                                                                             |
| T7b  | `0x80`         | `29-36=7f`                         | Left deck LEDs at offsets 29-36 lit, all others dark.                               | Which physical LEDs lit (offsets 29-36), left to right? Colours?           | left deck LEDs, 4 on the left, 4 on the right of the left deck display                                                                                                                                                              |
| T7c  | `0x80`         | `37-39=7f`                         | Left deck LEDs at offsets 37-39 lit, all others dark.                               | Which physical LEDs lit (offsets 37-39), left to right? Colours?           | Back, Capture, Edit (from top to bottom)                                                                                                                                                                                            |
| T7d  | `0x80`         | `40-43=7f`                         | Left deck LEDs at offsets 40-43 lit, all others dark.                               | Which physical LEDs lit (offsets 40-43), left to right? Colours?           | On Buttons 1-4 below left deck display                                                                                                                                                                                              |
| T7e  | `0x80`         | `44-51=7f`                         | Left deck LEDs at offsets 44-51 lit, all others dark.                               | Which physical LEDs lit (offsets 44-51), left to right? Colours?           | top to bottom: hotcue, loop, freeze, remix. all white                                                                                                                                                                               |
| T7f  | `0x80`         | `53-55=7f`                         | Left deck LEDs at offsets 53-55 lit, all others dark.                               | Which physical LEDs lit (offsets 53-55), left to right? Colours?           | Deck and Shift, both white                                                                                                                                                                                                          |
| T7g  | `0x80`         | `60-63=7f`                         | Left deck LEDs at offsets 60-63 lit, all others dark.                               | Which physical LEDs lit (offsets 60-63), left to right? Colours?           | Loop encoder is white                                                                                                                                                                                                               |
| T7h  | `0x80`         | `64-67=7f`                         | Left deck LEDs at offsets 64-67 lit, all others dark.                               | Which physical LEDs lit (offsets 64-67), left to right? Colours?           | Loop encoder is blue                                                                                                                                                                                                                |
| T8a  | `0x80`         | `0=10`                             | Pad 1 shows one of red, green or blue.                                              | Pad 1 colour for this write (offset 0, byte 0)?                            | red                                                                                                                                                                                                                                 |
| T8b  | `0x80`         | `1=10`                             | Pad 1 shows one of red, green or blue.                                              | Pad 1 colour for this write (offset 1, byte 1)?                            | green                                                                                                                                                                                                                               |
| T8c  | `0x80`         | `2=10`                             | Pad 1 shows one of red, green or blue.                                              | Pad 1 colour for this write (offset 2, byte 2)?                            | blue                                                                                                                                                                                                                                |
| T9a  | `0x80`         | `0=7f`                             | Pad 1 byte 0 at 0x7F.                                                               | Pad 1 colour and brightness?                                               | bright red                                                                                                                                                                                                                          |
| T9b  | `0x80`         | `0=ff` (above the logical maximum) | Brighter than T9a, or equal.                                                        | Is this brighter than the previous write, the same, or a different colour? | same                                                                                                                                                                                                                                |
| T10a | `0x80`         | `68=7f`                            | One blue strip LED lit (offset 68).                                                 | Which position on the strip (left, middle, right), what colour?            | left. blue                                                                                                                                                                                                                          |
| T10b | `0x80`         | `80=7f`                            | One blue strip LED lit (offset 80).                                                 | Which position on the strip (left, middle, right), what colour?            | middle blue                                                                                                                                                                                                                         |
| T10c | `0x80`         | `92=7f`                            | One blue strip LED lit (offset 92).                                                 | Which position on the strip (left, middle, right), what colour?            | right blue                                                                                                                                                                                                                          |
| T11a | `0x80`         | `93=7f`                            | One orange strip LED lit (offset 93).                                               | Which position on the strip (left, middle, right), what colour?            | left orange                                                                                                                                                                                                                         |
| T11b | `0x80`         | `117=7f`                           | One orange strip LED lit (offset 117).                                              | Which position on the strip (left, middle, right), what colour?            | right orange                                                                                                                                                                                                                        |
| T12  | `0x80`         | `68-117=7f`                        | Blue and orange banks both lit (Encdr says purple).                                 | Do both banks light, what is the colour?                                   | the top row of the left touchstrip is all purple. bottom row is off                                                                                                                                                                 |
| T13  | `0x80`, `0x80` | `59=7f` then `52=7f`               | PLAY lit after the first write, dark after the second (FLUX lit).                   | Did PLAY go dark?                                                          | yes, play went off, flux went on                                                                                                                                                                                                    |

## Mixer plan

Steps M1 to M17 cover report `0x82` only (#24). M2 sets the seed state; each
later step up to M10b changes it in one place. M11a to M14 test the claimed
VU ranges, and M15a and M15b the unmapped bytes.

| Step | Report | Bytes set, others zero                                                                   | Expected                                                                   | Question                                                                                                                  | Answer                                                                                          |
| ---- | ------ | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| M1   | `0x82` | all zero                                                                                 | Every LED that a 0x82 byte controls goes off.                              | Did any mixer LED change? Which?                                                                                          | no                                                                                              |
| M2   | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=7f`                | Seed state: FX C-1, FX D-2, SNAP, QUANT lit; the other FX assign LEDs dim. | Are FX C-1, FX D-2, SNAP and QUANT lit and FX A-1, A-2, B-1, B-2, C-2, D-1 dim? Answer yes or no for each.                | yes                                                                                             |
| M3   | `0x82` | `11-12=7f`, `44=7f`, `45-48=14`, `49=7f`, `50-52=14`, `53=7f`, `72=7f`                   | SNAP dim.                                                                  | Did SNAP go dim?                                                                                                          | yes                                                                                             |
| M4   | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48=7f`, `49-52=14`, `53=7f`, `72=7f`                   | QUANT dim.                                                                 | Did QUANT go dim?                                                                                                         | yes                                                                                             |
| M5   | `0x82` | `11-12=7f`, `44-47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=7f`                         | FX C-1 dim.                                                                | Did FX C-1 dim?                                                                                                           | yes                                                                                             |
| M6   | `0x82` | `11-12=7f`, `44=7f`, `45=14`, `46=7f`, `47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=7f` | FX A-1 lit.                                                                | Did FX A-1 light?                                                                                                         | Yes FX A-1 and FX C-1 came on                                                                   |
| M7   | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-50=7f`, `51-52=14`, `53=7f`, `72=7f`                | FX B-1 lit.                                                                | Did FX B-1 light?                                                                                                         | yes                                                                                             |
| M8   | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-49=7f`, `50-51=14`, `52-53=7f`, `72=7f`             | FX D-1 lit.                                                                | Did FX D-1 light?                                                                                                         | yes FX D-1 and FX D-2 are on                                                                    |
| M9   | `0x82` | `11-12=7f`, `44=7f`, `45=14`, `47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=7f`          | FX A-1 fully off, or still dim.                                            | Does 00 turn FX A-1 fully off, compared with the dim 14?                                                                  | yes                                                                                             |
| M10a | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=14`                | Unknown: some LED changes, or nothing.                                     | Which LED changed, if any?                                                                                                | FX A-1 is back on                                                                               |
| M10b | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-49=7f`, `50-52=14`, `53=7f`                         | Unknown: some LED changes, or nothing.                                     | Which LED changed, if any, compared with the previous write?                                                              | FX A-1 went dim                                                                                 |
| M11a | `0x82` | `11-21=7f`                                                                               | VU channel 1 segments lit (claimed range 11-21).                           | Did any VU segment light? Which channel, how many segments, bottom up? Do the meters still move with audio by themselves? | all four decks' FX 1 and FX 2 buttons went off                                                  |
| M11b | `0x82` | `11=7f`                                                                                  | One VU channel 1 segment lit.                                              | Which segment lit (bottom, top, other)? Do the meters still move with audio by themselves?                                | none                                                                                            |
| M11c | `0x82` | `21=7f`                                                                                  | One VU channel 1 segment lit.                                              | Which segment lit (bottom, top, other)? Do the meters still move with audio by themselves?                                | none                                                                                            |
| M12  | `0x82` | `0-10=7f`                                                                                | VU channel 3 segments lit (claimed range 0-10).                            | Did any VU segment light? Which channel, how many segments, bottom up? Do the meters still move with audio by themselves? | none                                                                                            |
| M13  | `0x82` | `22-32=7f`                                                                               | VU channel 2 segments lit (claimed range 22-32).                           | Did any VU segment light? Which channel, how many segments, bottom up? Do the meters still move with audio by themselves? | none                                                                                            |
| M14  | `0x82` | `33-43=7f`                                                                               | VU channel 4 segments lit (claimed range 33-43).                           | Did any VU segment light? Which channel, how many segments, bottom up? Do the meters still move with audio by themselves? | no, none                                                                                        |
| M15a | `0x82` | `54-62=7f`                                                                               | Unknown: some LED lit, or nothing.                                         | Which LED lit, if any? Where on the mixer?                                                                                | none                                                                                            |
| M15b | `0x82` | `63-71=7f`                                                                               | Unknown: some LED lit, or nothing.                                         | Which LED lit, if any? Where on the mixer?                                                                                | none                                                                                            |
| M17  | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=7f`                | Seed state again.                                                          | Are the mixer LEDs back in the M2 state?                                                                                  | decks 1-4's FX 1-2 all came on and so did snap and quantize. but FX C-1 and FX D-2 are brighter |

Steps M16-54 to M16-71 are optional. Each sets one byte from 54 to 71 to
`7f` and asks "Which LED lit, if any? Where on the mixer?". They run only
when M15a or M15b lights something and are named on the command line.

## Left for a later session

- Feature reports `0xF3` and `0xF4`: cue and filter LEDs and the mode
  switch. They need their own plan and approval.
- DIRECT THRU (`0xF4` command `0x24`) changes the audio path. It is not
  part of any plan.

## Results

Answers are copied unchanged from `notes.txt` of each run folder into the
Answer column above. Interpretation is in the lists below and is worded as
inference where it is one.

### Write results

- Deck run: 27 steps, T0 to T13, every one with exit status 0. Every write
  returned 119 for a request of 119: 29 writes in the steps (`0x80` 27,
  `0x81` 2) and two more in the final clear (`0x80`, `0x81`).
- Mixer run: 20 steps, M1 to M17 without M16, every one with exit status 0.
  Every write returned 74 for a request of 74: one per step, 20 in all,
  and one more in the final clear, next to `0x80` and `0x81` returning 119.
- The device was opened as interface 5, usage page `0xff01`, usage `0x0000`.
- No write failed and no step was answered `stop`.

### Recovery answers

- Deck run, "Are the panel labels and LEDs back?": `yes`.
- Mixer run, "Are the panel labels and LEDs back?": `yes`.
- M17, "Are the mixer LEDs back in the M2 state?": the answer is in the table.
  It is not a clean return to the M2 state: see Not settled.

### Established

Only what the answers show directly.

- A plain `write` of 119 bytes (report ID `0x80` or `0x81` plus 118 payload
  bytes) is accepted on this Mac and returns 119. A write of 74 bytes for
  `0x82` returns 74.
- `0x80` and `0x81` are independent: T4 (`0x81`, right PLAY) changed nothing
  on the left, and FLUX from T3 stayed lit.
- A write replaces the whole side: T2 (`58=7f`) lit CUE and left PLAY dark;
  T13 turned PLAY off and FLUX on with a second write.
- `0x80` offset 59 is PLAY (T1), 58 is CUE (T2), 52 is FLUX (T3).
- Offset 57 is a red SYNC, with no green SYNC lit (T5). Offset 56 is the
  left SYNC in green (T6).
- Offsets 24-28 lit FX Select and the buttons below FX1-FX4 (T7a).
- Offsets 29-36 lit the display-side buttons, 4 on each side of the left
  deck display (T7b).
- Offsets 37-39 lit Back, Capture and Edit (T7c).
- Offsets 40-43 lit On buttons 1-4 below the left deck display (T7d).
- Offsets 44-51 lit hotcue, loop, freeze and remix, all white (T7e).
- Offsets 53-55 lit Deck and Shift, both white (T7f).
- Offsets 60-63 lit the loop encoder in white (T7g) and 64-67 in blue (T7h).
- Pad 1: byte 0 is red (T8a), byte 1 green (T8b), byte 2 blue (T8c), so the
  order is R, G, B. `0x7F` on byte 0 is bright red (T9a).
- `0xFF` on pad 1 byte 0 looked the same as `0x7F` (T9b).
- Touchstrip: offset 68 is blue on the left (T10a), 80 blue in the middle
  (T10b), 92 blue on the right (T10c). Offset 93 is orange on the left
  (T11a) and 117 orange on the right (T11b).
- Mixer `0x82`: offsets 44-53 control the FX assign LEDs, SNAP and QUANT as
  mapped, with three levels: `00` off (M9), `14` dim, `7f` lit (M2-M8).
  Offset 46 is FX A-1 (M6, M9), 48 SNAP (M3), 49 QUANT (M4), 44 FX C-1 (M5),
  50 FX B-1 (M7), 52 FX D-1 (M8).
- No VU segment lit for any claimed VU range, offsets 0-10, 11-21, 22-32 and
  33-43, with the agent stopped (M11b, M11c, M12, M13, M14; the M11a answer
  names only FX buttons).
- Nothing lit for offsets 54-62 or 63-71 (M15a, M15b).
- After the final clear the user reported the panel labels and LEDs back in
  both runs.

### Not settled

- T0: after the two all-zero writes some LEDs stayed lit: CUE on each deck,
  filter on each deck, the left and right arrows at the top of each deck,
  mic 1, mic 2, SNAP and QUANT. `[INFERENCE]` The deck ones are not
  controlled by an `0x80` or `0x81` byte, or zero does not extinguish
  them; they may be fed by another report, such as `0xF4`. SNAP and QUANT
  are `0x82` LEDs, which T0 did not write.
- Order inside the deck groups: the T7a, T7b and T7e answers name the LEDs
  of a group, not which offset lit which one. Only the group is known.
- Single-byte colours of offsets 44-51 and 53-54: T7e and T7f lit them all
  together and all showed white, so which LED is each of the white and blue
  companions (44/45, 46/47, 48/49, 50/51, 53/54) is not known. No blue was
  reported in T7e or T7f.
- The dim level `0x14` on deck LEDs was not tested: no deck step sent it.
  Whether `0x00` extinguishes a `0x14`-class deck LED is open.
- T12 (`68-117=7f`): only the top row of the left touchstrip lit, in
  purple, and the bottom row was off. The expectation was both banks lit.
  Which bank is the top row, and why the bottom row stayed off, is
  unexplained.
- T9b: `0xFF` looked the same as `0x7F` on one pad byte. Whether the device
  clamps, wraps or ignores bit 7 is not shown, and strips were not tested.
- Right deck (`0x81`) offsets were tested only for PLAY (T4). The rest of
  the right-side map is not checked.
- M1: all-zero `0x82` was answered `no`, yet `0x82` LEDs were lit in later
  steps. Either they were already off or the zeros do not extinguish them;
  the answer does not say which.
- M10a and M10b, byte 72: the answers name FX A-1 only (back on, then dim).
  The plan bytes do not explain it: M10a changes byte 72 from `7f` to `14`
  and restores byte 46 to `14` after M9's `00`; M10b only removes byte 72.
  What byte 72 controls is not settled.
- M17: the answer says FX 1-2 of decks 1-4, SNAP and QUANT came on, but FX
  C-1 and FX D-2 are "brighter". It does not say that the other FX assign
  LEDs were dim, as in M2.
- M11a to M14, VU ranges: no segment lit, so segment order, clip LEDs and
  channel mapping are not known. Whether the firmware overrides the write,
  a mode is needed, or the bytes are not VU is not shown.
- The "do the meters still move with audio by themselves" part of the
  questions of M11a to M14 was not answered in any step.
- M11a: the answer says all four decks' FX 1 and FX 2 buttons went off.
  Bytes 44-53 were not in that write. `[INFERENCE]` The write replaced the
  whole report; no VU segment is named.
- Bytes 54-71: nothing was seen, and the optional M16 steps were not run.
- `0xF3` and `0xF4`: no step sent them.

### After the mixer run

The user said after the mixer run, outside the log: "none of the VU meters
worked. seemed like maybe half of the controls weren't working. whereas
almost all the other tests we've done have worked"
