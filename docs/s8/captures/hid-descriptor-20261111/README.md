---
summary: "The S8 interface 5 HID report descriptor read from the IORegistry, with its parser and report table."
type: capture
issues: [21]
---

# S8 HID report descriptor, 2026-11-11

Read on 2026-11-11 from the IORegistry of this Mac. The `ReportDescriptor`
property of the S8's HID interface holds the descriptor (508 bytes). The
device was not opened and nothing was sent. NIHardwareAgent, Controller Editor
and djay Pro were running. Evidence level: observed.

The node is `Traktor Kontrol S8`:

| Property               | Value    |
| ---------------------- | -------- |
| VID                    | `0x17CC` |
| PID                    | `0x1370` |
| VersionNumber          | `0x50`   |
| `MaxInputReportSize`   | 109      |
| `MaxOutputReportSize`  | 119      |
| `MaxFeatureReportSize` | 257      |
| `PrimaryUsagePage`     | `0xFF01` |
| `PrimaryUsage`         | 0        |

## Files

- `report-descriptor.hex`: the descriptor as hex, 508 bytes.
- `parse-descriptor.py`: prints the reports the descriptor declares. It reads
  the IORegistry or a hex file, opens no device and sends nothing.

Re-run the parser against the IORegistry with the S8 plugged in:

```sh
uv run docs/s8/captures/hid-descriptor-20261111/parse-descriptor.py
```

Run it on the saved hex instead:

```sh
uv run docs/s8/captures/hid-descriptor-20261111/parse-descriptor.py \
  docs/s8/captures/hid-descriptor-20261111/report-descriptor.hex
```

The parser reads these main items:

- `0x80`: input
- `0x90`: output
- `0xB0`: feature

It reads these globals:

- `0x84`: report ID
- `0x74`: report size
- `0x94`: report count

To read the property by hand:

```sh
ioreg -r -c IOHIDDevice -l -a -w 0
```

Then take `ReportDescriptor` from the node with VendorID 6092.

## Reports

Payload sizes exclude the report ID byte. On the wire an output report is the
ID plus the payload.

| Report | Kind    | Payload | With ID | Items                                    | Range  |
| ------ | ------- | ------- | ------- | ---------------------------------------- | ------ |
| `0x01` | input   | 40      | 41      | 4x6, 1x176, 16x1, 16x2, 16x1, 16x2, 1x24 | mixed  |
| `0x02` | input   | 108     | 109     | 16x8 (0..999), 16x46 (0..4095)           |        |
| `0x03` | input   | 96      | 97      | 8x96                                     | 0..255 |
| `0x80` | output  | 118     | 119     | 8x118                                    | 0..127 |
| `0x81` | output  | 118     | 119     | 8x118                                    | 0..127 |
| `0x82` | output  | 73      | 74      | 8x73                                     | 0..127 |
| `0xD0` | feature | 256     | 257     | 8x256                                    | 0..255 |
| `0xD1` | feature | 256     | 257     | 8x256                                    | 0..255 |
| `0xD8` | feature | 32      | 33      | 8x32                                     | 0..255 |
| `0xD9` | feature | 32      | 33      | 8x32                                     | 0..255 |
| `0xF0` | feature | 82      | 83      | 8x82                                     | 0..255 |
| `0xF2` | feature | 6       | 7       | 8x6                                      | 0..100 |
| `0xF3` | feature | 2       | 3       | 8x2                                      | 0..127 |
| `0xF4` | feature | 32      | 33      | 8x32                                     | 0..255 |
| `0xF8` | feature | 10      | 11      | 16x2 (0..1024), 8x3 (0..255), 8x2, 1x8   | see 1  |
| `0xF9` | feature | 10      | 11      | Same as `0xF8`                           | see 1  |

1. `0xF8`: two 16-bit values 0..1024, three 8-bit values 0..255, two 8-bit
   values 0..100 and one 8-bit group. Usages `0xE1`, `0xE2`; `0xE3` to `0xE5`;
   `0xE7`, `0xE6`; `0xE8`. Flags `0x01` constant and `0x02`.

Other descriptor fields:

- Usages: `0x01` input 0x03, 0x02, 0x04, 0x04, 0x32, 0x41, 0x0B, flags `0x02`;
  `0x02` input 0x06, 0x04, flags `0x02`; `0x03` input 0x40, flags `0x02`;
  `0x80`, `0x81` and `0x82` output usage `0x81`, flags `0x02`.
- Feature flags: `0x82` (volatile) on `0xD0`, `0xD1`, `0xD8`, `0xD9`, `0xF0`,
  `0xF3` and `0xF4`; `0x02` on `0xF2`.

## What this settles

- The device declares exactly three output reports: `0x80` and `0x81` with 118
  payload bytes each and `0x82` with 73, each value 0..127. On the wire that
  is 119, 119 and 74 bytes. 118 + 118 + 73 = 309, so the "309/310-byte
  buffer" in some Encdr docs is the three payloads joined in software, not one
  report.
- `0xF3` is a feature report of 2 payload bytes (3 with the ID), values
  0..127. `0xF4` is a feature report of 32 payload bytes (33 with the ID).
- `0xF8` and `0xF9` look like touchstrip calibration (two 16-bit values
  0..1024). `0xD0`, `0xD1`, `0xD8`, `0xD9`, `0xF0` and `0xF2` are
  undocumented. Input report `0x03` (96 bytes) is not in any S8 note yet.

## What it does not settle

The descriptor says nothing about which byte lights which LED, or whether the
firmware needs a handshake before it accepts LED reports. Those need sources
or a hardware test. See [led-output.md](../../led-output.md).
