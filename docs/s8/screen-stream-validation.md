# S8 screen phase 5: repeated frames, held claim and partial updates

Validation record for #43 ([screens.md](screens.md), phase 5). Every frame was
one bulk OUT transfer on interface 6, endpoint `0x04`, with a 500 ms timeout
and no retries, sent by `examples/s8-screen-frame/`. Streaming steps claimed
interface 6 once and held it for the step. Nothing was sent on interface 5.

The "Panels" columns hold what the tester typed at the S8. The expected
pictures are in [screen-stream-expected.md](screen-stream-expected.md).

## Answer

**Both panels take full frames back to back for a minute with no transfer
error. One full frame takes about 16.6 ms, so the S8 accepts about 60 full
frames per second in total: 60 per second on one panel, or 30 per second on
each of two.** The guessed SKIP and sub-rectangle formats both work.

| Question | Result |
| -------- | ------ |
| Held claim, one panel, 10 to 60 frames/s | No error on either panel; tester named 60 as clean on both |
| Both panels alternating | No error. 10, 20 and 30 frames/s per panel were delivered. Asked for 60 per panel, the tool delivered 30 |
| Transfer ceiling | 120 frames back to back in 2.0 s: 60 frames/s, 16.6 ms median per frame |
| 60 s at 30 frames/s per panel | 3,599 frames, no error, steady for the whole minute |
| SKIP command (`0x02`) | Works: the skipped top half kept the background |
| Sub-rectangle header | Works: a small pattern in the centre, background around it |
| Stream ends without a final frame | Both panels keep the last frame |
| Tool killed with SIGKILL mid-stream | Both panels showed the S8's startup screens, not a frame |
| Clean frame after the kill | Displays correctly on both panels |
| After `open -g` relaunch | Controller Editor labels again; Djay reacted to left PLAY |
| Side effects | No re-enumeration; HID input continued |

Photos were not added to the repository.

## Setup

macOS 27.3, Apple Silicon, one S8 in Controller Editor MIDI mode, Djay Pro
open with no track playing, release build of the tool. The passive recorder
ran on interface 5 HID and every `Kontrol S8` MIDI endpoint.
`mise run s8:screen-stream` ran twice on 2026-11-10:

| Run | UTC | Steps | Logs |
| --- | --- | ----- | ---- |
| 1 | 15:22–15:25 | a-left only; the tester answered `stop`, unsure which rate to name | `captures/screen-stream-20261110T152202Z/` |
| 2 | 15:30–15:38 | all steps, a to f | `captures/screen-stream-20261110T153024Z/` |

The numbers below are from run 2 unless stated. In the capture folders,
`<step>.txt` is the tool's output, `<step>.dump` lists each frame's hash (or
holds the frame's bytes for single frames), `notes.txt` holds the prompts and
the tester's answers, `snapshots.txt` the `ioreg` state after each step, and
`recorder.jsonl` the HID and MIDI events. The two `f-clean` frames are stored
as `.dump.gz`.

## Steps a to d: repeated full frames

Rates are frames per second per panel. "Late" counts frames that started more
than one frame interval after their slot. No transfer failed or timed out in
any stage.

| Step | Asked | Panel | Frames | Achieved | Late | Transfer ms (min / median / p95 / max) |
| ---- | ----- | ----- | ------ | -------- | ---- | --------------------------------------- |
| a-left | 10 | left | 50 | 10.17 | 0 | 16.15 / 16.24 / 16.33 / 16.57 |
| a-left | 20 | left | 100 | 20.12 | 0 | 16.12 / 16.23 / 16.32 / 16.36 |
| a-left | 30 | left | 150 | 30.10 | 0 | 16.10 / 16.24 / 16.30 / 16.73 |
| a-left | 60 | left | 300 | 59.99 | 0 | 16.20 / 16.60 / 16.70 / 16.75 |
| a-right | 10 | right | 50 | 10.16 | 0 | 16.12 / 16.19 / 16.28 / 16.30 |
| a-right | 20 | right | 100 | 20.13 | 0 | 16.13 / 16.19 / 16.26 / 16.30 |
| a-right | 30 | right | 150 | 30.08 | 0 | 16.10 / 16.19 / 16.28 / 16.39 |
| a-right | 60 | right | 300 | 59.99 | 0 | 16.23 / 16.58 / 16.70 / 16.79 |
| b-alternate | 10 | left | 50 | 10.06 | 0 | 16.15 / 16.23 / 16.39 / 16.55 |
| b-alternate | 10 | right | 50 | 10.06 | 0 | 16.11 / 16.21 / 16.31 / 16.37 |
| b-alternate | 20 | left | 100 | 20.03 | 0 | 16.13 / 16.23 / 16.30 / 16.36 |
| b-alternate | 20 | right | 100 | 20.03 | 0 | 16.12 / 16.23 / 16.30 / 16.37 |
| b-alternate | 30 | left | 150 | 30.00 | 0 | 16.19 / 16.53 / 16.65 / 16.72 |
| b-alternate | 30 | right | 150 | 30.00 | 0 | 16.21 / 16.51 / 16.65 / 16.76 |
| b-alternate | 60 | left | 300 | 29.99 | 598 of 600 | 16.22 / 16.60 / 16.71 / 16.83 |
| b-alternate | 60 | right | 300 | 29.99 | 598 of 600 | 16.38 / 16.60 / 16.71 / 16.76 |
| c-burst | unpaced | left | 120 | 60.01 | – | 16.11 / 16.62 / 16.67 / 16.71 |
| d-soak | 30 | left | 1,800 | 30.00 | 1,077 of 3,599 | 16.09 / 16.61 / 16.69 / 25.14 |
| d-soak | 30 | right | 1,799 | 29.98 | 1,077 of 3,599 | 16.07 / 16.60 / 16.69 / 17.23 |

What the tester reported:

| Step | Panels |
| ---- | ------ |
| a-left | Highest clean rate: 60 |
| a-right | Highest clean rate: 60 |
| b-alternate | Highest clean rate: 60 |
| c-burst | No tearing or stale areas on the left panel afterwards |
| d-soak | Steady for the whole 60 s |

- **The limit is the transfer time, not the panel.** A full frame takes
  16.1 to 16.8 ms however fast frames are asked for, which is one frame per
  1/60 s. One panel alone therefore reaches 60 frames per second. Two panels
  share that: the 60-per-panel stage of step b took 10 s for its 600 frames
  instead of 5 s, so each panel got 30 per second. The tester's `60` for
  step b is a statement that this stage looked clean, not that 60 frames per
  second reached each panel.
- **30 per panel on two panels is at the limit.** In the soak, 1,077 frames
  started late, the tool stopped at its 60 s cap one frame short of 3,600,
  and both panels still averaged 30.0 and 29.98 frames per second. One left
  transfer took 25.1 ms; every other one stayed under 17.3 ms.
- [INFERENCE] The constant 16.6 ms suggests the S8 takes one frame per
  refresh of a 60 Hz panel. Nothing in the run measured the panel's refresh.
- Run 1 measured the same for a-left: 10.17, 20.12, 30.09 and 59.98 frames
  per second, no late frames, medians 16.19 to 16.61 ms.

## Step e: partial updates

Each partial frame followed a fresh solid `F8 00` background (the 32-byte
REPEAT frame of phase 2), all on the left panel.

| Step | Bytes | SHA-256 | Transfer | Panels |
| ---- | ----- | ------- | -------- | ------ |
| e-skip | 130,592 | `698f240501d201db4ebda513adb455b2ee0c67f22b2a2d7ffdd06b611cdd0fcd` | OK, 8.2 ms | "top half red, bottom half has color bars" |
| e-rect | 25,628 | `1585abe9e5aad480f4905812bedecd242904dd3d3cc65715903cb0d68dfef476` | OK, 1.7 ms | "in the center and small" |

Both match the expected pictures, so both guessed formats are right:

- **SKIP.** Full-panel header, then `02 00 7F 80` (skip 32,640 pixel pairs,
  the top half), then `00 00 7F 80` (LITERAL for the bottom half) and its
  pixels. The skipped area kept what the panel showed before.
- **Sub-rectangle.** Header bytes 8–15 are x, y, width and height as
  big-endian 16-bit values: `00 A0 00 60 00 A0 00 50` for 160×80 at x 160,
  y 96, followed by `00 00 19 00` (LITERAL, 6,400 pixel pairs). Only that
  rectangle changed.

The tester described where the rectangle landed in words only, so its exact
position to the pixel is not checked.

## Step f: exit state

| Step | Tool | Panels |
| ---- | ---- | ------ |
| f-end | 5 s alternating at 30 frames/s, 150 frames per panel, no error, no late frames; no final frame sent | "Left S8 Screen with the yellow bar underneath the fourth grey bar from the left. Right screen is the same" |
| f-kill | 30 frames/s alternating, killed with SIGKILL after about 4 s | "Traktor logo on the left, Kontrol S8 on the right with the hold S & Q buttons during startup for Vegas Mode line" |
| f-clean | One full pattern frame to each panel, both OK in 16.1 ms | Both panels show the clean test pattern |

- **f-end** is frame 149, the last one sent: its bar spans x 108–132, under
  the fourth counter cell (x 105–125), and in frame 149 the eight leftmost
  cells are grey. In run 1 the left panel likewise kept its last frame, 299:
  the tester saw the bar under the eighth cell (bar x 212–236, cell x
  213–233).
- **f-kill did not leave a frame on the panels.** The expectation was a
  frame near number 120, possibly torn. Instead both panels showed the S8's
  own startup screens. The device did not re-enumerate: its registry id
  (`0x100007415`) and the interface ids are the same in every snapshot, and
  the recorder's HID handle stayed open. [INFERENCE] A transfer cut off
  inside a frame makes the S8 reset its display. The run cannot tell that
  apart from the kill landing between frames.
- The S8 took clean frames straight afterwards, so a kill mid-stream leaves
  the panels usable.
- The clean frames are the phase 4 frames byte for byte (SHA-256
  `bd568210…c138e4` left, `43f8d9ea…83148d` right).

## Release and recovery

| Moment | Panels |
| ------ | ------ |
| Interface released, agent still stopped | "both have the clean test pattern" |
| After `open -g` relaunch | "Controller Editor labels again" |
| Left PLAY after recovery | Djay reacted |

## Input and MIDI per window

Counts from the passive recorder, run 2. Each window holds one left PLAY
press and release.

| Window | HID report 1 | MIDI on `Traktor Kontrol S8 Input` |
| ------ | ------------ | ---------------------------------- |
| Baseline | 2 | PLAY ch 1 CC 10 on/off |
| Agent stopped | 2 | port gone |
| After a-right | 2 | port gone |
| After d-soak | 2 | port gone |
| Recover | 2 | PLAY ch 1 CC 10 on/off; Djay reacted |

The runner asked after a-right and after d-soak whether LEDs or audio changed
on the PLAY press. The tester left both answers empty, so this run records no
observation about LEDs or audio.

## Interface ownership

| Snapshot | Interface 6 | Device registry id |
| -------- | ----------- | ------------------ |
| Before, agent running | NIHardwareAgent (pid 89594) | `0x100007415` |
| Agent stopped, and 1 s after each step | no client | `0x100007415` |
| Recovered + 5 s | NIHardwareAgent (pid 94398) | `0x100007415` |

## What this changes

- The bridge's cap of 30 frames per second per panel
  ([screen-transport.md](screen-transport.md)) is the most two panels can
  take, and it held for a minute. It is no longer a guess.
- A frame that changes part of a panel can be sent as a SKIP or
  sub-rectangle frame and costs proportionally less transfer time (8.2 ms
  for half a panel, 1.7 ms for 160×80). The bridge does not use this yet.
- The bridge must stop its screen worker cleanly. A process killed while it
  streams leaves the startup screens on the panels until the next frame.

## Not covered

- Photos, and pixel-exact colour or position: observations are by eye.
- Whether 60 frames per second on one panel shows every frame; the tester
  judged steadiness, not individual frame numbers.
- Rates above 60 per second, input-to-panel latency, and runs longer than
  60 s.
- Why the panels show the startup screens after the kill, and whether a kill
  between two frames does the same.
- Partial frames in a stream, and SKIP or sub-rectangles on the right panel.
- Frames through the bridge's `--s8-screens-stdin` channel (#44).
- LED and audio behaviour during streaming.

## Rerunning

From an interactive terminal with someone at the S8:

       mise run s8:screen-stream

The script checks for one S8 and one verified NIHardwareAgent, starts the
recorder, stops the agent with SIGKILL, lists the expected pictures and asks
before each step, then relaunches the agent with `open -g`. Answering `stop`
at any prompt skips to recovery. The four stages of steps a and b always run
in the order 10, 20, 30, 60 frames per second and the terminal names each as
it starts; the prompt wants the highest stage that looked clean.
