# Wheel-tension diagnostic

October 8, 2026. This standalone diagnostic prepares **supervised** mechanical
tension experiments. It does not change the normal bridge, Djay mappings,
jog sensitivity, audio, LEDs, or saved preferences. Powered playback rotation,
cue haptics, and app integration remain outside its scope.

**Physical motor behavior is unverified.** Offline replay proves decoding,
direction selection, bounds, and software cleanup decisions, not physical
resistance, neutral torque, or a backspin fix. Do not run motor trials unattended.

## Start here: bipolar fader-tension diagnostic, in order

This update is offline preparation, not physical acceptance. Step 2 is offline;
steps 3-9 are instructions for a later supervised controller session.
The lower half selects held-touch assistance, center selects zero, and the
upper half selects braking. This is not a clone of Traktor's tension curve.

Follow steps 1-9 in this section. Each step gives the command, physical actions,
and result to check before continuing. The references and older fixed-mode
experiments below are optional, not additional steps in this walkthrough.
Use a fresh run even if you tried earlier commands; previous captures stay intact.

Channel faders, left to right: **3 | 1 | 2 | 4**.
**Channel 1 is the second fader from the left.** The comparison control is
**channel 2, the third fader from the left**. Neither is the crossfader.
Only the physical left jog receives nonzero proposals in this test.

### 1. Open the suite folder and prepare filenames

Unzip `S4 Jog Diagnostic-arm64.zip`. Open Terminal. Type `cd ` (including the
trailing space), drag the extracted **S4 Jog Diagnostic** folder from Finder
into Terminal, and press Return. If Terminal is already in that folder, stay
there. Rust, Cargo, Python, and jq are not needed.

Run this block once:

```sh
ls ./s4mk3-jog-diagnostic ./jog-diagnostic-tension-replay.jsonl
run_id=$(date -u +%Y%m%dT%H%M%SZ)
rehearsal="rehearsal-tension-$run_id.jsonl"
capture="fader-check-$run_id.jsonl"
analysis="fader-check-$run_id-analysis.jsonl"
trial="tension-left-$run_id.jsonl"
```

**Continue when:** both files are listed without an error. Keep this same
Terminal window and folder for every remaining command; the filename variables
above will be reused. If a file is missing, stop and open the correct folder.

### 2. Run the offline bipolar rehearsal

This reads the supplied synthetic fixture. It does not access the S4 or send
motor commands. Do not move any controls for this step. You do not need the
older brake/assist rehearsals below.

```sh
./s4mk3-jog-diagnostic replay --input jog-diagnostic-tension-replay.jsonl \
  --mode fader-tension --max-force 30000 --assist-force 8000 > "$rehearsal"
rehearsal_status=$?
printf 'Rehearsal exit status: %s\n' "$rehearsal_status"
awk '
/"kind":"motion"/ {
  count++
  if ($0 !~ /"torque":\[-?[0-9]+,0\]/) bad=1
  if (/"zone":"neutral"/) {
    center++
    if ($0 !~ /"selected_force":0/ || $0 !~ /"torque":\[0,0\]/) bad=1
  }
  if (/"zone":"assist"/ && /"raw":0,/ && /"selected_force":8000/) low++
  if (/"zone":"brake"/ && /"raw":4095,/ && /"selected_force":30000/) high++
  if (/"zone":"assist"/ && /"touch":\[false,/) {
    released++
    if ($0 !~ /"torque":\[0,0\]/) bad=1
  }
  if (/"armed":false/ && $0 !~ /"torque":\[0,0\]/) bad=1
}
/"kind":"end"/ { ended++ }
END {
  ok=(count && center && low && high && released && ended && !bad)
  print ok ? "PASS: zones, endpoints, zero gates, and end" : "FAIL: rehearsal checks"
  exit !ok
}' "$rehearsal"
checks_status=$?
printf 'Rehearsal checks exit status: %s\n' "$checks_status"
```

**Continue when:** both exit statuses are `0` and the check prints `PASS`.
This checks selected zones, endpoint magnitudes, center/release/unarmed zeros,
right-wheel zero, and a completed replay, not a claimed full torque trace.
If either command fails or prints `FAIL`, stop here and keep the rehearsal.
No physical feel has been accepted by this check.

### 3. Prepare the controller

Choose **Stop bridge** in the menu-bar app, and quit Traktor and Mixxx.
Connect and power one S4. Keep the current Djay mapping and jog settings.
Leave the bridge stopped throughout steps 4-8. This standalone test sends no MIDI.

### 4. Capture both fader movements without motor writes

Read the actions first. Immediately after starting the command below, during
its single 30-second capture:

- Move **channel 1, second from the left**, fully down and hold briefly.
- Move it to the midpoint and hold briefly, then fully up and hold briefly.
- Return channel 1 to **CENTER** and leave it there.
- With channel 1 still at **CENTER**, move **channel 2, third from the left**, up and down.
- Leave both controls alone and let capture finish. Both movement checks must
  happen before the 30 seconds end; do not replay between them.

Start that capture now:

```sh
./s4mk3-jog-diagnostic capture --analog --seconds 30 --out "$capture"
capture_status=$?
printf 'Capture exit status: %s\n' "$capture_status"
printf 'Saved capture: %s\n' "$capture"
```

**Continue when:** capture ends on its own, the prompt returns, and the exit
status is `0`. This command sends no output reports, including neutral.
If it fails or you missed the movements, stop and start a fresh walkthrough
with a new run ID; do not reuse an existing capture filename.

### 5. Replay your completed capture and check channel 1

No controls need to move now. Replay reads the saved file without accessing
HID or sending motor commands. Run:

```sh
./s4mk3-jog-diagnostic replay --input "$capture" \
  --mode baseline > "$analysis"
replay_status=$?
printf 'Capture replay exit status: %s\n' "$replay_status"
grep -o '"deck1_fader":[0-9]*' "$analysis" | uniq
```

**Continue when:** the exit status is `0` and the displayed channel-1 values
follow the movement you recorded: near `0`, around `2048`, near `4095`,
then back to center. During the final channel-2 movement they must stay near
channel 1's center. A little variation at a held position is normal ADC noise.
The center arming band is **raw 2016 through 2080 inclusive**. Confirm the
returned center readings fall in that band before continuing.

The command lists changes in order; it may print many intermediate values.
If there are no values, a framing/range/timing error, or channel 2 substantially
changes this field, stop here. Preserve the capture and analysis for diagnosis.

### 6. Start the continuous motor test at CENTER

Put **channel 1, second from the left, at CENTER**, not minimum, and leave
both wheels at rest. The command explicitly uses the new mode's default
30000 braking and 8000 assistance ceilings. It has no duration limit:
`--until-stopped` runs until Ctrl-C or a fault. Omitting that flag would give
a finite 10-second trial; don't add `--seconds` to this continuous command.
Run:

```sh
./s4mk3-jog-diagnostic live --out "$trial" \
  --mode fader-tension --side left --max-force 30000 --assist-force 8000 \
  --enable-motor --until-stopped
```

**This is the first command in the walkthrough that enables motor output.**
It keeps running; the Terminal prompt will not return until you stop it or a
fault ends it. Do not enter another command while it runs.

Look for a `DIAGNOSTIC_FADER` message with `armed=true` and
`selected_force=0` and the neutral zone at center. No startup position outside
raw 2016-2080 can arm this mode. If necessary, adjust channel 1 slightly around
center while both wheels remain at rest. If it won't arm, press Ctrl-C and go
to step 8 to inspect the stop; don't lower it to minimum or raise the ceiling.

### 7. Compare the three zones while the same process runs

With `armed=true`, perform these actions in order without restarting:

- At **CENTER**, gently turn the left wheel in both directions. Selection and
  torque must be zero throughout raw 2016-2080.
- Keep touching and gently turning the left wheel while gradually lowering
  **channel 1**. Below center, selection rises toward 8000 at raw 0. Assistance
  follows measured motion only while touch is held. Release the wheel while
  it is moving: assistance must become zero, even if the wheel keeps coasting.
- Return channel 1 to **CENTER** before the next comparison.
- Gradually raise channel 1 while gently turning the left wheel in both
  directions. Above center, braking rises toward 30000 at raw 4095.
  Release while moving: braking remains eligible after release, until rest.
- Return channel 1 to **CENTER**. Let both wheels come to rest.

Terminal messages identify raw fader value, zone, arming, and selected force.
Selection isn't actual torque: unarmed, rest, duplicate snapshots, and the
right wheel always propose zero. These are raw diagnostic units, not factory
tension settings or calibrated physical torque.

**Expected physical result, still unverified:** assistance feels looser only
while touching and moving below center; resistance increases above center in
both directions. Held-touch assistance is deliberately more restricted than
native free rotation. Record the actual result, including no noticeable change.
If it self-starts, assists in the brake zone, continues driving in the assist
zone after release, reverses unexpectedly, or affects the right wheel, press
Ctrl-C immediately and proceed only to step 8.

### 8. Stop and inspect the actual final write

For a normal stop, first return channel 1 to **CENTER**, then press **Ctrl-C** in the
running Terminal window. If behavior is unexpected, press Ctrl-C immediately.
Once the prompt returns, run this block before any other command:

```sh
trial_status=$?
printf 'Live exit status: %s\n' "$trial_status"
grep '"kind":"motor","reason":"final_neutral"' "$trial"
grep '"kind":"end"' "$trial"
```

**Pass:** exit status `0`; the final motor line contains all three:

- `"report":[49,1,32,1,0,0,1,32,1,0,0]`
- `"written":11`
- `"error":null`

The end line should say `"reason":"ctrl_c"`. Also confirm the process has
stopped and physical drive has stopped. An accepted write does not establish
that physical drive stopped; check both wheels yourself.

If the command failed, either line is missing, neutral delivery failed, or
physical drive remains, stop testing and preserve the trial log. Do not increase
the ceiling. A disconnected device cannot receive neutral.

### 9. Save the results and finish

Run this to show the files belonging to this run:

```sh
printf 'Rehearsal: %s\nCapture: %s\nAnalysis: %s\nMotor trial: %s\n' \
  "$rehearsal" "$capture" "$analysis" "$trial"
```

Keep those files and note the feel below center, at center, and above center,
the held-touch/release difference, both directions, and whether Ctrl-C removed
physical drive.
The primary walkthrough is finished. You may restart the normal bridge after
the diagnostic has exited and the physical stop check passed.

If the default ceilings produced no useful difference but every check passed, record
that result first. A higher-ceiling repeat is a separate supervised experiment,
not an automatic next step; see the option limits below. A user-chosen braking
cap up to 60000 is allowed only for `fader-tension`, not an automatic jump to
60000. Each new session must observe CENTER again. To disable assistance in
a separately chosen repeat, use `--assist-force 0`; the lower zone then stays
zero. Use fresh filenames and repeat the arming and stop checks. Don't run the
older assist sequence as part of this test.

On a fault, the diagnostic terminates and attempts neutral. The
[pinned protocol comparison](jog-fader-braking-protocol.md) establishes software
packet agreement, not resistance or physical shutdown acceptance.

## Command reference (optional)

| Command | Hardware effect |
| --- | --- |
| `capture --out PATH --seconds N` | Current button/jog snapshots only. **No output reports**, including neutral; preserves the wheel's current state. Default 30 seconds, maximum 120. |
| `replay --input PATH --mode MODE` | Offline raw decoding and proposed reports on stdout. Never initializes HID or sends reports. |
| `live --out PATH --mode MODE --enable-motor --seconds N` | Explicitly enabled bounded motor trial. Default 10 seconds, maximum 30; Ctrl-C stops it. `trial` is an alias. |
| `live --out PATH --mode fader-tension --enable-motor --until-stopped` | Supervised continuous fader test, ending on Ctrl-C or an input/output fault. Also available for legacy `fader-brake`. Conflicts with explicit `--seconds`; unavailable for fixed modes. |

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
| `--mode fader-tension` | Physical channel-1 fader selects left-wheel held-touch assistance below center, zero at center, braking above center. Raw 2016-2080 inclusive must be observed to arm. |
| `--mode fader-brake` | Optional legacy mode, unchanged. Channel 1 selects left-wheel braking only. Raw 0-32 must be observed to arm; no assistance. |
| `--side left` / `right` | Physical wheel, not logical deck A/B/C/D; default left. Both fader modes reject `right`. The other wheel always receives zero magnitude. |
| `--gain N` | Fixed modes only: integer raw units per count/ms, default 1, range 1-100. Both fader modes reject even an explicit `--gain 1`. |
| `--max-force N` | Fixed modes: default 100, live 1-500, replay 1-1000. Legacy `fader-brake`: default 5000, live/replay 1-25000. New `fader-tension`: default 30000, live/replay 1-60000. `--force-cap` is an alias. |
| `--assist-force N` | Only `fader-tension`: integer 0-8000, default 8000; 0 disables assistance. All other modes reject this option. |

For fixed modes, the magnitude is
`min(max_force, abs(raw_count_delta) * gain * 1000 / elapsed_host_microseconds)`,
rounded down. Host timing is used; hardware timer frequency is not assumed.
Legacy `fader-brake` instead selects `floor(raw_fader * max_force / 4095)`,
independent of wheel speed. Raw values 0-32 select zero and arm that mode.
New `fader-tension` selects `floor((2016 - raw) * assist_force / 2016)` below
2016, zero at 2016-2080 inclusive, and
`floor((raw - 2080) * max_force / 2015)` above 2080. Only center arms it.
These are **uncalibrated diagnostic units**, not percentages, physical torque,
factory tension equivalents, or established safe limits. Start below the default
cap in fixed trials. Start the bipolar walkthrough at CENTER; minimum is only
the legacy braking mode's arming position. Legacy's 25000 ceiling comes from
pinned Mixxx source, not physical safety acceptance. New-mode caps are separate
experimental bounds informed by native captures, not accepted physical limits.

Fresh rest, first observations, and duplicate snapshots propose zero torque.
All non-baseline modes apply only to measured motion. Lifting a hand doesn't
terminate physical motion tracking, but does gate new-mode assistance to zero.
Braking can continue after release while moving. This changes no Djay audio routing.

## Older fixed-mode rehearsals (optional)

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

## Older fixed-mode physical sequence (optional)

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
Fader decisions also contain `raw`, `armed`, and `selected_force`; new-mode
decisions include `zone` (`assist`, `neutral`, or `brake`). Live `motor`
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

## Developer packaging reference

The DJ laptop uses the ZIP and walkthrough above; no development tools are
needed there. To build the standalone suite from the repository root:

```sh
bash macos/build-jog-diagnostic.sh
```

This creates `dist/S4 Jog Diagnostic/` and
`dist/S4 Jog Diagnostic-arm64.zip`, without rebuilding or replacing the app.
Preserve existing outputs before rebuilding. The executable targets Apple
Silicon and macOS 12 or later and is ad-hoc signed, not Developer ID signed
or notarized.

Developer equivalent: `cargo run --locked --bin s4mk3-jog-diagnostic -- ...`.
Normal `cargo run --locked -- ...` still selects the MIDI bridge.
The packager disables Cargo release stripping to avoid the recorded Rust
1.88/Apple SDK 27 proc-macro loading failure; it still builds optimized code.
