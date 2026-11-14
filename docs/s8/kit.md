---
summary: "The S8 kit: prebuilt bridge, S8Host, launcher and mapping for a second Apple Silicon Mac without the repository; build, transfer, run and limits."
type: procedure
issues: [131, 16]
---

# S8 kit

The kit runs the S8 with djay Pro on a Mac that has neither the repository nor a
toolchain. It is a folder, not an app. The menu-bar app does not run the S8 yet
([#16](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/16)).

The launcher stops NIHardwareAgent and sends screen frames to the S8. With deck LEDs on it
also sends LED reports. Both need the user's go-ahead on this Mac (see _Approvals_ in
`AGENTS.md`). On the other Mac the user starts it himself.

## Contents

| File                                   | Source                                                 |
| -------------------------------------- | ------------------------------------------------------ |
| `s4mk3-hid-midi-bridge`                | Release build of the Rust bridge.                      |
| `S8Host`                               | Release build of `macos/S8Host`.                       |
| `Start S8.command`                     | `docs/s8/kit-start.sh`.                                |
| `Kontrol S8 Bridge rN.djayMidiMapping` | The mapping the bridge generates.                      |
| `README.txt`                           | `docs/s8/kit-readme.txt` with commit and names filled. |

Both programs are signed ad hoc, not with a Developer ID, and are not notarized.

## Build

```sh
mise run s8:kit
```

It sends nothing. Output: `dist/Kontrol S8 Kit <commit>/` and the same folder as a zip.
`<commit>` ends in `-dirty` when tracked files have uncommitted changes.

The build stops when:

- a kit of that name exists. Move it away first.
- a program links a library outside `/System/Library` and `/usr/lib`.
- the generated mapping differs from the newest frozen revision in
  [mappings/](mappings/README.md). Check the new mapping on the S8 and freeze it first. The
  kit then never carries a mapping that was not checked.

## Transfer and first start

`README.txt` in the kit has the steps for the other Mac. In short:

1. Send the zip (AirDrop, a drive). Unzip it and move the folder out of Downloads.
2. Remove the quarantine mark, which would block the unsigned programs:
   `xattr -dr com.apple.quarantine <folder>`.
3. Open djay Pro and Controller Editor, connect the S8, and run `Start S8.command` from
   Terminal.

The other Mac needs Apple Silicon, macOS 12 or later, djay Pro and the Native Instruments
software for the S8.

## What the launcher does

In this order. Up to step 5 nothing is stopped and nothing is sent.

1. Checks: a terminal, Apple Silicon, both programs, NIHardwareAgent installed, exactly one
   S8, and no Traktor, Maschine or Komplete Kontrol running.
2. Copies the mapping into `~/Music/djay/MIDI Mappings/` when no file of that name is
   there. It never overwrites one. With djay Pro running it then stops: djay reads the
   folder at start.
3. `S8Host --check`: Accessibility access for Terminal, with the macOS prompt, and a running
   djay Pro.
4. Asks for Controller Editor when it is closed
   ([S8 dark at start](translator-validation.md#s8-dark-at-start)).
5. Asks two questions: deck LEDs (`--leds`), and which deck the right panel shows
   (`--deck N`).
6. Stops NIHardwareAgent and runs
   `S8Host --bridge ./s4mk3-hid-midi-bridge --screens [--leds] [--deck N]`. The output also
   goes to `logs/s8-<UTC>.log` in the kit folder.
7. On Ctrl-C, and on every other way out, it starts NIHardwareAgent again.

## Limits

- Deck LEDs and panels in one run are not yet checked on the S8. Each was checked alone
  ([led-feedback-validation.md](led-feedback-validation.md),
  [panel-views.md](panel-views.md)).
- The layout is the generated mapping's ([mapping.md](mapping.md)), not the Controller
  Editor template's.
- No VU meters and no mixer LEDs
  ([#123](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/123)).
- Display buttons 1 and 4 belong to Native Instruments' agent and do nothing while it is
  stopped.
- The kit was not yet run on a second Mac.
