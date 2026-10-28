# S8 research sources

Survey of outside S8 work, as of October 2026, and how this fork uses it.
Findings come from reading the repositories; none of their claims were
re-tested here unless the other S8 notes say so.

## kontrol-s8-protocol (primary reference)

[gusgustavodj/kontrol-s8-protocol](https://github.com/gusgustavodj/kontrol-s8-protocol),
five commits (September 2026).

- Machine-readable tables in `data/` (CSV and JSON): HID input (181 controls
  including the 16 display buttons), LED output, feature and control outputs,
  display init and payloads, USB descriptors, pad palette. HID input rows
  carry report ID, offsets with and without the ID, mask, encoding and an
  evidence tag.
- An interactive atlas in `atlas/` and a written spec in `docs/overview.md`.
  Some internal documents are in Portuguese.
- Evidence: Windows physical capture with NI's driver bound to the display
  interface; Android diagnostic only; "no operational macOS evidence". Raw
  captures are not published.
- Explicitly open: raw HID descriptor, `0xF8` meaning, RGB/BGR order,
  display shutdown, maximum frame size, latency, coexistence with NI software,
  touchstrip positions.
- LED coverage is partial: PLAY, CUE and FLUX have physical confirmation;
  SYNC, LOOP, pads, FX, SNAP and QUANTIZE do not.

It is preferred over Encdr for S8 facts because it is evidence-tagged and
marks its unknowns. Our macOS capture agreed with every HID field it tested
(see [input-capture.md](input-capture.md)).

## Encdr (secondary)

`../Encdr` (gootecks fork of RufusIbiza/Encdr) has an S8 descriptor and
hardware notes. It agrees with the protocol repo on endpoints, report sizes,
LED prefixes (`0x80`/`0x81`/`0x82`), the `0xF4` mixer LED feature report, and
the display footer. Its display notes have errors listed in
[screens.md](screens.md). Its NI-service screen route (v0.8.4) is the only
proposed way to draw on the panels while NI software keeps interface 6, but
it has no macOS evidence for the S8.

## Mixxx

- Official Mixxx has no S8 mapping. Its controller folder covers the S4 MK3,
  S2, S3, X1, Z1 and F1, and its issue tracker has nothing S8-specific.
- [gusgustavodj/kontrol-s8-mixxx-mappings](https://github.com/gusgustavodj/kontrol-s8-mixxx-mappings)
  is a Windows-only, single-author mapping with display support. It needs a
  patched Mixxx 2.5.6; macOS and Linux are untested. It claims four-deck
  control, waveform and browser displays, and lists REMIX, CAPTURE, Stems,
  Key Lock, SNAP and beatgrid editing as not working.
- Mixxx's S4 MK3 screen code (`TraktorKontrolS4MK3Screens.qml`) belongs to
  the same frame family as the S8 (`0x84` header, `0x40` footer) at 320×240.

Porting a Mixxx S8 mapping to Djay is not practical: Mixxx scripts read the
engine state directly (waveforms, track data), while Djay only exposes MIDI.
S8 panels driven by this bridge can show what the S4 panels show today
(mode legends and indicators), not waveforms or titles.

## Licensing

Not legal advice; review before shipping.

- Protocol facts (report IDs, offsets, masks, header bytes) are not
  copyrightable and can be used with a citation.
- kontrol-s8-protocol: code MIT, documents and tables CC BY 4.0, which
  requires attribution. Its rights matrix still says the code licence is
  pending, so treat it as unsettled. Copying its tables into this repo needs
  an attribution notice; this bridge's MIT notice does not cover it.
- Mixxx and the S8 Mixxx mappings are GPL-2.0-or-later. Do not copy or
  closely translate their JavaScript or QML into this MIT bridge. Implement
  from documented facts instead.
