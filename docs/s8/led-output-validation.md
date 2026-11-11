---
summary: "Attended LED output check on the S8: the deck and mixer step plans and, once run, the answers (led-check.sh)."
type: validation
issues: [21, 24]
---

# S8 LED output check

Validation record for #21 and #24. The byte maps and the open questions are
in [led-output.md](led-output.md) and [led-output-mixer.md](led-output-mixer.md).

## Status

Not run. Nothing has been sent to the S8 for this check. The plan below
needs approval first, and so does stopping NIHardwareAgent.

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

| Step | Report         | Bytes set, others zero             | Expected                                                                            | Question                                                                   | Answer  |
| ---- | -------------- | ---------------------------------- | ----------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------- |
| T0   | `0x80`, `0x81` | all zero then all zero             | Both writes return 119. Deck LEDs that NI left lit go dark; the mixer is unchanged. | Did any deck LEDs change? Did anything on the mixer change?                | Not run |
| T1   | `0x80`         | `59=7f`                            | Left PLAY lit.                                                                      | Is left PLAY lit, and only it?                                             | Not run |
| T2   | `0x80`         | `58=7f`                            | CUE lit, PLAY dark.                                                                 | Is CUE lit and PLAY now dark?                                              | Not run |
| T3   | `0x80`         | `52=7f`                            | FLUX lit.                                                                           | Which button lit?                                                          | Not run |
| T4   | `0x81`         | `59=7f`                            | Right PLAY lit.                                                                     | Is right PLAY lit? Did anything on the left change?                        | Not run |
| T5   | `0x80`         | `52=7f`, `57=7f`                   | FLUX and SYNC lit.                                                                  | What colour is SYNC? Is a second SYNC LED (green) also lit?                | Not run |
| T6   | `0x80`         | `56=7f`                            | SYNC (green?) or nothing.                                                           | Which LED, what colour?                                                    | Not run |
| T7a  | `0x80`         | `24-28=7f`                         | Left deck LEDs at offsets 24-28 lit, all others dark.                               | Which physical LEDs lit (offsets 24-28), left to right? Colours?           | Not run |
| T7b  | `0x80`         | `29-36=7f`                         | Left deck LEDs at offsets 29-36 lit, all others dark.                               | Which physical LEDs lit (offsets 29-36), left to right? Colours?           | Not run |
| T7c  | `0x80`         | `37-39=7f`                         | Left deck LEDs at offsets 37-39 lit, all others dark.                               | Which physical LEDs lit (offsets 37-39), left to right? Colours?           | Not run |
| T7d  | `0x80`         | `40-43=7f`                         | Left deck LEDs at offsets 40-43 lit, all others dark.                               | Which physical LEDs lit (offsets 40-43), left to right? Colours?           | Not run |
| T7e  | `0x80`         | `44-51=7f`                         | Left deck LEDs at offsets 44-51 lit, all others dark.                               | Which physical LEDs lit (offsets 44-51), left to right? Colours?           | Not run |
| T7f  | `0x80`         | `53-55=7f`                         | Left deck LEDs at offsets 53-55 lit, all others dark.                               | Which physical LEDs lit (offsets 53-55), left to right? Colours?           | Not run |
| T7g  | `0x80`         | `60-63=7f`                         | Left deck LEDs at offsets 60-63 lit, all others dark.                               | Which physical LEDs lit (offsets 60-63), left to right? Colours?           | Not run |
| T7h  | `0x80`         | `64-67=7f`                         | Left deck LEDs at offsets 64-67 lit, all others dark.                               | Which physical LEDs lit (offsets 64-67), left to right? Colours?           | Not run |
| T8a  | `0x80`         | `0=10`                             | Pad 1 shows one of red, green or blue.                                              | Pad 1 colour for this write (offset 0, byte 0)?                            | Not run |
| T8b  | `0x80`         | `1=10`                             | Pad 1 shows one of red, green or blue.                                              | Pad 1 colour for this write (offset 1, byte 1)?                            | Not run |
| T8c  | `0x80`         | `2=10`                             | Pad 1 shows one of red, green or blue.                                              | Pad 1 colour for this write (offset 2, byte 2)?                            | Not run |
| T9a  | `0x80`         | `0=7f`                             | Pad 1 byte 0 at 0x7F.                                                               | Pad 1 colour and brightness?                                               | Not run |
| T9b  | `0x80`         | `0=ff` (above the logical maximum) | Brighter than T9a, or equal.                                                        | Is this brighter than the previous write, the same, or a different colour? | Not run |
| T10a | `0x80`         | `68=7f`                            | One blue strip LED lit (offset 68).                                                 | Which position on the strip (left, middle, right), what colour?            | Not run |
| T10b | `0x80`         | `80=7f`                            | One blue strip LED lit (offset 80).                                                 | Which position on the strip (left, middle, right), what colour?            | Not run |
| T10c | `0x80`         | `92=7f`                            | One blue strip LED lit (offset 92).                                                 | Which position on the strip (left, middle, right), what colour?            | Not run |
| T11a | `0x80`         | `93=7f`                            | One orange strip LED lit (offset 93).                                               | Which position on the strip (left, middle, right), what colour?            | Not run |
| T11b | `0x80`         | `117=7f`                           | One orange strip LED lit (offset 117).                                              | Which position on the strip (left, middle, right), what colour?            | Not run |
| T12  | `0x80`         | `68-117=7f`                        | Blue and orange banks both lit (Encdr says purple).                                 | Do both banks light, what is the colour?                                   | Not run |
| T13  | `0x80`, `0x80` | `59=7f` then `52=7f`               | PLAY lit after the first write, dark after the second (FLUX lit).                   | Did PLAY go dark?                                                          | Not run |

## Mixer plan

Steps M1 to M17 cover report `0x82` only (#24). M2 sets the seed state; each
later step up to M10b changes it in one place. M11a to M14 test the claimed
VU ranges, and M15a and M15b the unmapped bytes.

| Step | Report | Bytes set, others zero                                                                   | Expected                                                                   | Question                                                                                                                  | Answer  |
| ---- | ------ | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ------- |
| M1   | `0x82` | all zero                                                                                 | Every LED that a 0x82 byte controls goes off.                              | Did any mixer LED change? Which?                                                                                          | Not run |
| M2   | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=7f`                | Seed state: FX C-1, FX D-2, SNAP, QUANT lit; the other FX assign LEDs dim. | Are FX C-1, FX D-2, SNAP and QUANT lit and FX A-1, A-2, B-1, B-2, C-2, D-1 dim? Answer yes or no for each.                | Not run |
| M3   | `0x82` | `11-12=7f`, `44=7f`, `45-48=14`, `49=7f`, `50-52=14`, `53=7f`, `72=7f`                   | SNAP dim.                                                                  | Did SNAP go dim?                                                                                                          | Not run |
| M4   | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48=7f`, `49-52=14`, `53=7f`, `72=7f`                   | QUANT dim.                                                                 | Did QUANT go dim?                                                                                                         | Not run |
| M5   | `0x82` | `11-12=7f`, `44-47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=7f`                         | FX C-1 dim.                                                                | Did FX C-1 dim?                                                                                                           | Not run |
| M6   | `0x82` | `11-12=7f`, `44=7f`, `45=14`, `46=7f`, `47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=7f` | FX A-1 lit.                                                                | Did FX A-1 light?                                                                                                         | Not run |
| M7   | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-50=7f`, `51-52=14`, `53=7f`, `72=7f`                | FX B-1 lit.                                                                | Did FX B-1 light?                                                                                                         | Not run |
| M8   | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-49=7f`, `50-51=14`, `52-53=7f`, `72=7f`             | FX D-1 lit.                                                                | Did FX D-1 light?                                                                                                         | Not run |
| M9   | `0x82` | `11-12=7f`, `44=7f`, `45=14`, `47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=7f`          | FX A-1 fully off, or still dim.                                            | Does 00 turn FX A-1 fully off, compared with the dim 14?                                                                  | Not run |
| M10a | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=14`                | Unknown: some LED changes, or nothing.                                     | Which LED changed, if any?                                                                                                | Not run |
| M10b | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-49=7f`, `50-52=14`, `53=7f`                         | Unknown: some LED changes, or nothing.                                     | Which LED changed, if any, compared with the previous write?                                                              | Not run |
| M11a | `0x82` | `11-21=7f`                                                                               | VU channel 1 segments lit (claimed range 11-21).                           | Did any VU segment light? Which channel, how many segments, bottom up? Do the meters still move with audio by themselves? | Not run |
| M11b | `0x82` | `11=7f`                                                                                  | One VU channel 1 segment lit.                                              | Which segment lit (bottom, top, other)? Do the meters still move with audio by themselves?                                | Not run |
| M11c | `0x82` | `21=7f`                                                                                  | One VU channel 1 segment lit.                                              | Which segment lit (bottom, top, other)? Do the meters still move with audio by themselves?                                | Not run |
| M12  | `0x82` | `0-10=7f`                                                                                | VU channel 3 segments lit (claimed range 0-10).                            | Did any VU segment light? Which channel, how many segments, bottom up? Do the meters still move with audio by themselves? | Not run |
| M13  | `0x82` | `22-32=7f`                                                                               | VU channel 2 segments lit (claimed range 22-32).                           | Did any VU segment light? Which channel, how many segments, bottom up? Do the meters still move with audio by themselves? | Not run |
| M14  | `0x82` | `33-43=7f`                                                                               | VU channel 4 segments lit (claimed range 33-43).                           | Did any VU segment light? Which channel, how many segments, bottom up? Do the meters still move with audio by themselves? | Not run |
| M15a | `0x82` | `54-62=7f`                                                                               | Unknown: some LED lit, or nothing.                                         | Which LED lit, if any? Where on the mixer?                                                                                | Not run |
| M15b | `0x82` | `63-71=7f`                                                                               | Unknown: some LED lit, or nothing.                                         | Which LED lit, if any? Where on the mixer?                                                                                | Not run |
| M17  | `0x82` | `11-12=7f`, `44=7f`, `45-47=14`, `48-49=7f`, `50-52=14`, `53=7f`, `72=7f`                | Seed state again.                                                          | Are the mixer LEDs back in the M2 state?                                                                                  | Not run |

Steps M16-54 to M16-71 are optional. Each sets one byte from 54 to 71 to
`7f` and asks "Which LED lit, if any? Where on the mixer?". They run only
when M15a or M15b lights something and are named on the command line.

## Left for a later session

- Feature reports `0xF3` and `0xF4`: cue and filter LEDs and the mode
  switch. They need their own plan and approval.
- DIRECT THRU (`0xF4` command `0x24`) changes the audio path. It is not
  part of any plan.

## Results

None yet. After the run this section records the answers as given, the
write results from the log and the rows that stayed unsettled.
