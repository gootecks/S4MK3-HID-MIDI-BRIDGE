---
summary: "Usage and verification status of the read-only S8 input recorder used for the agent-stopped capture (#10)."
type: procedure
---

# S8 input recorder

The recorder behind the agent-stopped capture for #10. The ordered control list is in
[capture-checklist.md](capture-checklist.md).

## Recorder

`examples/s8-input-diff` opens only the S8 control HID interface (5) in macOS
shared mode and reads input reports. It sends nothing: no output or feature
reports, no control transfers, no bulk or screen traffic, and it never opens
interface 4 or 6. It does not stop or start `NIHardwareAgent`.

Each report is diffed against the previous report with the same ID. For every
changed field it prints one line: time, report ID, byte offset (with report ID,
as in the datasets), mask or byte range, `old->new`, and field names. For
rotary encoders it adds the wrapped delta (`+1 mod 16`).

```text
    1.000s r1 [9] mask 0x10 0->1 E:left_play | P:BtnPlayButtonLeft
    2.600s r1 [1] mask 0x0f 0->1 (+1 mod 16) P:EncLoopKnobLeft
    3.200s r2 [17..18] mask 0x0fff 2050->2624 (+574) E:left_fader_1 | P:FadFader1Left
    3.300s r2 [1..2] 0->65535 (-1 mod 65536) E:left_screen_encoder_1 | P:EndKnob1Left
    2.700s r1 [36] mask 0x80 0x00->0x80 ?
```

Names come from two sources:

- `P:` kontrol-s8-protocol `data/HID_INPUT.json`
  ([gusgustavodj/kontrol-s8-protocol](https://github.com/gusgustavodj/kontrol-s8-protocol),
  CC BY 4.0).
- `E:` Encdr `encdr/descriptors/ni_kontrol_s8.json`.

`?` means a changed bit that neither source describes. Fields that both
sources place at the same bytes print on one line; where they disagree (mask,
width or report) each prints its own line, which is how the discrepancy list
below gets resolved. The 4 heartbeat bytes at `r1 [26..27]` and `[32..33]`
(changing in every report per [input-capture.md](input-capture.md)) are
hidden unless `--show-heartbeat` is given.

Options:

| Option                  | Meaning                                                                                                                                        |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `--log <path>`          | Raw JSONL log. Default `/tmp/s8-input-<epoch>.jsonl`; never inside the repo.                                                                   |
| `--deadband <n>`        | Suppress analog changes of `n` counts or fewer from the last printed value (default 3). Touch bits, buttons and encoders are never suppressed. |
| `--step <label>`        | Label active from the start. Type further labels on stdin.                                                                                     |
| `--seconds <n>`         | Stop after `n` seconds (default: Ctrl-C).                                                                                                      |
| `--show-heartbeat`      | Print the heartbeat bytes too.                                                                                                                 |
| `--list`                | List S8 HID entries and exit; opens nothing.                                                                                                   |
| `--replay <log.jsonl>`  | Decode an earlier log offline; no device access.                                                                                               |
| `--protocol`, `--encdr` | Dataset paths (defaults: sibling checkouts `../kontrol-s8-protocol`, `../Encdr`, relative to the working directory).                           |

The JSONL log holds one record per line: `hid` (every report: `t`, `wall_ms`,
`step`, `id`, `len`, `hex`), `change` (the printed line) and `step` markers.

## Build and run

From the worktree root (the defaults assume the sibling checkouts):

```sh
cargo build --release --example s8-input-diff
./target/release/examples/s8-input-diff --log /tmp/s8-capture-$(date +%Y%m%d-%H%M).jsonl
```

Type a step label and press Enter **before** each control, for example `L-play`,
then perform the motion. Labels are stamped into the log and printed as
`=== step: <label> ===`. Use the step labels in the tables below so the log can
be grouped later. Stop with Ctrl-C. Offline check of any log:

```sh
./target/release/examples/s8-input-diff --replay /tmp/s8-capture-....jsonl --log /tmp/replay.jsonl
```

## Recorder verification status

- Opens the connected S8 (interface 5, shared) read-only without error; `--list`
  shows interface 5 (usage page `0xff01`).
- An idle run (20 s, agent running, no presses) received no reports, as in
  [agent-stop-validation.md](agent-stop-validation.md): the S8 reports only on
  change. Reports 1 (41 B) and 2 (109 B) were therefore not re-observed by this
  tool; they were observed by the earlier Swift recorder
  ([input-capture.md](input-capture.md)).
- Name resolution, nibble and 16-bit wrap deltas, deadband, heartbeat hiding
  and step markers were exercised with `--replay` on a synthetic log, not on
  hardware.
