# Standalone macOS app

The app bundle packages the native status-bar app, the release bridge, its
default LED, jog mapping, and hardware tension configurations, and a Djay MIDI
mapping. It targets Apple Silicon (arm64) and macOS 12.0 or later.
Builds also retain the embedded screen font's OFL notice as
`Contents/Resources/S4ScreenSans-LICENSE.txt`.
Build it on an Apple Silicon Mac with Xcode
Command Line Tools, the Rust toolchain, and the locked Cargo dependencies
available:

```sh
bash macos/build-app.sh
```

The results are `dist/S4 MK3 Bridge.app`, the transfer archive
`dist/S4 MK3 Bridge-arm64.zip`, and the versioned release disk image
`dist/S4 MK3 Bridge-0.1.1-beta.3-arm64.dmg`. The script compiles
`macos/S4BridgeApp.swift` against the installed Apple macOS SDK, builds
`s4mk3-hid-midi-bridge` with `cargo build --locked --release`, copies
`examples/led-config.json`, `examples/jog-config.json`, and
`examples/tension-config.json` into the app resources as `default-led-config.json`,
`default-jog-config.json`, and `default-tension-config.json`, and creates
`S4 MK3 Bridge.djayMidiMapping` by running that same release bridge with
`--generate-mapping`. Cargo identifies this release as `0.1.1-beta.3`;
the bundle uses numeric version `0.1.1`, build `4`, and records the complete
release in `S4BridgeReleaseVersion`. The script checks alignment with Cargo.
It generates `AppIcon.icns` from `macos/Assets/AppIcon.png` and includes an
Applications shortcut in the compressed read-only DMG.
If the app output already exists, move it aside before building again.

The bridge and then the app bundle receive ad-hoc signatures for local transfer
and launch. This is not Developer ID signing or notarization. Gatekeeper may
require the recipient to approve opening the transferred app; distribution to
unfamiliar Macs requires a separately provisioned Developer ID signing and
notarization process.

No package manager or project-specific runtime dependency needs to be installed
on the target laptop. The bundled bridge's linked libraries are checked during
the build; the script stops if it finds an absolute dependency outside
`/System/Library` or `/usr/lib`, instead of silently shipping a machine-local
library reference.

## Use on the DJ laptop

Download the DMG from GitHub Releases, open it, and drag the app to the
Applications shortcut. Eject the disk image and launch the installed app.
The ZIP is an alternative: unzip it and move the app to Applications.
The app lives in the menu bar and does not require Rust, Cargo, or this repository.
Its first launch opens Settings; closing that window leaves the menu-bar app
running.

1. Connect the S4 and its power adapter. Use **Check controller** if needed.
2. Choose the full-controller profile and click **Start bridge**.
3. Use **Install Djay mapping…**, save the file, and accept Djay's import.
4. Enable **S4 MK3 MIDI Full** in Djay and select **S4 MK3 Bridge** as its mapping.
   Djay may append a number to the imported configuration name on repeated imports.
   If only a blank S4 MK3 configuration appears, restart Djay with the bridge
   already running. Do not run a separate CLI bridge at the same time.
5. Keep audio routed directly through the S4 in Djay.

### S4 screens

The October 22 handover build adds a **Screens** settings tab and shared
**Enable S4 screens** menu toggle. Use the normal full-profile **Start bridge**
action; **Save and restart** keeps your selected screen support and saved
LED/tension settings. Minimal MIDI,
HID probe, controller checks and mapping export remain screen-free.

HOTCUE/SAMPLES shows JUMP, active LOOP size, KEY and global RANGE.
STEMS/MUTE shows actual mute/solo feedback; Shift changes the upper row to ONLY.
All pages use the accepted smooth font, embedded in the bridge.

Stop any separate prototype bridge before starting this app. The existing
Info Screen mapping can stay selected. Otherwise use **Install Djay mapping...**
under a new name and select it in Djay; older mappings lack numeric feedback.
Preserve custom mappings. Quit Traktor/Mixxx before using the screens.
If NIHardwareAgent owns display interface 4, quit it temporarily using the
[ordered screen instructions](../docs/screens.md#macos-ownership-finding).
The app can stop NIHardwareAgent with confirmation or explicit automatic-stop
consent, and restore an agent it stopped. Screen handover force-closes only the
verified background driver: its normal shutdown blanks the S4 panels, while the
guarded forced stop preserves them. Active Traktor, Maschine or Komplete Kontrol
blocks the operation; the app never closes those applications.
Manual **Restart Native Instruments screen driver** works even with screen
support off and no previous app ownership. See the
[ordered handover and recovery guide](../docs/screen-driver-handover.md).
Screen failures turn the running status icon red while MIDI/LEDs/tension continue.
The app's Stop/Quit path also stops screen service.

This local rebuild keeps the existing version identifier; it is not a new
public release. The older GitHub release does not include these screens.
See the [app integration and package validation](../docs/app-screen-integration.md).
The new handover app/ZIP/DMG are built separately under
`dist/Screens NI Handover Fix-20261023/`, preserving the earlier apps and packages.
Build into a separate folder with `S4_BRIDGE_OUTPUT_DIR=/absolute/output/path`
when invoking `macos/build-app.sh`.

### Updated encoder layout

Use a build containing the encoder increment and reinstall its generated
mapping through **Install Djay mapping...**, saving under a new filename.
Keep custom mappings under their existing names, then select the new mapping
in Djay. Restarting an older bridge or selecting its old mapping isn't enough.
The historical app QA record doesn't establish that a local bundle contains
this increment.

This layout is full-profile only. Physical naming is corrected for all
rotations and only left presses. LOOP turn/press use native
`autoLoopDurationRotary` / `autoLoopOnOff`, independent of jump size.
Shifted LOOP turn uses `autoLoopMoveRotary`; shifted press uses `reloop`
to reactivate the stored loop range. Manual loop in/out is unmapped.
Reinstall this rebuilt bundle's mapping under a new filename to apply Reloop,
preserving existing custom mappings.

REV now uses native `reverseHold`: releasing it ends reverse, even after a
deck or Shift change. It does not automatically enable or restore Slip.
Normal Browse direction is corrected once in the mapping. Hold Preview to
start preview, turn BROWSER while held to scrub, and release to stop. No Shift
is required; otherwise BROWSER browses normally. Complete preview-toggle pulses
are sent on press and release, including shutdown. If preview was already
active or changes externally, close it using the mouse to recover.
Shift+BROWSER press still goes back, and LOOP is unchanged. Shift+CUE jumps
to the beginning of the song, leaving normal CUE unchanged. Playlist now
marks/unmarks selected songs and Star focuses the Queue (Note 10/11 swap).
Use a rebuilt bridge and install its newly generated mapping under a new name,
preserving existing custom mappings. The earlier Shift+BROWSER scrubber was
user-confirmed; this new held route and the new assignments need physical
acceptance.

Physical crossfader assignments and curve wait for raw Djay playback feedback
before their initial application, then follow physical changes only. This
supports both launch orders without repeatedly overriding later UI choices.
After mapping reselection or reconnect, use **Stop bridge** / **Start bridge**
to initialize a fresh session; automatic reconnect detection is unavailable.
The updated mapping's channel-7 playback outputs are required.

Rebuilding or replacing the app does not overwrite saved LED preferences in
Application Support. User-corrected palette values remain separate from
bundled defaults. Save or export any editor-only changes before quitting.

Normal MOVE jumps with or without an active loop using CC 2 on channels 1-4
(`skipRotary`). Press MOVE to toggle size-select mode for that logical deck:
the press sends no MIDI, there is no timeout, and unshifted rotation uses
CC 7 (`skipDurationRotary`) to change the native jump value immediately.
Press again to exit, not to save. Shift+MOVE always jumps one beat per
detent in either direction, even while selecting, preserving chosen jump
size: note 5 forward / note 14 backward on channels 9-12 only. Base-channel
aliases in the native catalog aren't runtime Shift destinations.

A/B/C/D retain independent native Djay loop/jump values and independent
local mode flags. The bridge doesn't count or reset shared sizes. Only the
selected deck selector pulses while selecting, with a 1.2-second
palette-brightness cycle and dark trough; hidden decks retain their modes
without flashing inactive buttons. Mode exit restores normal lighting.
**Stop bridge** followed by **Start bridge** initializes every mode flag off.
The full profile enables S4 screens when the Screens setting is on. Hardware jog tension is configured
separately below.

The user mouse-verified beat jump moving an active loop. Fresh MIDI/native
and physical LED checks are still pending. Follow the
[encoder live check](../docs/full-midi-test.md#encoder-layout-live-check)
for A/C/B/D independence, duplicate press/release, Shift while selecting,
active-loop movement, independent LOOP size, pulse restoration, and restart.

LED, run, jog mapping, and hardware tension preferences are stored under
`~/Library/Application Support/S4 MK3 Bridge/`. LEDs and Palette cover every
current LED JSON setting; import/export accepts a full or partial LED JSON object.
**Save and restart** applies edits to an active bridge. Invalid settings remain
visible and do not silently replace saved preferences.

### Mixer ON and Neural Mix EQ

With the updated full-profile bridge and its newly exported mapping, channel ON
buttons use dim/bright white for normal EQ or cyan for Neural Mix EQ. Bright
means that deck is the Instant FX target. An active Instant FX takes priority
using its button color, then restores the mode indication on release. Unknown
mode is dark until Djay sends feedback.

Press Filter to flash cyan awaiting a deck, then press its ON to toggle only
that deck's EQ mode without changing the FX target. Press Filter again to
cancel; outside selection ON retains its normal target-selection behavior.
Filter no longer resets the selected filter. Install the new mapping under a
new filename and preserve any custom mappings. Follow the
[live check](../docs/neural-mix-eq-check.md). MIDI-only capture verified initial
states and independent mouse-driven mode transitions for all four decks;
the user subsequently accepted the physical behavior and dim-cyan idle Filter
refinement. This is general acceptance, not completion of every edge-case
check in the runbook.

### Hardware jog tension

The **Jog** tab has a separate **Hardware** section. Its shared
**Jog Wheel Tension** slider (Assist / Neutral / Brake) configures both wheels, while
each wheel keeps independent movement and touch behavior. A first install
starts at **Neutral (0)**; later launches restore the saved value.
Channel 1's fader remains a normal mixer control, not a tension selector.

The integer range is -100 to 100: negative values select assistance (endpoint
7000), positive values select braking (endpoint 30000), and zero selects
neutral. Those endpoints are uncalibrated force ceilings, not torque units or
constant force. Assistance requires each wheel's own touch and movement;
braking can continue after release while that wheel is moving.

Use the existing **Save preferences** action before the next start, or
**Save and restart** while running. No mapping generation or reinstallation
is required for hardware tension. Only the full-controller runtime receives
`--tension-config PATH`; minimal, probe, controller checks, and mapping export
do not receive it.

The file is separate from jog mapping settings:
`~/Library/Application Support/S4 MK3 Bridge/tension-config.json`.
Its bundled default is `{"tension":0}`. A partial object (`{}`) keeps the
bundled default. Malformed JSON, noninteger values (including `1.0` and
booleans), out-of-range values, and unknown fields are rejected. Invalid
saved files are preserved until explicit reset and save; invalid editor
values remain in the editor without replacing saved preferences.

The integrated tension build passed 237 Rust and 17 Swift tests; the subsequent
Mixer ON/EQ and UI increments passed 248 Rust and 18 Swift tests, strict Clippy,
build and package integrity checks. On October 20, 2026, the user also confirmed
assistance/braking with Djay controls and LEDs, Stop/Quit clearing force,
saved-setting restore while stopped, and explicit restart after USB reconnect.
The [live integration check](../docs/jog-tension-integration.md) records that
focused procedure. The full UI regression matrix remains deferred at the
user's request; these passes do not establish a full visual QA pass.

The menu-bar symbol is green while the bridge is running and returns to the
normal system appearance in other states. Diagnostics shows the app's bundled
release version and build number.

The beta.3 [timeout recovery fix](../docs/tension-timeout-recovery.md)
clears force after a 50 ms input gap without closing the bridge. Diagnostics
logs `TENSION_SUSPENDED`; control/feedback processing continues while tension
history is reset. Fresh input establishes a new baseline and logs
`TENSION_INPUT_RECOVERED`; force returns only after each wheel qualifies new
movement with its usual touch rules. No Stop/Start is needed, and the saved
value is unchanged.
Older beta.2 packages still exit on this timeout. Physical recovery testing
remains pending.

### Jog mapping preferences

The generated mapping assigns **Shift+Sync** on either side to Djay's
tempo-range cycle. **This is global: it changes the tempo-fader range for all
decks, not just the selected deck.** Normal Sync remains deck-specific.
Install and select the new mapping to apply this binding; existing custom
mappings are preserved. The user confirmed this works on October 6, 2026.

The mapping sections of the **Jog** tab configure native Djay **Speed** and
**Reaction**, globally for decks A-D. Defaults match the user's calibration:

| Action | Speed | Reaction |
| --- | ---: | ---: |
| Scratch while touching | 2.7% | 150% |
| Pitch bend without touch | 2.7% | 17% |

Speed is the mapping's `rotarySensitivity`; Reaction is `rotaryAcceleration`.
Seek remains unchanged. Save persists these values separately in
`~/Library/Application Support/S4 MK3 Bridge/jog-config.json`.
An older installation without this file uses the bundled defaults.

Use **Install Djay mapping...** to generate a mapping from the current jog
editor values, save under a new name, and select that configuration in Djay.
The bundled generator handles export without opening HID or MIDI, even while
the bridge is running. Export replaces the chosen destination atomically
after the native save dialog's overwrite confirmation. Preserve existing
custom mappings, including **S4 MK3 Bridge 2**, under their existing names.
Saving or restarting the bridge alone does not apply mapping changes in Djay.

The user tested these values in Djay. This increment packages/configures those
mapping parameters; it does not fix backspin termination on touch release or
claim a fresh physical pass for every deck. Hardware tension is independent
of these mapping parameters.

**Stop bridge** and **Quit S4 MK3 Bridge** send the bridge its normal Ctrl-C
signal and wait for it to exit. Quitting the settings window does not stop it.
Diagnostics shows the actual launch arguments, exit status, and a bounded log.
Raw/debug logging is intended for troubleshooting rather than normal DJ use.
Continuous mode permits an idle controller and stops cleanly without requiring
input activity. A fixed-duration run retains the CLI probe's activity checks.

See [configurable-controls.md](../docs/configurable-controls.md) for the
distinction between existing preferences, useful future input-sensitivity
settings, and hardware/protocol constants that should remain fixed.
The verified build/UI evidence and remaining limits are recorded in
[macos-app-qa.md](../docs/macos-app-qa.md).

## Developer checks

The app's full-controller launch includes `--screens` when enabled, and a live
screen-control pipe. The standalone CLI
still requires an explicit `--midi full --screens`. See
[screen setup and physical checks](../docs/screens.md), including the
NIHardwareAgent interface-ownership conflict. Existing published mapping-only
builds do not contain this integration.

```sh
swift test --package-path macos
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
bash -n macos/build-app.sh
```
