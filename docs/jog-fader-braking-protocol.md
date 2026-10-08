# Fader braking protocol comparison

October 8, 2026. This is an offline source/packet check. No HID device was opened
for this increment, and physical resistance, direction, and shutdown acceptance
remain for the supervised morning run.

## Physical channel-1 input

The channel faders are ordered `3 | 1 | 2 | 4`; channel 1 is second from the left.
It is not a deck-layer-dependent MIDI control in this diagnostic.

The [pinned Encdr descriptor, line 158](https://github.com/RufusIbiza/Encdr/blob/5d7a689ca5789161b3efd1b2049d7e54504fe881/encdr/descriptors/ni_kontrol_s4_mk3.json#L158)
puts `ch1_fader` at payload bytes 2-3 in the 78-byte sliders report. Its
`max_value: 65535` is not the correct normalization range.

[Pinned Mixxx, line 1457](https://github.com/mixxxdj/mixxx/blob/f12661e29b36cfdd261939674f1ab94c52f1b2d8/res/controllers/Traktor-Kontrol-S4-MK3.js#L1457)
assigns channel-1 volume to `inByte: 2`. Its original `HIDInputReport` decodes
little-endian fields, and `incomingData` removes the numbered report-ID byte.
`Pot.prototype.inBitLength = 16` and
[`Pot.prototype.max = 2 ** 12 - 1`, line 1808](https://github.com/mixxxdj/mixxx/blob/f12661e29b36cfdd261939674f1ab94c52f1b2d8/res/controllers/Traktor-Kontrol-S4-MK3.js#L1808)
independently establish the 4095 range. The existing bridge already corrects
this descriptor range in `src/main.rs`.

Our capture stores numbered report 2, exactly 79 bytes including ID. It decodes
numbered bytes 3-4 and rejects a short/wrong-ID report or a value above 4095.
Historical captures without analog remain valid for fixed diagnostic modes;
fader-brake rejects missing analog rather than inferring a level.

## Motor payload and HIDAPI boundary

The original [Mixxx motor manager, lines 3303-3384](https://github.com/mixxxdj/mixxx/blob/f12661e29b36cfdd261939674f1ab94c52f1b2d8/res/controllers/Traktor-Kontrol-S4-MK3.js#L3303)
starts each wheel block as `[1,32,1,0,0]`. Negative signed requests replace the
middle word with bytes `[224,254]`; the last two bytes hold the bounded
unsigned magnitude, little-endian. The left block precedes the right block.

Its [original `motorCallback`](https://github.com/mixxxdj/mixxx/blob/f12661e29b36cfdd261939674f1ab94c52f1b2d8/res/controllers/Traktor-Kontrol-S4-MK3.js#L3746)
sends a ten-byte payload with report ID 49 (`0x31`) and strict FIFO enabled.
The [native FIFO, lines 26-28 and 70-72](https://github.com/mixxxdj/mixxx/blob/f12661e29b36cfdd261939674f1ab94c52f1b2d8/src/controllers/hid/hidioglobaloutputreportfifo.cpp#L26)
prepends that report ID before `hid_write`. Rust HIDAPI 2.6.7 likewise requires
the first byte to contain the ID. Both paths therefore submit **11 bytes**;
comparing Mixxx's ten-byte wrapper payload with our eleven-byte HIDAPI buffer
does not establish a transport discrepancy.

Examples including the mandatory report ID:

| Left request | Complete HIDAPI bytes; right wheel zero |
| --- | --- |
| Zero | `49,1,32,1,0,0,1,32,1,0,0` |
| -5000 | `49,1,224,254,136,19,1,32,1,0,0` |
| +5000 | `49,1,32,1,136,19,1,32,1,0,0` |
| -25000 | `49,1,224,254,168,97,1,32,1,0,0` |

## Original-source offline oracle

The complete upstream script at revision
`f12661e29b36cfdd261939674f1ab94c52f1b2d8` was fetched and archived locally.
Its SHA-256 is
`c058a168d537ccaf0374701004755af344c62a395d55e7d9f5aa668e9a8597c8`.
The executed regions were unchanged original `HIDInputReport`, `Component`,
`Pot`, `S4Mk3MotorManager`, `incomingData`, and `motorCallback` code.
Only Mixxx's engine/output boundary was captured, not its parser or encoder.

For packet comparison, synthetic jog speeds were chosen inside each desired
integer-magnitude floor bucket. This tests encoding and callback assembly,
not whether Mixxx's tension algorithm equals our direct fader policy.
All 33 packets from actual diagnostic replays at caps 3666, 5000, and 25000
matched the original manager/callback output exactly, including neutral,
negative and positive requests, and the right-wheel zero block.
The original input parser and potentiometer also matched all 11 fader positions.

Local evidence is in `.omo/evidence/jog-fader/protocol-oracle.json`,
`replay.jsonl`, `cli-errors.json`, and archived upstream sources.
These are local validation artifacts, not hardware captures.

Protocol agreement does not establish firmware interpretation, physical torque,
feel, or successful removal of drive. The diagnostic's raw values are not
factory tension settings or physical safety limits. Those conclusions require
the supervised physical checks in the suite's README.
