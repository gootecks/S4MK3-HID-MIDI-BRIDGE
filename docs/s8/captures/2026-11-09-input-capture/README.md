---
summary: "Recovered logs of the agent-stopped input capture of 2026-11-09 (#10)."
type: capture
---

# S8 input capture, 2026-11-09 (recovered copy)

Fork-only evidence for [input-capture.md](../../input-capture.md). The original
logs of the #10 agent-stopped capture lived in `target/s8-captures/` and were
deleted when the build cache was garbage-collected. These files were written
out from the analysis session that still held the reports in memory. They are
committed so the evidence does not depend on a cache directory again.

Everything here was read from S8 interface 5 HID, shared and read-only.
Nothing was sent to the S8. Report hex includes the report ID as its first
byte.

| File                                            | Contents                                                                                                                                                                            |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `runs.jsonl.gz`                                 | 192 stamped runs with 25,914 reports in total. One JSON object per line: `label` (checklist step), `file` (original log name), `base` and `reports` (hex strings in arrival order). |
| `labels.json`                                   | The 191 checklist step labels in order.                                                                                                                                             |
| `rows.json`                                     | The 191 checklist rows: number, step, control, motion and expected field.                                                                                                           |
| `steps.json`                                    | Per step, the reports selected for analysis; where a step was stamped more than once, its last run.                                                                                 |
| `senc_steps.json`                               | The 24 screen-encoder sub-steps (clockwise, counterclockwise and so on) with their reports.                                                                                         |
| `raw-live-check-20261110T062133Z.log.gz`        | Bridge output of the attended `--device s8 --raw` check for #12, build lines removed.                                                                                               |
| `raw-live-check-browse-20261110T062923Z.log.gz` | Second attended run of that check, for the right BROWSE encoder.                                                                                                                    |

Not recovered: the two re-check logs of 2026-11-09 (screen encoder turns and
crossfader assign positions). Their totals are in
[input-capture.md](../../input-capture.md#screen-encoder-counts-per-turn).

`tests/fixtures/s8-input-reports.jsonl` is the reduced copy the decoder test
uses; it was derived from `steps.json` and `senc_steps.json`.
