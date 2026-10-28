# S8 screen phases 2–4: first frames and full frame

Validation record for #34 and #35 ([screens.md](screens.md), phases 2–4).
Every frame was a single bulk OUT transfer on interface 6, endpoint `0x04`,
with a 500 ms timeout and no retries, sent by `examples/s8-screen-frame.rs`.
Nothing was sent on interface 5. No transfer touched the configuration, did a
reset or changed the alternate setting.

## Answer

**The S8 accepts frames from the bridge's process on macOS. The pixels are
RGB565, big-endian.** Both the observed 32-byte run-length frames and one full
uncompressed LITERAL frame display correctly on both panels. The `0xF8`
feature report is not needed: phase 3 did not run.

| Question | Result |
| -------- | ------ |
| `BLACK_0` replay, byte for byte | Left panel black |
| Screen byte | `00` left, `01` right, in header byte 2 and footer byte 6 |
| Colour order | RGB565: `F8 00` shows red, not blue; the RGB-encoded pattern's bars match their labels |
| Full LITERAL frame (261,148 bytes) | Displays correctly on both panels, about 16 ms per transfer |
| Init (`0xF8` on interface 5) | Not needed |
| After release, agent still stopped | The last frame stays on the panels |
| After `open -g` relaunch | The agent redraws Controller Editor labels on both panels; Djay reacts to left PLAY |
| Side effects | No LED or audio change, no re-enumeration, HID input continued |

Photos of every step were taken at the S8; they stay on the tester's phone
and are not part of the repository.

## Setup

As in [screen-claim-validation.md](screen-claim-validation.md): macOS 27.2,
Apple Silicon, one S8 in Controller Editor MIDI mode with Evan's template,
Djay Pro open with no track playing. The passive recorder ran on interface 5
HID and every `Kontrol S8` MIDI endpoint. Runs on 2026-10-28 with
`mise run s8:screen-frame first` (08:53–08:58Z) and
`mise run s8:screen-frame pattern rgb` (08:59–09:03Z), both through
`screen-frame-check.sh`. Raw logs stayed outside the repository.

## Phase 2: first frames

| Step | Bytes | SHA-256 | Transfer | Panels |
| ---- | ----- | ------- | -------- | ------ |
| Black, left | `84000060000000000000000001e001100100ff00000000000300000040000000` | `4dac8e38…` | 32 of 32, OK | Left black, right unchanged |
| `F8 00`, left | `84000060000000000000000001e001100100ff00f800f8000300000040000000` | `37e8ba5f…` | 32 of 32, OK | Left solid light orange/peach, right unchanged |
| Black, right | `84000160000000000000000001e001100100ff00000000000300000040000100` | `a7cf0dc7…` | 32 of 32, OK | Right black, left still orange |
| `F8 00`, right | `84000160000000000000000001e001100100ff00f800f8000300000040000100` | `222117be…` | 32 of 32, OK | Right the same orange/peach as left |

- The left black frame is the observed `BLACK_0` payload; the tool checks
  that its frame builder reproduces it exactly.
- `F8 00` is pure red in RGB565 and pure blue in BGR565. The panel showed a
  red-family colour that looked light orange or peach to the eye, much
  brighter than the dim orange buttons around it. Nothing looked blue. The
  phase 4 pattern confirmed the order: its red, green and blue bars matched
  their labels.
- Each transfer finished within 1 ms of the claim; the tool released the
  interface right after it.

## Phase 4: full frame

| Step | Bytes | SHA-256 | Transfer | Panels |
| ---- | ----- | ------- | -------- | ------ |
| Pattern, left | 261,148 | `bd568210d97edd6e93e826b5560d771c84adab3fe3ebcb8f9a342ffa42c138e4` | 261,148 of 261,148, OK, 16 ms | Left exactly as intended, right unchanged |
| Pattern, right | 261,148 | `43f8d9ea9844cc5b1196fee734e64027a888f53fde19f781005df7433883148d` | 261,148 of 261,148, OK, 16 ms | Right exactly as intended, left still its pattern |

The frame is the 16-byte header, one `00 00 FF 00` LITERAL command, 480×272
RGB565 big-endian pixels and the 8-byte footer. The pattern has a dark
background, the panel title, red, green, blue and white bars with labels,
`480 X 272 LITERAL`, a 1-pixel grey border on all four edges, a 16-pixel
white marker top left and 8-pixel white markers in the other corners. The
tester confirmed every element on both panels: the full raster is visible,
with no cropping, offset or mirroring.

`s8-screen-frame pattern --preview FILE.ppm` writes the intended image for
comparison with photos.

## Input and MIDI per window

Counts from the passive recorder. Each frame window holds one left PLAY press
and release after the frame.

| Window | Phase 2 HID report 1 | Phase 4 HID report 1 | MIDI on `Traktor Kontrol S8 Input` |
| ------ | -------------------- | -------------------- | ---------------------------------- |
| Baseline | 2 | 2 | PLAY ch 1 CC 10 on/off |
| Agent stopped | 2 | 2 | port gone |
| Each frame | 2 | 2 | port gone |
| Recover | 4 | 2 | PLAY ch 1 CC 10 on/off (phase 2: twice); Djay reacted |

[INFERENCE] The second PLAY in the phase 2 recovery window paused the deck
again.

## Interface ownership

| Snapshot | Interface 6 | Device registry id |
| -------- | ----------- | ------------------ |
| Before, agent running | NIHardwareAgent | `0x1138e6119` |
| Agent stopped, and 1 s after each frame | no client | `0x1138e6119` |
| Recovered + 5 s | NIHardwareAgent (new pid) | `0x1138e6119` |

The S8 did not re-enumerate in either run.

## Not covered

- Frames while NIHardwareAgent runs, or with the interface held across
  frames; each step claimed and released interface 6.
- Partial updates (SKIP, sub-rectangles), refresh rate and latency under
  repeated frames. The plan allowed one frame per step.
- Pixel-exact colour: observations are by eye and phone camera.
- The S8 in any mode other than Controller Editor MIDI mode.

## Rerunning

From an interactive terminal with someone at the S8:

       mise run s8:screen-frame first
       mise run s8:screen-frame pattern rgb

The script checks for one S8 and one verified NIHardwareAgent, starts the
recorder, stops the agent with SIGKILL, prints each frame's size and hash,
sends it after Enter, asks what the panels show and for a PLAY press, then
relaunches the agent with `open -g`. Answering `stop` at any prompt skips to
recovery. Per-window counts go to `target/s8-captures/`.
