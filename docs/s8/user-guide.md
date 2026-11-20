---
summary: "Playing a set with the Kontrol S8 and djay Pro through Kontrol S8 Bridge.app: setup, controls, screens, LEDs and limits, without MIDI knowledge."
type: procedure
issues: [19, 16]
---

# Kontrol S8 user guide

This guide is for playing with the Traktor Kontrol S8 in djay Pro through the
`Kontrol S8 Bridge` menu-bar app. It needs no MIDI knowledge. The S4 MK3 has its
own [user guide](../user-guide.md).

The app is built from source in this fork; there is no published download yet. How to
build, sign and copy it to another Mac is in [the S8 app](app.md).

## What you need

- An Apple Silicon Mac with macOS 12 or later.
- djay Pro with PRO access; custom MIDI mappings need it.
- The S8, powered and connected by USB.
- Native Instruments' software for the S8: Controller Editor and NIHardwareAgent.
- `Kontrol S8 Bridge.app`, ideally in `/Applications`.

## One-time setup

1. Open the app. Its icon sits in the menu bar; there is no Dock icon.
2. If macOS says the app cannot be opened, open System Settings > Privacy & Security and
   choose Open Anyway, or see [transfer and first start](app.md#transfer-and-first-start).
3. Choose Start once with djay Pro open. macOS asks for Accessibility access: allow
   Kontrol S8 Bridge, then choose Start again. The app uses this access to read djay's
   decks and library for the screens, and to switch djay's sidebar.
4. The app copies its two mappings into djay's MIDI mappings folder. It asks you to
   restart djay Pro once so that djay sees them.
5. In djay's MIDI settings, choose the device `Kontrol S8 MIDI Full` and the mapping the
   app's menu names, for example `Kontrol S8 Bridge r9`.

## Start and finish a set

Before playing:

1. Connect the S8 and open Controller Editor and djay Pro.
2. Choose Start in the app's menu. The menu shows Running and names the mapping to use.
3. Move each fader and knob once. The first movement after start only tells the bridge
   where the control is.

When finished, choose Stop or Quit. The app gives the S8 back to Native Instruments'
software; Controller Editor and your usual template work again.

Settings in the menu change only while stopped:

- **Layout:** Default, or Evan for Evan's Hotcues and Slicer layout. Each layout has its
  own mapping in djay; pick the one the menu names.
- **Deck LEDs:** the S8's lights follow djay. On by default.
- **Panels:** the two screens show djay's library and track data. On by default.
- **Right Panel:** what the right screen shows: the selected track (Library) or one deck.
- **Start When the App Opens:** starts a run as soon as the app opens.

## Controls (Default layout)

Each side controls one deck at a time. DECK switches the left side between A and C and
the right side between B and D. Hold SHIFT for the second function.

| Control        | Function                                | With SHIFT                     |
| -------------- | --------------------------------------- | ------------------------------ |
| PLAY           | Play / pause                            |                                |
| CUE            | Cue                                     | Jump to the start              |
| SYNC           | Sync                                    |                                |
| FLUX           | Slip mode                               | Reverse while held             |
| Loop encoder   | Turn: loop length; press: loop on / off | Turn: beat jump; press: reloop |
| Browse encoder | Turn: scroll the library; press: load   | Turn: move between sections    |
| BACK           | Switch library table                    | Switch back                    |
| EDIT           | Preview the selected track              |                                |
| FX SELECT      | Expand or shrink the library            |                                |
| Touchstrip     | Scratch                                 | Seek through the track         |
| Stem ON 1-4    | Mute a stem                             | Solo a stem                    |
| Stem faders    | Stem volumes                            |                                |

Pads: HOTCUE gives hot cues (SHIFT + pad clears one), SHIFT + HOTCUE gives held bounce
loops from 1/32 to 4 beats, FREEZE gives the Slicer, and REMIX the sampler. LOOP opens
djay's Looper view.

FX units: the FX buttons on each mixer channel choose which deck the left or right unit
works on. Buttons and dials 1-3 switch and adjust djay's three effect slots; button 4
switches the deck's effects on or off and dial 4 sets dry/wet.

Mixer: gain, EQ, filter, channel faders, crossfader and the mixer CUE buttons work in
djay. The tempo encoder changes the selected deck's tempo; pressing it resets it.
QUANTIZE toggles quantize. MAIN, BOOTH and the headphone controls set the S8's own audio
outputs.

The full list, with every MIDI address, is [the S8 mapping](mapping.md). The Evan layout
is described in [mapping-evan.md](mapping-evan.md).

## Screens and screen buttons

With Panels on, the left screen shows the list in djay's sidebar. The right screen shows
the selected track, or one deck if Right Panel names it: title, artist, BPM, key, time,
waveform and cue points.

The buttons beside the screens switch djay's sidebar:

| Left screen button | Sidebar tab |
| ------------------ | ----------- |
| 2                  | Library     |
| 3                  | Queue       |
| 5                  | Match       |
| 6                  | History     |
| 7                  | Info        |

On the right screen, buttons 2 and 3 switch between Info and Related while the Info tab
is open. Turning any screen encoder moves the selection in the list; clockwise moves
down. Buttons 1 and 4 belong to Native Instruments and do nothing while the app runs.

## LEDs

With Deck LEDs on, PLAY, CUE, SYNC, FLUX, the loop ring, the FX and stem buttons, the
mixer CUE buttons and the pads follow djay. Hot cue pads take the cue point's colour. The
touchstrip shows track progress as a blue bar and your finger as an orange dot. VU meters
and the other mixer lights do not follow djay.

## Troubleshooting

| Problem                                       | What to do                                                                   |
| --------------------------------------------- | ---------------------------------------------------------------------------- |
| The menu says Connect the Kontrol S8          | Check USB and power; exactly one S8 may be connected.                        |
| The menu asks to quit Traktor or Maschine     | Quit it; it would take the S8 from the app.                                  |
| The menu asks for Accessibility               | Allow Kontrol S8 Bridge in Privacy & Security > Accessibility, then Start.   |
| The S8 stays dark after Start                 | Stop, open Controller Editor, Start again.                                   |
| djay does not react                           | Check djay's MIDI settings: `Kontrol S8 MIDI Full` and the named mapping.    |
| Controller Editor does not work after the set | Open the app once; it restarts Native Instruments' agent if it was left off. |

Open Run Log in the menu shows what happened in the current run.

## Limits

- The touchstrip registers broad scrubbing only.
- CAPTURE has no visible result in djay; SNAP and MIC 2 are not assigned.
- The booth output was not checked.
- The app was checked on the development Mac only, not yet on a second Mac. See
  [app.md](app.md#verification) for what was checked.
