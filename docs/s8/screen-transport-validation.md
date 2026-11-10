# S8 screen transport: attended check (#44)

The check of `--s8-screens-stdin` on the S8, as planned in
[screen-transport.md](screen-transport.md#attended-check). It ran on
2026-11-10 in three runs of `mise run s8:screen-transport`, with the user at
the S8, djay Pro and Controller Editor open and NIHardwareAgent stopped by
the runner. Each step ran
`s8-screen-feed | bridge --device s8 --midi full --s8-screens-stdin --until-stopped`
from a release build.

| Run | Capture | Steps |
| --- | --- | --- |
| 1 | [captures/screen-transport-20261110T162556Z](captures/screen-transport-20261110T162556Z/) | a, b, c (c not judged) |
| 2 | [captures/screen-transport-20261110T163759Z](captures/screen-transport-20261110T163759Z/) | c to i, started with the first-step argument |
| 3 | [captures/screen-transport-20261110T164659Z](captures/screen-transport-20261110T164659Z/) | i again |

Per step the capture holds the record hashes of the dry run (`.dry`), the
bridge output (`.txt`) and the feeder's stderr (`.feeder.txt`); `notes.txt`
holds the prompts and the user's answers.

## Results

"Status" and "Exit" come from the bridge output. "Reported" is the user's
answer to the runner's question, unchanged. No photos were taken.

| Step | Stream | Status lines | Exit, time | Question | Reported |
| --- | --- | --- | --- | --- | --- |
| a-left-10 | left, 10 fps, 50 frames, hold 5 s | pending, active, inactive | 0, 10 s | left panel: moving bar, counter ends at 49 and holds 5 s, then black; right panel unchanged | "y" |
| b-right-30 | right, 30 fps, 150 frames, hold 5 s | pending, active, inactive | 0, 10 s | right panel: steady bar, counter ends at 149 and holds 5 s, then black; left panel unchanged | "y" |
| c-alternate-10 | both, 10 fps each, 50 frames each, hold 5 s | pending, active, inactive | 0, 10 s | both panels move, both counters end at 49 and hold, then both black; no panel shows the other's title | run 1: "run it again" (only one panel was watched); run 2: "y" |
| d-alternate-30 | both, 30 fps each, 150 frames each, hold 5 s | pending, active, inactive | 0, 10 s | both panels steady, both counters end at 149, then both black; any tearing or flicker? | "y"; asked again after the run: "no tearing or flicker" |
| e-over-cap | left, 60 fps, 180 frames, hold 5 s | pending, active, inactive | 0, 8 s | left panel: bar moves in visible jumps but the counter ends at 179, then black | "y" |
| f-stall | left, 10 fps, 30 frames, hold 15 s | pending, active, inactive | 0, 18 s | left panel kept frame 29 unchanged and without flicker for the 15 s after the feeder stopped, then went black | "y" |
| g-eof | both, 10 fps each, 30 frames each, no hold | pending, active, inactive | 0, 3 s | both panels went black within about a second of the last frame and the bridge exited by itself | "y" |
| h-bad-record | left, 10 fps, 20 frames, then 12 bytes that are no record, hold 5 s | pending, active, `failed bad record magic [4e, 4f, 50, 45]` | 0, 7 s | left panel counted to 19, then went black at once; the bridge kept running until the 5 s hold ended | "y" |
| i-ctrl-c, run 2 | left, 10 fps, 600 frames (60 s), Ctrl-C asked after 5 s or more | pending, active, inactive | 0, 60 s | the bar was moving; after Ctrl-C the left panel went black and the bridge exited | "y" |
| i-ctrl-c, run 3 | the same | pending, active | 134, 12 s | not asked: the runner stopped on the status order | – |

After the steps, in runs 1 and 2:

- Bridge gone, agent still stopped: "black on both panels" (run 1), "all
  black" (run 2).
- After the agent was relaunched: "Controller Editor labels", and Djay
  reacted to left PLAY.

## What this establishes

- The bridge's worker shows frames from stdin on each panel alone and on
  both panels in one process, at 10 and at 30 frames per second per panel.
- Above the cap (60 fps offered to one panel) frames are dropped and the last
  frame still arrives.
- With stdin open and no new records the last frame stays on the panel.
- When stdin closes the panels that received a frame go black and the bridge
  exits with status 0.
- A bad record ends the stream with a `failed` status line that names the
  reason, the panel goes black, and the bridge keeps running.
- Every step that claimed interface 6 released it: the next step claimed it
  again, and NIHardwareAgent drew its labels after recovery.

## Failed: Ctrl-C while streaming

- **Run 2.** The bridge log runs from 16:40:16 to 16:41:16 and the feeder
  wrote all 600 records, so the bridge ended when stdin closed after 60 s,
  not on a Ctrl-C. The answer "y" does not match that, so the step was run
  again.
- **Run 3.** Ctrl-C about 12 s in. The bridge exited with status 134, which
  is an abort, and printed no `inactive` line. Its captured output ends
  after `S8_SCREEN_STATUS active`. The feeder wrote no summary line. The
  user, with the bridge gone and the agent still stopped: "left display is
  black, right display still shows Controller Editor labels". The right
  panel was not addressed in this step. Recovery was as in the other runs:
  Controller Editor labels on both panels, and Djay reacted to left PLAY.
- Cause. Ctrl-C in a terminal goes to every process of the step, so the
  feeder and the `tee` that recorded the bridge's output ended with it, and
  the bridge lost the reader of its stdout and stderr. The bridge logs to
  stdout. Its next status line panicked on the closed pipe, the closing log
  line panicked as well, and a third panic while the worker was dropped
  aborted the process. The exit frame for the left panel had gone out
  before that. Reproduced without the S8, with the worker on a fake
  transport and both streams closed: exit status 134. The chain through the
  real binary is inferred from that, not observed.
- Fix, not yet run on the S8. The bridge writes status and `S8_CONTROL`
  lines without panicking and stops cleanly when their reader is gone, log
  lines to a closed stdout are dropped, and a panic in the status callback
  can no longer skip the exit frames or the release. The runner starts
  `tee` with `-i` so that the log outlives Ctrl-C. Step i has to be run
  again: `mise run s8:screen-transport i-ctrl-c`.

## Not settled

- **d-alternate-30** is settled by the second answer above. The question
  ended with "any tearing or flicker?", so the first "y" could be read both
  ways; the runner's question is reworded.
- No photos, so the pictures in
  [screen-transport.md](screen-transport.md#steps) were compared by eye only.

## Not covered by this check

- Killing the bridge while it streams.
- The claim failure when NIHardwareAgent holds interface 6.
- The 500 ms transfer timeout under a stalled device.
- Two panels at the cap for longer than 5 s through the bridge (phase 5 ran
  60 s with the example tool,
  [screen-stream-validation.md](screen-stream-validation.md)).
