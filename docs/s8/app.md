---
summary: "Kontrol S8 Bridge.app, the S8 menu-bar app: packaging decision, build, signing, transfer, first start and limits."
type: procedure
issues: [16, 17, 18, 19]
---

# Kontrol S8 Bridge app

`Kontrol S8 Bridge.app` runs the S8 with djay Pro from the menu bar. It does what the
[kit](kit.md) launcher does, without Terminal: checks, mappings, NIHardwareAgent stop,
`S8Host` with the bridge, and the agent's restore on every way out.

Starting a run stops NIHardwareAgent and sends screen frames and, with deck LEDs on, LED
reports to the S8. On this Mac both need the user's go-ahead (see _Approvals_ in
`AGENTS.md`).

## Packaging decision

The S8 gets its own bundle; the S4 MK3 app stays as upstream builds it
([#18](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/18)).

- One app for both would put S8 code into upstream's `macos/S4BridgeApp.swift`,
  `BridgeController` and `build-app.sh`, and make every merge from upstream harder. The S8
  side shares no settings with the S4 side: no jog, motor or S4 screens.
- The S8 app is fork-owned: `macos/S8App/`, `macos/S8AppTests/` and
  `docs/s8/app-build.sh`. It reuses `S8Host` and the same Rust bridge binary.
- Names: bundle `Kontrol S8 Bridge.app`, bundle ID `com.gootecks.kontrol-s8-bridge`,
  executable `S8App`. The app has its own version, `0.1.0` in `macos/S8App/Info.plist`.
  The build number is the commit count and `S8BridgeCommit` names the commit.

So the S4 parts of [#17](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/17)
(profile switch and hidden S4 settings in the S4 app) are not needed: each app runs one
profile.

## Contents

| Path in `Contents/`           | Source                                                     |
| ----------------------------- | ---------------------------------------------------------- |
| `MacOS/S8App`                 | `macos/S8App`, the menu-bar app.                           |
| `MacOS/S8Host`                | Release build of `macos/S8Host`.                           |
| `MacOS/s4mk3-hid-midi-bridge` | Release build of the Rust bridge.                          |
| `Resources/Mappings/`         | Default and Evan mappings under their newest frozen names. |
| `Resources/AppIcon.icns`      | `macos/Assets/AppIcon.png`.                                |

## Build

```sh
mise run s8:app
```

It sends nothing. Output: `dist/Kontrol S8 Bridge <commit>/Kontrol S8 Bridge.app` and
`dist/Kontrol S8 Bridge <commit>.zip`. `<commit>` ends in `-dirty` when tracked files have
uncommitted changes. The build stops when that output exists, or when a generated mapping
differs from its newest frozen revision in [mappings](mappings/README.md).

### Signing

`S8_APP_SIGN_IDENTITY` names the codesign identity. Unset, the script takes the first
`Developer ID Application` identity in the keychain, then `Apple Development`, then signs
ad hoc. It prints the identity it used.

- **Apple Development** (what this Mac has today): signed by the team, but Gatekeeper
  rejects it on another Mac. The other Mac needs the quarantine step below once.
- **Developer ID Application**: the script adds the hardened runtime and a timestamp. With
  `S8_APP_NOTARY_PROFILE` set to a `notarytool` keychain profile it notarizes the zip and
  staples the app. The app then opens on any Mac without extra steps.

A Developer ID certificate needs the account holder of the Apple Developer team. In Xcode,
open Settings > Accounts > Manage Certificates and add a Developer ID Application
certificate. Store a notary profile once:

```sh
xcrun notarytool store-credentials kontrol-s8 --apple-id <id> --team-id PHZSDK92NB
S8_APP_NOTARY_PROFILE=kontrol-s8 mise run s8:app
```

## Transfer and first start

The other Mac needs Apple Silicon and macOS 12 or later. It also needs djay Pro with PRO
access and the Native Instruments software for the S8: Controller Editor and
NIHardwareAgent.

1. Send the zip (AirDrop, a drive), unzip it and move the app to `/Applications`.
2. Without notarization: remove the quarantine mark once,
   `xattr -dr com.apple.quarantine "/Applications/Kontrol S8 Bridge.app"`. Opening the app
   and then choosing Open Anyway in System Settings > Privacy & Security works as well.
3. Open djay Pro and Controller Editor, connect the S8, then open the app. Its icon is in
   the menu bar; it has no Dock icon.
4. Choose Start. The first start asks for Accessibility access: allow Kontrol S8 Bridge in
   System Settings > Privacy & Security > Accessibility and choose Start again.
5. If the app installed a mapping while djay Pro was running, it asks to restart djay Pro
   first. In djay's MIDI settings choose `Kontrol S8 MIDI Full` and the mapping the menu
   names.

A rebuild signed with the same certificate keeps the Accessibility grant. An ad hoc build
loses it at every rebuild.

## What Start does

In this order. Up to step 4 nothing is stopped and nothing is sent.

1. Checks: no other bridge, exactly one S8, NIHardwareAgent installed, no Traktor, Maschine
   or Komplete Kontrol running, djay Pro running, Accessibility access.
2. Copies both mappings into `~/Music/djay/MIDI Mappings/` when they are missing. It never
   overwrites a file. After a copy with djay Pro open it asks before going on.
3. Asks to open Controller Editor when it is closed
   ([S8 dark at start](translator-validation.md#s8-dark-at-start)).
4. Stops NIHardwareAgent and notes that in its preferences.
5. Runs `S8Host` with the bridge and the menu settings. The menu shows Running once the
   bridge reports `MIDI_FULL_READY`.

Stop, Quit, logging out and `SIGTERM` stop `S8Host`, wait for it and start NIHardwareAgent
again. When the app itself is killed, `S8Host` and the bridge see their stdin close and
stop; the next app launch starts the agent again.

## Menu

| Item                           | Effect                                                      |
| ------------------------------ | ----------------------------------------------------------- |
| Start / Stop                   | Runs or stops the S8.                                       |
| Layout                         | Default or Evan; picks `--layout` and the mapping it names. |
| Deck LEDs                      | `--leds`; on by default.                                    |
| Panels                         | `--screens`; on by default.                                 |
| Right Panel                    | Library, or Deck 1-4 (`--deck N`).                          |
| Start When the App Opens       | Starts a run at launch; off by default.                     |
| Install Mappings in djay       | Copies missing mappings without a run.                      |
| Open Run Log / Show Log Folder | The run's output.                                           |

Settings change only while stopped. Each run writes
`~/Library/Logs/Kontrol S8 Bridge/s8-<UTC>.log`; the app keeps the newest 30.

## Verification

- `swift test --package-path macos --filter S8AppTests`: runner sequence, agent restore,
  mapping install and preferences, against fakes.
- On this Mac on 2026-11-20, launched from Terminal with djay Pro, Controller Editor and the
  S8: a run reached `MIDI_FULL_READY` with panels and deck LEDs on. `SIGTERM` stopped
  `S8Host` and the bridge and restarted NIHardwareAgent within 0.3 s. After `kill -9` of the
  app both children exited within a second, and the next launch restarted the agent.
- Used for a set on the DJ MacBook (2026-11-21), copied there as the zip. The user's
  report: "it worked!". Individual controls and fault paths were not checked one by one.

## Limits

The app has the limits of the [kit](kit.md#limits). Also:

- Not notarized until a Developer ID certificate exists.
- Display buttons 1 and 4 belong to Native Instruments' agent and do nothing during a run.
