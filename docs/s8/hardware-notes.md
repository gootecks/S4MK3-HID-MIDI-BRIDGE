---
summary: "S8 USB identity, interfaces, input and output evidence, and open questions."
type: reference
issues: []
---

# Kontrol S8 hardware notes

Evidence levels used below:

- **Observed** — seen on this Mac without opening the device (`ioreg`).
- **Hardware-tested (Encdr)** — Encdr documents a successful hardware test.
- **Descriptor** — present in Encdr's loaded descriptor, without a documented
  hardware test.
- **Unconfirmed** — research notes or decompile claims; may conflict.

Input fields were checked control by control in the agent-stopped capture
(#10, [input-capture.md](input-capture.md)); where that record and the
descriptor summary below differ, the record wins. LED framing is #21.

## USB identity (observed)

| Field       | Value              |
| ----------- | ------------------ |
| Product     | Traktor Kontrol S8 |
| VID:PID     | `0x17CC:0x1370`    |
| Speed       | USB 2.0 High Speed |
| `bcdDevice` | `0x0050`           |

| Interface | Class           | Owner / use                                                                  |
| --------- | --------------- | ---------------------------------------------------------------------------- |
| 0         | Audio control   | Core Audio                                                                   |
| 1, 2      | Audio streaming | Core Audio (Djay audio goes here, not through the bridge)                    |
| 3         | MIDI streaming  | `MIDIServer`; silent in MIDI mode: the agent's virtual port carries the MIDI |
| 4         | DFU (`0xFE`)    | **Never touch**                                                              |
| 5         | HID             | Controls in, LEDs out (Encdr: IN `0x84`, OUT `0x03` interrupt)               |
| 6         | Vendor (`0xFF`) | Screens (Encdr: bulk OUT `0x04`); NIHardwareAgent holds it                   |

Interface ownership in the last column comes from Encdr docs and earlier
research, not from this bridge.

## Input (interface 5)

Source: `../Encdr/encdr/descriptors/ni_kontrol_s8.json` (loaded by
`encdr/src/device/loader.rs`; the `_full` and `_standalone` variants are not
loaded and look abandoned). Level: **descriptor**.

| Packet    | Size | Contents                                         |
| --------- | ---- | ------------------------------------------------ |
| `buttons` | 41   | Buttons, pads, tempo encoder, touchstrips, touch |
| `sliders` | 109  | Screen encoders, faders, FX dials, mixer knobs   |

Byte layout from the descriptor:

- `buttons`: deck and mixer buttons (bit masks, bytes 4–25), pads (digital,
  not velocity), tempo encoder (byte 3, wrap16), touchstrip positions (bytes
  28–29 left, 34–35 right, 10-bit) and touch bits.
- `sliders`: screen encoders (bytes 1–16, wrap16_wide), channel faders
  (17–42, 12-bit, max 4078), FX dials (43–58) and mixer gain/EQ/filter
  (69–108).

The macOS capture ([input-capture.md](input-capture.md)) resolved three
points. Report IDs 1 (41 bytes) and 2 (109 bytes) arrive in MIDI mode. The
kontrol-s8-protocol offsets match for every button tested. Shared-mode reads
coexist with NIHardwareAgent and Controller Editor.

With the agent running, CoreMIDI shows two S8 sources. `Traktor Kontrol S8`
is the S8's own USB MIDI port and carries no control data.
`Traktor Kontrol S8 Input` is the agent's virtual source with the MIDI of
the Controller Editor template.

The agent-stopped capture ([input-capture.md](input-capture.md)) corrects
this descriptor in several places:

- Touch caps are in report 1 (bytes 10/11 left, 18/19 right), not report 2.
- Touchstrips have no touch bit; position is 0 when released and 1–1024
  while touched. A report follows only after the position has moved about
  3 units, roughly every 13 ms during motion
  ([translator-validation.md](translator-validation.md#touchstrip)).
- Screen encoders wrap at 1000, not 65536, and one full turn is 1000 counts.
- EQ LOW on channels C and D is 12-bit at bytes 95–96 and 105–106.
- Crossfader assign is one three-position switch per channel (left sets the
  end bit, middle the middle bit, right neither), and channels B and D are
  swapped in the descriptor.
- Fader and knob full scale runs 4074–4095 depending on the control.

The bridge applies these corrections in `src/device/s8.rs`. It decodes the S8
read-only with `--device s8 --raw` (#12). `tests/s8_input.rs` replays the
captured reports through that decoder.

Still open: the right deck fader 1 touch bit, which never reported on this
unit.

## LED output (interface 5, OUT `0x03`)

| Group                           | Location                                                              | Level                                                    |
| ------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------- |
| Pads, RGB                       | `0x80`/`0x81` offsets 0–23                                            | Hardware-tested (Encdr)                                  |
| Deck single buttons             | `0x80`/`0x81` offsets 24–67                                           | Hardware-tested (Encdr)                                  |
| Touchstrip blue / orange        | `0x80`/`0x81` offsets 68–92 / 93–117                                  | Hardware-tested (Encdr), 119-byte reports                |
| Mixer FX assign, snap, quantize | `0x82` offsets 44–53                                                  | Descriptor; conflicts with `0x81` claims                 |
| Mixer CUE / FILTER / THRU       | EP0 SET_REPORT `0xF4`, 33-byte payload, commands `0x26`/`0x25`/`0x24` | Unconfirmed                                              |
| Whole mixer bank                | `0xF3` handshake `[0xF3, 0x01]`                                       | Hardware-tested (Encdr): lights the bank as a group only |
| VU meters                       | Candidate `0x82` ranges                                               | Unconfirmed, not in descriptor                           |
| Deck selectors                  | Offsets 118–121                                                       | Unconfirmed, not in descriptor                           |

Framing conflict: the descriptor uses 118/118/73-byte buffers per prefix.
Several Encdr docs describe one 309- or 310-byte buffer. The touchstrip test
only worked with 119-byte `[prefix] + [118]` reports.
`mixer_led_results.json` and `probe_0x82.json` in `../Encdr` are empty.

## Screens (interface 6)

- Two 480×272 displays, `bgr565_be` with full-blit headers per side in the
  Encdr descriptor (right footer byte 6 = `0x01`). Level: descriptor. The
  bridge's tests on macOS showed RGB565 big-endian, not BGR565, and full
  uncompressed frames display
  ([screen-frame-validation.md](screen-frame-validation.md)).
- Encdr v0.8.4 (`3698b09`, after this bridge's pin) adds an S8 route through
  the NI hardware service (CFMessagePort via `NIHostIntegrationAgent`), used
  when interface 6 cannot be claimed; `ENCDR_FORCE_SERVICE` forces it.
- Earlier passive capture research (see [README.md](README.md#prior-research-outside-this-repo))
  found the envelope `84 00 <selector> 60 00 00 00 00 <x> <y> <w> <h>` + encoded
  payload + `40 00 <selector> 00`, big-endian RGB565 inside mandatory run/delta
  encoding, two selectors at 480×272 each. Phase 4 showed the encoding is
  not mandatory: one LITERAL command covering the panel works. Selector 0 is
  the left panel, selector 1 the right.

Current protocol summary, source conflicts and the first-frame test plan:
[screens.md](screens.md).

## Not applicable

The S8 has no motorised platters: motor tension, backspin assist and jog-ring
LEDs from the S4 path do not apply. The touchstrips replace the jog wheels
(#13).

## Encdr pin

The bridge pins `gootecks/Encdr` branch `pin/5d7a689-deviceid-default`
(`8986302`).

- Base: RufusIbiza/Encdr `5d7a689` (2026-10-04).
- Addition: a cherry-pick of upstream `675c70c`, which derives `Default` for
  `DeviceId`.
- The S8 descriptor corrections need that derive (#12).
- Nothing else differs from `5d7a689`, which already contains
  `ni_kontrol_s8.json`.

Decision (#11): keep this pin. Upstream v0.9.0 (`a541465`, 52 commits after
`5d7a689`) was evaluated on 2026-11-10 and adds nothing the S8 path uses:

- Parser: pad-stream filtering for Maschine, NI-service ownership gating and
  a fallback for zero-padded reports. The bridge frames reports itself and
  runs with `NIHardwareAgent` stopped; the live `--raw` checks decode
  correctly on the current pin.
- Hotplug: an added controller filter API. The bridge only calls
  `scan_devices`.
- `ni_kontrol_s8.json`: LED protocol tags, an RLE screen protocol and an
  NI-service quirk. The bridge's S8 descriptor drops LEDs, screens and quirks,
  and screens use the bridge's own transport.

A trial build against v0.9.0 compiled without source changes and the S8
replay test passed. One S4 MK3 descriptor test failed: upstream added two
jog-position sliders to the S4 MK3 descriptor. Upstream also renamed the
`wheel_leds` group that the S4 path selects by name. Moving the pin therefore
needs S4 MK3 fixes and an attended S4 MK3 revalidation, and no S4 MK3 is
available. The full suite passes on the current pin.

Issue #61 tracks the move to an upstream release. It stays deferred until an
S4 MK3 is available or an upstream submission needs an upstream-reachable pin.
