---
summary: "The prebuilt S8 bridge, S8Host, launcher and two mappings: build, transfer, run and limits."
type: procedure
issues: [131, 16, 65]
---

# S8 kit

The kit runs the S8 with djay Pro on a Mac that has neither the repository nor a
toolchain. It is a folder, not an app. The menu-bar app does not run the S8 yet
([#16](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/16)).

The launcher stops NIHardwareAgent and sends screen frames to the S8. With deck LEDs on it
also sends LED reports. Both need the user's go-ahead on this Mac (see _Approvals_ in
`AGENTS.md`). On the other Mac the user starts it himself.

## Contents

| File                                     | Source                                                  |
| ---------------------------------------- | ------------------------------------------------------- |
| `s4mk3-hid-midi-bridge`                  | Release build of the Rust bridge.                       |
| `S8Host`                                 | Release build of `macos/S8Host`.                        |
| `Start S8.command`                       | `docs/s8/kit-start.sh`.                                 |
| `Kontrol S8 Bridge rN.djayMidiMapping`   | Checked default mapping.                                |
| `Kontrol S8 Bridge Evan.djayMidiMapping` | Checked Evan mapping, the newest `Evan rN` frozen copy. |
| `README.txt`                             | `docs/s8/kit-readme.txt` with commit and names filled.  |

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
- the generated default mapping differs from the newest frozen default revision in
  [mappings](mappings/README.md). Check the new default on the S8 and freeze it first.
- the generated Evan mapping is not a valid plist. The kit offers it as an opt-in layout;
  it has no frozen revision or attended S8 check.

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
2. Asks for Default (Return) or Evan (`e`). It copies only the chosen mapping into
   `~/Music/djay/MIDI Mappings/` and never overwrites an existing file. If it copies a
   mapping while djay Pro is running, it asks you to restart djay Pro and the launcher.
3. `S8Host --check`: Accessibility access for Terminal, with the macOS prompt, and a running
   djay Pro.
4. Asks for Controller Editor when it is closed
   ([S8 dark at start](translator-validation.md#s8-dark-at-start)).
5. Asks about deck LEDs and which deck the right panel shows.
6. Stops NIHardwareAgent and runs `S8Host` with `--screens` and `--layout`, plus `--leds`
   or `--deck N` if selected. It prints which matching mapping to pick in djay's MIDI
   settings on `Kontrol S8 MIDI Full`. The output also goes to `logs/s8-<UTC>.log`.
7. On Ctrl-C, and on every other way out, it starts NIHardwareAgent again.

## Limits

- Deck LEDs and panels in one run are not yet checked on the S8. Each was checked alone
  ([led-feedback-validation.md](led-feedback-validation.md),
  [panel-views.md](panel-views.md)).
- [Default](mapping.md) and [Evan](mapping-evan.md) are bridge layouts, not Controller
  Editor templates. Use the mapping that matches the launcher choice.
- No VU meters and no mixer LEDs
  ([#123](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/123)).
- Display buttons 1 and 4 belong to Native Instruments' agent and do nothing while it is
  stopped.
- The kit was not yet run on a second Mac.
