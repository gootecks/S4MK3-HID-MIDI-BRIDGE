# Standalone macOS app

The app bundle packages the native status-bar app, the release bridge, its
default LED, jog mapping, and hardware tension configurations, and a Djay MIDI
mapping. It targets Apple Silicon (arm64) and macOS 12.0 or later.
Build it on an Apple Silicon Mac with Xcode
Command Line Tools, the Rust toolchain, and the locked Cargo dependencies
available:

```sh
bash macos/build-app.sh
```

The results are `dist/S4 MK3 Bridge.app`, the transfer archive
`dist/S4 MK3 Bridge-arm64.zip`, and the versioned release disk image
`dist/S4 MK3 Bridge-0.1.1-beta.1-arm64.dmg`. The script compiles
`macos/S4BridgeApp.swift` against the installed Apple macOS SDK, builds
`s4mk3-hid-midi-bridge` with `cargo build --locked --release`, copies
`examples/led-config.json`, `examples/jog-config.json`, and
`examples/tension-config.json` into the app resources as `default-led-config.json`,
`default-jog-config.json`, and `default-tension-config.json`, and creates
`S4 MK3 Bridge.djayMidiMapping` by running that same release bridge with
`--generate-mapping`. Cargo identifies this release as `0.1.1-beta.1`;
the bundle uses numeric version `0.1.1`, build `2`, and records the complete
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
Normal Browse direction is corrected once in the mapping. Preview remains
a playback toggle; Shift+BROWSER now seeks preview instead of navigating
sections. Each detent emits a native forward/backward preview button pulse,
without tracking playback state. Seeking while preview is stopped is ignored
by Djay; ordinary browsing cancels preview. Shift+BROWSER press still goes
back, and LOOP is unchanged. Use a rebuilt bridge and install its newly
generated mapping under a new name; old bridge binaries and mappings do not
provide this route. Physical seek direction and multi-detent response still
need a live check.

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
Screens remain excluded. Hardware jog tension is configured separately below.

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
physical Filter/ON gestures and lamp colors still need confirmation.

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

The integrated build passes 237 Rust and 17 Swift tests, strict Clippy, build
and package integrity checks. On October 20, 2026, the user also confirmed
assistance/braking with Djay controls and LEDs, Stop/Quit clearing force,
saved-setting restore while stopped, and explicit restart after USB reconnect.
The [live integration check](../docs/jog-tension-integration.md) records that
focused procedure. The full UI regression matrix remains deferred at the
user's request; these passes do not establish a full visual QA pass.

The menu-bar symbol is green while the bridge is running and returns to the
normal system appearance in other states. Diagnostics shows the app's bundled
release version and build number.

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

```sh
swift test --package-path macos
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
bash -n macos/build-app.sh
```
