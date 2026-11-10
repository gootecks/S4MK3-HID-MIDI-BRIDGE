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

| Field       | Value                                  |
| ----------- | -------------------------------------- |
| Product     | Traktor Kontrol S8                     |
| VID:PID     | `0x17CC:0x1370`                        |
| Speed       | USB 2.0 High Speed                     |
| `bcdDevice` | `0x0050`                               |

| Interface | Class                  | Owner / use                                                   |
| --------- | ---------------------- | ------------------------------------------------------------- |
| 0         | Audio control          | Core Audio                                                     |
| 1, 2      | Audio streaming        | Core Audio (Djay audio goes here, not through the bridge)      |
| 3         | MIDI streaming         | `MIDIServer`; silent in MIDI mode: the agent's virtual port carries the MIDI |
| 4         | DFU (`0xFE`)           | **Never touch**                                                |
| 5         | HID                    | Controls in, LEDs out (Encdr: IN `0x84`, OUT `0x03` interrupt) |
| 6         | Vendor (`0xFF`)        | Screens (Encdr: bulk OUT `0x04`); NIHardwareAgent holds it     |

Interface ownership in the last column comes from Encdr docs and earlier
research, not from this bridge.

## Input (interface 5)

Source: `../Encdr/encdr/descriptors/ni_kontrol_s8.json` (loaded by
`encdr/src/device/loader.rs`; the `_full` and `_standalone` variants are not
loaded and look abandoned). Level: **descriptor**.

| Packet    | Size | Contents                                                                                       |
| --------- | ---: | ---------------------------------------------------------------------------------------------- |
| `buttons` |   41 | Deck and mixer buttons (bit masks, bytes 4–25), pads (digital, not velocity), tempo encoder (byte 3, wrap16), touchstrip positions (bytes 28–29 left, 34–35 right, 10-bit) and touch bits |
| `sliders` |  109 | Screen encoders (bytes 1–16, wrap16_wide), channel faders (17–42, 12-bit, max 4078), FX dials (43–58), mixer gain/EQ/filter (69–108) |

Resolved by the macOS capture ([input-capture.md](input-capture.md)): report
IDs 1 (41 bytes) and 2 (109 bytes) arrive in MIDI mode, the
kontrol-s8-protocol offsets match for every button tested, and shared-mode
reads coexist with NIHardwareAgent and Controller Editor.

The agent-stopped capture ([input-capture.md](input-capture.md)) corrects
this descriptor in several places:

- Touch caps are in report 1 (bytes 10/11 left, 18/19 right), not report 2.
- Touchstrips have no touch bit; position is 0 when released and 1–1024
  while touched.
- Screen encoders wrap at 1000, not 65536, and one full turn is 1000 counts.
- EQ LOW on channels C and D is 12-bit at bytes 95–96 and 105–106.
- Crossfader assign is one three-position switch per channel (left sets the
  end bit, middle the middle bit, right neither), and channels B and D are
  swapped in the descriptor.
- Fader and knob full scale runs 4074–4095 depending on the control.

The bridge applies these corrections in `src/device/s8.rs` and decodes the S8
read-only with `--device s8 --raw` (#12); `tests/s8_input.rs` replays the
captured reports through that decoder.

Still open: the right deck fader 1 touch bit, which never reported on this
unit.

## LED output (interface 5, OUT `0x03`)

| Group                                   | Location                             | Level                     |
| --------------------------------------- | ------------------------------------ | ------------------------- |
| Pads, RGB                               | `0x80`/`0x81` offsets 0–23           | Hardware-tested (Encdr)   |
| Deck single buttons                     | `0x80`/`0x81` offsets 24–67          | Hardware-tested (Encdr)   |
| Touchstrip blue / orange                | `0x80`/`0x81` offsets 68–92 / 93–117 | Hardware-tested (Encdr), 119-byte reports |
| Mixer FX assign, snap, quantize         | `0x82` offsets 44–53                 | Descriptor; conflicts with `0x81` claims |
| Mixer CUE / FILTER / THRU               | EP0 SET_REPORT `0xF4`, 33-byte payload, commands `0x26`/`0x25`/`0x24` | Unconfirmed |
| Whole mixer bank                        | `0xF3` handshake `[0xF3, 0x01]`      | Hardware-tested (Encdr): lights the bank as a group only |
| VU meters                               | Candidate `0x82` ranges              | Unconfirmed, not in descriptor |
| Deck selectors                          | Offsets 118–121                      | Unconfirmed, not in descriptor |

Framing conflict: the descriptor uses 118/118/73-byte buffers per prefix,
several Encdr docs describe one 309- or 310-byte buffer, and the touchstrip
test only worked with 119-byte `[prefix] + [118]` reports.
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
(`8986302`): RufusIbiza/Encdr `5d7a689` (2026-10-04) plus a cherry-pick of
upstream `675c70c`, which derives `Default` for `DeviceId`. The S8 descriptor
corrections need that derive (#12). Nothing else differs from `5d7a689`, which
already contains `ni_kontrol_s8.json`. The 49 later upstream commits include
the hidapi HID backend (`b87a283`), NI-service ownership gating (`fc9c730`),
the S8 service screen route (`3698b09`) and a 7-line S8 descriptor addition.
The bump decision is #11; moving the pin back to an upstream release needs an
attended S4 MK3 revalidation.
