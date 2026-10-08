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
| `live --out PATH --mode MODE --enable-motor --seconds N` | Explicitly enabled bounded motor trial. Default 10 seconds, maximum 30; Ctrl-C stops it. `trial` is an alias. |
| `live --out PATH --mode fader-brake --enable-motor --until-stopped` | Supervised continuous fader test, ending on Ctrl-C or an input/output fault. Conflicts with explicit `--seconds`; unavailable for fixed modes. |

Add `--analog` to `capture` to include fresh 79-byte numbered mixer snapshots
(report 2). The physical channel-1 volume fader is decoded from numbered bytes
3-4, little-endian, range 0-4095. Analog capture still sends no output reports;
historical captures without this optional report remain replayable.

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
| `--mode fader-brake` | Physical channel-1 volume fader selects opposing-motion force on the left wheel. Minimum must be observed before arming; no assistance or right-wheel option. |
| `--side left` / `right` | Physical wheel, not logical deck A/B/C/D; default left. Fader mode rejects `right`. The other wheel always receives zero magnitude. |
| `--gain N` | Fixed modes only: integer raw units per count/ms, default 1, range 1-100. Fader mode rejects even an explicit `--gain 1`. |
| `--max-force N` | Fixed modes: default 100, live 1-500, replay 1-1000. Fader mode: default 5000, live/replay 1-25000. `--force-cap` is an alias. |

For fixed modes, the magnitude is
`min(max_force, abs(raw_count_delta) * gain * 1000 / elapsed_host_microseconds)`,
rounded down. Host timing is used; hardware timer frequency is not assumed.
Fader mode instead selects `floor(raw_fader * max_force / 4095)`, independent of
wheel speed. Raw fader values 0-32 always select zero and arm the session.
These are **uncalibrated diagnostic units**, not percentages, physical torque,
factory tension equivalents, or established safe limits. Start below the default
cap in fixed trials, and start at fader minimum in the continuous test below.
The 25000 ceiling comes from pinned Mixxx source, not physical safety acceptance.

Fresh rest, first observations, and duplicate snapshots propose zero torque.
All non-baseline modes apply only to measured motion; lifting a hand does not terminate
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

The suite also includes a synthetic fader capture:

```sh
./s4mk3-jog-diagnostic replay --input jog-diagnostic-fader-replay.jsonl \
  --mode fader-brake --max-force 3666 > rehearsal-fader.jsonl
```

PASS: exit 0; high startup remains unarmed and neutral, then minimum arms it.
The left-wheel sequence is `0,0,0,-1833,-1833,-3666,3666,0,0,0,0`; every
right-wheel torque is zero. Reversal still brakes, touch release keeps tracking,
and lowering the fader, rest, and duplicate snapshots are neutral.
Adjacent mixer controls change independently without changing channel-1 force.

## Morning: compare braking with one continuous fader test

Use the **channel-1 volume fader, second from the left**, not the leftmost
fader or crossfader. The four channel faders are ordered `3 | 1 | 2 | 4`.
This is a physical control assignment, independent of the selected deck layer.
Only the physical left jog receives nonzero proposals.

1. Stop the bridge and quit Traktor/Mixxx. Connect and power one S4. Keep the
   current mapping and jog settings; this standalone test sends no MIDI.
   Perform the offline rehearsal above first.
2. Check the fader field without motor writes:

   ```sh
   ./s4mk3-jog-diagnostic capture --analog --seconds 30 \
     --out fader-check-$(date -u +%Y%m%dT%H%M%SZ).jsonl
   ```

   Move only channel 1 through minimum, midpoint, and maximum. Replay the saved
   filename with `--mode baseline`; inspect `motion.deck1_fader` near 0, 2048,
   and 4095. Move another fader separately: it must not change this field.
   Stop here if the field, framing, or range is wrong.
3. Lower channel 1 fully and start the supervised session:

   ```sh
   ./s4mk3-jog-diagnostic live \
     --out fader-left-$(date -u +%Y%m%dT%H%M%SZ).jsonl \
     --mode fader-brake --side left --enable-motor --until-stopped
   ```

   This first command uses a 5000-unit ceiling. An accidentally high startup
   fader stays neutral until a raw value at or below 32 is observed. Terminal
   `DIAGNOSTIC_FADER` messages show percentage, raw value, `armed`, and
   `selected_force` when level/arming changes. Confirm `armed=true` at minimum.
4. Gently turn the left wheel in both directions. While comparing the same
   gesture, gradually raise and lower the fader without restarting the process.
   Minimum selects zero, midpoint about 2500, and maximum 5000. The force
   opposes observed movement; at rest it is zero even with a high fader.
   Stop immediately if it self-starts, assists, reverses unexpectedly, or the
   right wheel responds. Physical feel is the acceptance check, not log output.
5. Return the fader to minimum and press Ctrl-C. Check the final `motor` record
   has an 11-byte accepted neutral write, the process exits, and physical drive
   stops. `selected_force` is the requested moving-wheel magnitude;
   `decision.torque` and actual `motor` records show what was proposed/attempted.
   If the 5000 ceiling is insufficient, record that result before a separate
   supervised run with an explicit higher `--max-force`, at most 25000.
   Each new session starts unarmed and requires minimum again.

On input timeout, malformed data, or output failure, the session terminates and
attempts neutral. Do not interpret cleanup attempts as delivery after disconnect.
The [pinned protocol comparison](jog-fader-braking-protocol.md) proves software
packet agreement, not resistance or shutdown acceptance on this controller.

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
complete numbered button/jog bytes and optional numbered analog bytes.
Fader live trials always request fresh analog report 2. `motion` contains counters,
sensor timers, touch, signed deltas, elapsed time, and the proposed torque/report.
Fader decisions also contain `raw`, `armed`, and `selected_force`. Live `motor`
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
[pinned Mixxx motion-dependent tension](https://github.com/mixxxdj/mixxx/blob/f12661e29b36cfdd261939674f1ab94c52f1b2d8/res/controllers/Traktor-Kontrol-S4-MK3.js),
and [local captured report lengths](traktor-ring-capture.md).
