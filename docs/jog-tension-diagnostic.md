# Wheel-tension diagnostic

October 8, 2026. This standalone diagnostic prepares **supervised** mechanical
tension experiments. It does not change the normal bridge, Djay mappings,
jog sensitivity, audio, LEDs, or saved preferences. Powered playback rotation,
cue haptics, and app integration remain outside its scope.

**Physical motor behavior is unverified.** Offline replay proves decoding,
direction selection, bounds, and software cleanup decisions, not physical
resistance, neutral torque, or a backspin fix. Do not run motor trials unattended.

## Get the standalone suite

From the repository root:

```sh
bash macos/build-jog-diagnostic.sh
```

This creates `dist/S4 Jog Diagnostic/` and
`dist/S4 Jog Diagnostic-arm64.zip`, without rebuilding or replacing the app.
Transfer and unzip the ZIP on the Apple Silicon DJ laptop; Rust/Cargo are not
needed there. The executable targets macOS 12 or later and is ad-hoc signed,
not Developer ID signed or notarized. Open Terminal in the extracted folder:

```sh
./s4mk3-jog-diagnostic --help
```

Developer equivalent: `cargo run --locked --bin s4mk3-jog-diagnostic -- ...`.
Normal `cargo run --locked -- ...` still selects the MIDI bridge.
The packager disables Cargo release stripping to avoid the recorded Rust
1.88/Apple SDK 27 proc-macro loading failure; it still builds optimized code.

## Commands and controls

| Command | Hardware effect |
| --- | --- |
| `capture --out PATH --seconds N` | Current button/jog snapshots only. **No output reports**, including neutral; preserves the wheel's current state. Default 30 seconds, maximum 120. |
| `replay --input PATH --mode MODE` | Offline raw decoding and proposed reports on stdout. Never initializes HID or sends reports. |
| `live --out PATH --mode MODE --enable-motor --seconds N` | Explicitly enabled motor trial. Default 10 seconds, maximum 30; Ctrl-C stops it. `trial` is an alias. |

Every capture/trial creates a **new** file; existing captures are never
overwritten. Live commands refuse known running bridge, diagnostic, Traktor,
or Mixxx processes before opening HID. An idle menu-bar app is allowed.
Stop its bridge first. Process inspection failure also refuses access.
This is not race-free ownership locking; do not start another controller
program during a trial. Connect only one powered S4.

Replay/live options:

| Option | Meaning |
| --- | --- |
| `--mode baseline` | Zero magnitude on both wheels, including startup/exit. This is an explicit neutral experiment, unlike no-write capture. |
| `--mode brake` | Torque opposite the selected wheel's observed movement. |
| `--mode assist` | Torque in the selected wheel's observed direction, including after touch release. Never a target RPM. |
| `--side left` / `right` | Physical wheel, not logical deck A/B/C/D; default left. The other wheel always receives zero magnitude. |
| `--gain N` | Integer raw magnitude units per count/ms, default 1, range 1-100. |
| `--max-force N` | Raw magnitude ceiling, default 100. Live accepts 1-500; offline replay accepts 1-1000. `--force-cap` is an alias. |

The magnitude is
`min(max_force, abs(raw_count_delta) * gain * 1000 / elapsed_host_microseconds)`,
rounded down. Host timing is used; hardware timer frequency is not assumed.
These are **uncalibrated diagnostic units**, not percentages, physical torque,
factory tension equivalents, or established safe limits. Start below the default
cap in the sequence below; a small raw cap can still have an unknown effect.

Fresh rest, first observations, and duplicate snapshots propose zero torque.
Brake/assist apply only to measured motion; lifting a hand does not terminate
the diagnostic's physical motion tracking. This changes no Djay audio routing.

## Offline rehearsal, no controller required

The included `jog-diagnostic-replay.jsonl` is synthetic software test data,
not a Traktor or bridge hardware capture.

```sh
./s4mk3-jog-diagnostic replay --input jog-diagnostic-replay.jsonl \
  --mode brake --side left --gain 1 --max-force 10 > rehearsal-brake.jsonl
./s4mk3-jog-diagnostic replay --input jog-diagnostic-replay.jsonl \
  --mode assist --side right --gain 1 --max-force 10 > rehearsal-assist.jsonl
```

Both must exit successfully. Brake's three movement torques are
`[-5,0]`, `[-10,0]`, `[10,0]`; right assistance gives
`[0,-5]`, `[0,-10]`, `[0,10]`. Rest and the duplicate are `[0,0]`.
The final summary is 60/-60 net counts and 50/-50 released counts.
Final `neutral_report` is a proposal, not evidence that hardware received it.

## Supervised physical sequence

Keep the current mapping and jog Speed/Reaction settings unchanged. Finish each
trial and inspect its evidence before proceeding. Use a new filename on repeats.

1. **No-write baseline.** Stop the bridge and quit Traktor/Mixxx. From rest,
   slowly turn each wheel one revolution in both directions, then make a gentle
   backward flick and release it. Use separate captures for each movement:

   ```sh
   ./s4mk3-jog-diagnostic capture --out baseline-left.jsonl --seconds 30
   ./s4mk3-jog-diagnostic replay --input baseline-left.jsonl \
     --mode baseline > baseline-left-analysis.jsonl
   ```

   PASS: capture contains valid 23/60-byte snapshots; counters follow both
   directions and continue after release; rest produces zero deltas. Record
   physical turn counts, stop markers, and wheel feel alongside the capture.
   Repeat for the right wheel. Do not assume 2880 counts/revolution without
   comparing a deliberately measured turn. A replay timing/filter failure is
   useful evidence; keep the raw file rather than relaxing limits blindly.

2. **Explicit neutral baseline.** Keep both wheels at rest at startup:

   ```sh
   ./s4mk3-jog-diagnostic live --out neutral-left.jsonl \
     --mode baseline --side left --enable-motor --seconds 10
   ```

   Gently repeat the passive movements. PASS: startup/final motor records show
   successful 11-byte zero-magnitude writes, and neither wheel starts itself.
   Record whether feel/free-spin distance differs from the no-write baseline.
   A successful write is only transport evidence: physically check both wheels.

3. **Low-cap braking, one wheel.**

   ```sh
   ./s4mk3-jog-diagnostic live --out brake-left.jsonl \
     --mode brake --side left --gain 1 --max-force 20 \
     --enable-motor --seconds 10
   ```

   Begin with gentle turns in both directions, not a hard flick.
   PASS: selected-wheel resistance increases rather than assists; the other
   wheel has no commanded torque; fresh rest is neutral. If direction/feel is
   unexpected, Ctrl-C and stop the sequence. Only repeat with higher caps after
   the preceding result is understood; never infer safety from a raw number.

4. **Low-cap assistance.**

   ```sh
   ./s4mk3-jog-diagnostic live --out assist-left.jsonl \
     --mode assist --side left --gain 1 --max-force 20 \
     --enable-motor --seconds 10
   ```

   PASS: motion feels looser, released movement remains tracked, and assistance
   ends at actual rest without self-start or reversal. Press Ctrl-C while moving
   for a separate Stop trial. Repeat brake/assist with `--side right`, using
   right-named files. Do not increase gain to hide missing or delayed snapshots.

5. **Lifecycle and reference comparison.** Confirm duration expiry and Ctrl-C
   both end the process with a successful final-neutral record and no lingering
   physical drive. After a supervised disconnect, expect a nonzero exit and
   failed neutral delivery if the device is gone. Reconnect and explicitly
   establish neutral again before further trials; do not assume firmware reset.
   Run Traktor separately, record its exact wheel-tension setting, and compare
   turns/free-spin duration, not only equal-force gestures. PASS: documented
   mechanical comparison with normal bridge audio/control behavior still intact
   after restarting it. Djay scratch continuation remains a separate open issue.

## Evidence and stop boundaries

Version-1 JSONL starts with `session`. `raw` contains host microseconds and
complete numbered button/jog bytes. `motion` contains counters, sensor timers,
touch, signed deltas, elapsed time, and the proposed torque/report. Live `motor`
records contain each attempted 11-byte report, accepted byte count, and any
transport error; inspect actual `motor` results rather than only proposals.
`end.summary` counts net and released movement. No-write capture ends with
`capture_end`; it does not claim computed totals or motor cleanup.

The decoder rejects out-of-order host/sensor time, changed counters at unchanged
sensor time, more than 512 counts/sample, and input gaps over 50 ms. Sensor
rollover is accepted only near the 32-bit boundary. Identical snapshots do not
refresh sensor freshness. These filters are deliberately conservative and must
be checked against actual captures.

Live input uses the bridge's current snapshot transport, not its slow interrupt
copies. A reader thread and a separate motor-writing handle let the writer react
to Stop, duration, reader errors, queue overflow, and a 50 ms input timeout.
Malformed/stale input, short writes, or failed writes terminate the trial and
attempt neutral; a Drop guard makes a further best-effort attempt on failure.
Reader cancellation is bounded rather than waiting indefinitely.

**This is not a hardware failsafe.** A native output call or disk write can
block, a forced process kill bypasses cleanup, and a disconnected device cannot
receive neutral. The diagnostic cannot guarantee torque removal in those cases.
Physical neutral and shutdown acceptance remain required before app integration.

Protocol references:
[pinned Encdr motor layout](https://github.com/RufusIbiza/Encdr/blob/5d7a689ca5789161b3efd1b2049d7e54504fe881/docs/hardware/ni_kontrol_s4_mk3.md),
[Mixxx motion-dependent tension](https://github.com/mixxxdj/mixxx/blob/main/res/controllers/Traktor-Kontrol-S4-MK3.js),
and [local captured report lengths](traktor-ring-capture.md).
