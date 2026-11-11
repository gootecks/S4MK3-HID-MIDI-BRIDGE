---
summary: "S8 mixer LED output: report 0x82, VU meters, deck selectors, feature reports and Encdr corrections."
type: reference
issues: [24]
---

# Kontrol S8 mixer LED output

Status: unconfirmed. Nothing here has been confirmed by a write to this S8; the
descriptor facts are Observed. The attended results will be recorded in
[led-output-validation.md](led-output-validation.md).

This note covers the mixer report `0x82`, the feature reports `0xF3` and
`0xF4`, the never-write list, deck selectors and VU meters. Evidence levels,
source names, framing, the deck reports and values are in
[led-output.md](led-output.md). The contradiction rows and open questions keep
one numbering across both notes. Offsets are payload offsets; the wire index
is the offset plus 1.

<?toc?>

- [Mixer report 0x82](#mixer-report-0x82)
- [Feature reports](#feature-reports)
  - [0xF4: cue, filter and THRU](#0xf4-cue-filter-and-thru)
  - [0xF3](#0xf3)
  - [Never write](#never-write)
- [Deck selectors](#deck-selectors)
- [VU meters](#vu-meters)
- [Contradictions between sources](#contradictions-between-sources)
- [Open questions for the hardware run](#open-questions-for-the-hardware-run)
- [Proposed Encdr descriptor corrections](#proposed-encdr-descriptor-corrections)

<?/toc?>

## Mixer report 0x82

Payload offset is wire byte minus 1. Protocol states its own offsets include
the report ID (`docs/overview.md:11`), so its numbers are converted down by 1.
Mixxx and the Encdr descriptors already exclude the ID (Mixxx:15496-15509,
15516-15530).

| Offset | Name                | Level                      | Note |
| ------ | ------------------- | -------------------------- | ---- |
| 0-10   | VU ch3, 11 segments | Unconfirmed                | 1    |
| 11-21  | VU ch1, 11 segments | Unconfirmed                | 1, 2 |
| 22-32  | VU ch2, 11 segments | Unconfirmed                | 1    |
| 33-43  | VU ch4, 11 segments | Unconfirmed                | 1    |
| 44     | FX assign C-1       | Table; HW (Mixxx, Windows) | 3    |
| 45     | FX assign C-2       | Table; HW (Mixxx, Windows) | 3    |
| 46     | FX assign A-1       | Table; HW (Mixxx, Windows) | 4    |
| 47     | FX assign A-2       | Table; HW (Mixxx, Windows) | 4    |
| 48     | SNAP                | Table                      | 5    |
| 49     | QUANT               | Table; HW (Mixxx, Windows) | 5    |
| 50     | FX assign B-1       | Table; HW (Mixxx, Windows) | 6    |
| 51     | FX assign B-2       | Table; HW (Mixxx, Windows) | 6    |
| 52     | FX assign D-1       | Table; HW (Mixxx, Windows) | 7    |
| 53     | FX assign D-2       | Table; HW (Mixxx, Windows) | 7    |
| 54-71  | Unknown             | Unconfirmed for 62 and 63  | 8    |
| 72     | Unknown             | Unknown                    | 9    |

Mixer map notes:

1. A Traktor-decompile claim
   (`docs/hardware/attempts/ni_kontrol_s8_mixer_leds.md:50-57`). It has not
   been verified physically. Sweeps showed zero response (same file,
   lines 154-162).
2. The Mixxx seed has offset 11 = 0x7F and offset 12 = 0x7F, with offsets
   0-10 and 13-43 zero. `[INFERENCE]` These are the two lowest segments of ch1
   lit in a captured frame. That fits the claim but does not prove it.
3. Encdr `ni_kontrol_s8.json`, the `0x82` group (buffer size 73, lines about
   366-384): 44 is `mixer_fx_c_1_inactive`. Mixxx `FX_ASSIGN` payload index
   44 (Mixxx:15575-15592); seed 0x7F at 44 and 0x14 at 45. The Mixxx README
   lists "PFL, Mixer FX, per-deck FX 1/2 assignment LED feedback". That claim
   is owner-reported only.
4. Encdr `ni_kontrol_s8.json`. Protocol `data/HID_OUTPUT_LEDS.csv:13,56,
   106-111`: `FX_ASSIGN1_A` and `FX_ASSIGN2_A`, with-ID index 47. Protocol:
   "Physical light response not reported". Mixxx seed 0x14 at both.
5. Encdr `ni_kontrol_s8.json`. Protocol CSV: "physical not established".
   Mixxx seed 0x7F at 48 and 49. The Mixxx README claims only quantize visual
   feedback: "Quantize visual feedback works".
6. Encdr `ni_kontrol_s8.json`. Protocol CSV `FX_ASSIGN1_B` and the second B
   row. Mixxx seed 0x14 at both.
7. Encdr `ni_kontrol_s8.json` names 52 `d1`. Mixxx index 52; seed 0x14 at 52
   and 0x7F at 53. Not in the Protocol CSV, which has only byte-change rows
   for 44, 45, 52 and 53.
8. No named source. The decompile claims MIC at 62 and 63; the seed has them
   at 0.
9. The Mixxx seed has 0x7F here, with no documented meaning. `[INFERENCE]` It
   could be a clip, aux or booth indicator.

More facts about this map:

- The seed uses 0x7F for active and 0x14 for inactive on 44-53, with SNAP,
  QUANT, C-1 and D-2 at 0x7F. Mixxx's comment says 0x00 is "not generalized"
  (Mixxx:15516-15530). `[INFERENCE]` The `*_inactive` suffix in Encdr's names
  is misleading: 0x14 looks like a dim idle level, not off.
- The Mixxx seed is a "captured physical S8 IF5 frame" (Mixxx:15496-15509)
  from Windows. It is a live NI-driver frame, not a minimal write.
  `[INFERENCE]` Writing the seed whole is the safest baseline.
- Encdr keeps the mixer LED map unconfirmed
  (`s8_discovery/LED_MAPPING_STATUS.md:32-40`;
  `docs/hardware/ni_kontrol_s8.md:111,113-120,149`). `mixer_led_results.json`
  has `confirmed_leds=[]` and `probe_0x82.json` is empty.
- Protocol's `ATLAS_OUTPUT_STATE_INDEX.csv` has `0x82` rows for with-ID
  offsets 46-51 and byte-change-only rows for 44, 45, 52 and 53.
  `docs/hid-output-leds.md:14`: "0x82 46-51 fields present, physical policies
  incomplete".
- Segment order, per-channel master and clip LEDs: no source states them. No
  master meter or clip LED is documented. Unknown.
- The S5 template (`encdr/descriptors/ni_kontrol_s5.json:329-355`) puts cue
  A-D at 0-3 and filter A-D at 4-7 of its mixer `0x82` group. Then FX c1 44,
  c2 45, a1 46, a2 47, b1 48, b2 49, d1 50, d2 51, snap 52, quant 53. That
  order differs from the S8, which has snap and quant at 48-49 and b and d at
  50-53. The S5 offsets 0-7 are an untested template. The bridge notes
  (`docs/s8/mapping.md:192-197`: CUE 0-3, MIC 4, MIC2 5, SNAP 6, QUANT 7) are
  input names only. `[INFERENCE]` S5 offsets 0-7 may not carry over to the
  S8, where those LEDs come from `0xF4`.
- S4 MK3, for contrast: report `0x81`, `linear_7bit`, 14 segments plus clip
  per channel (ch1 0..13 and clip 14, ch2 15..28 and 29, ch3 30..43 and 44,
  ch4 45..58 and 59; S4 MK3 JSON lines 317-332). The Z2 has 7-segment
  meters (`z2.json:87-88`). The S8's 11 segments are not corroborated by a
  sibling device.

## Feature reports

The descriptor lists feature reports `0xF3` and `0xF4`, which can drive mixer
LEDs, and several that the bridge must never write.

### 0xF4: cue, filter and THRU

`0xF4` has 32 payload bytes (33 with the ID). Encdr's "33 bytes" is ID plus 32
and matches the descriptor. Mixxx's 32-byte buffer excludes the ID.

| Cmd    | Meaning             | Effect                      | Level                  | Note |
| ------ | ------------------- | --------------------------- | ---------------------- | ---- |
| `0x26` | CUE / PFL LEDs A-D  | Mask bit lit                | Table; HW (Mixxx, Win) | 2    |
| `0x25` | FILTER (FX on) LEDs | Mask bit lit                | Table; HW (Mixxx, Win) | 3    |
| `0x24` | THRU                | Direct THRU, not only a LED | Unconfirmed            | 4    |

Masks: A = 1, B = 2, C = 4, D = 8 (Encdr `quirks.feature_report_leds`, lines
423-441 of `ni_kontrol_s8.json`). The state is carried in one cumulative mask.

Notes:

1. Encdr sends a 33-byte buffer: `buf[0] = 0xF4`, `buf[1] = cmd`, `buf[2] =
   cumulative mask`, over EP0 class SET_REPORT (`encdr/src/device_thread.rs`
   `:910-930, 1860-1884`; `wValue = (3<<8)|id`, `wIndex` = interface 5).
   Encdr ORs and ANDs the mask per command (`device_thread.rs:1732-1757`).
   Mixxx uses `Uint8Array(32)`, `[0]` = command (0x25 filter, 0x26 PFL), `[1]` =
   `mask & 0x0F`, `sendFeatureReport(0xF4, buf)` (Mixxx:15958-15975,
   16086-16111). Protocol `HID_FEATURE_OUTPUTS.csv:2-3`: `MIXER_FILTER`, F4
   byte 25 (0x0F and 0x00), "Physical pass stated in source evidence"
   (Windows, Mixxx). `USB_CONTROL_OUTPUTS.csv:2-9`: `CUE_PFL` and
   `DIRECT_THRU` on F4 index 2 masks: "USB control OUT observed; not physical
   light". `docs/hid-output-leds.md:15-16,19`: F4 26 PFL transport observed,
   F4 24 THRU native mode inconclusive.
2. Encdr keeps state on the host and resends the cumulative mask.
   `[INFERENCE]` The device likely latches. Mixxx PFL is owner-reported
   working; the gate starts closed and the operator enables it (README and
   script).
3. Mixxx: "M285 proved on Windows", individual bits only. Multi-bit masks are
   a candidate.
4. Mask 1-8 toggles DIRECT_THRU per channel. Protocol `hid-output-leds.md:16`:
   native mode inconclusive. Mixxx `controls.md`: DIRECT THRU via feature
   report is not physically confirmed.

Risks:

- Sending `0x24` can change audio routing, not only lights. The worst
  plausible outcome is channel audio bypassing or being interrupted. Undo is
  `0x24` with mask 0, or a power cycle. `[INFERENCE]`
- Encdr's shutdown sends `0xF4` with a zero mask for all three commands
  (`0x24`, `0x25`, `0x26`; `device_thread.rs:1909-1925`). It therefore clears
  THRU on exit. If a channel was in THRU, which is a physical mixer function
  that bypasses the signal path, Encdr's shutdown takes it out. The bridge
  must not call that path on exit unless intended.
- Never send `0xF4` with a command byte other than `0x24`, `0x25` or `0x26`.
  The other 29 values are undocumented.

### 0xF3

The descriptor declares `0xF3` as a feature report with 2 payload bytes (3
with the ID), values 0..127, with the volatile flag. Observed.

Sources disagree about its kind:

- The Encdr attempts doc sends `[0xF3, 0x01]` as an interrupt-OUT report on
  interface 5. It lit all mixer LEDs as a group only
  (`docs/hardware/attempts/ni_kontrol_s8_mixer_leds.md:122-137`;
  `mixer_discovery_0xf3.log:2`). HW (Encdr, macOS per the research notes), as
  group-on only.
- `s8_standalone.json:18-28` treats `0xF3` as an interrupt OUT: prefix `0xf3`,
  buffer 309, item `mixer_mixer_mode_global` at offset 1.
- The bridge's [hardware-notes.md](hardware-notes.md) recorded "0xF3
  hardware-tested lights bank as group" (its lines 94-110 before this change).
- Mixxx has no `0xF3` (grep).

The descriptor wins: it is a feature report, not an interrupt OUT, and not 309
bytes. `[F3 01]` is the ID plus one payload byte, one short of the two the
descriptor wants. The "309-byte packet" is the concatenated `0x80`, `0x81` and
`0x82` payload sum, not one report.

Persistence is unknown. The meaning is
unknown; "mixer mode" or a handshake are candidates. `[INFERENCE]` The worst
case is mixer LEDs lit as a bank. Undo is a power cycle or a second write of
`[F3 00 00]`, which is not documented anywhere.

### Never write

The bridge never writes these. The descriptor facts are Observed.

| Report | Payload | Descriptor                                            |
| ------ | ------- | ----------------------------------------------------- |
| `0xF8` | 10      | 16x2 (0..1024), 8x3 (0..255), 8x2 (0..100), 1x8; 0x01 |
| `0xF9` | 10      | Same as `0xF8`                                        |
| `0xF2` | 6       | 8x6, 0..100, flags 0x02                               |
| `0xF0` | 82      | 8x82, 0..255, flags 0x82 (volatile)                   |
| `0xD0` | 256     | 8x256, 0..255, flags 0x82 (volatile)                  |
| `0xD1` | 256     | 8x256, 0..255, flags 0x82                             |
| `0xD8` | 32      | 8x32, 0..255, flags 0x82                              |
| `0xD9` | 32      | 8x32, 0..255, flags 0x82                              |

- `0xF8` and `0xF9`: two 16-bit values 0..1024, three 8-bit values, two 8-bit
  values 0..100 and 8 bits. Protocol `docs/overview.md:179` (section 12) says
  `0xF8` is sent at startup with a display and touch init,
  `E0 01 10 01 05 60 00 14 32 00`, length 11. `docs/overview.md:83` says IF5
  under usbhid failed on `0xF8`. `docs/overview.md:270` says the byte-8
  variants 0x14, 0x64 and 0x50 are unresolved.
  [screens.md](screens.md) covers its use for screens. `[INFERENCE]`
  Touchstrip or display configuration, or calibration.
- `0xF2`, `0xF0`, `0xD0`, `0xD1`, `0xD8`, `0xD9`: no source in either repo
  documents them. Grep found no rows in Protocol and no Encdr use.
  `[INFERENCE]` The 256-byte `0xD0` and `0xD1` and the 82-byte `0xF0` look
  like firmware, calibration or table blobs.
- No source warns explicitly, so treat all of them as never-write by default.
  `0xD0` carries the volatile flag, so a read is not safe to assume benign
  either. A GET_REPORT read is the only probe, and nothing in #24 needs it.

## Deck selectors

The claim "deck selector LEDs at offsets 118..121" cannot be a report offset.

- In the concatenated 309-byte buffer, abs 118..121 is `0x81` payload bytes
  0..3: the right deck's pad 1 RGB and pad 2 R.
- With report-ID-inclusive indices, index 118 is `0x80` payload byte 117, the
  last touchstrip byte, and 119 is past the end of `0x80`.
- `[INFERENCE]` The only convention that makes 118..121 plausible is the
  Traktor decompile's own internal layout: a "CUE slot abs 119/118/127/126"
  (`docs/hardware/attempts/ni_kontrol_s8_mixer_leds.md:78-96`). The Mixxx seed
  contradicts it: `0x81` seed bytes 0..23 are a repeating `10 04 00` pad
  pattern, not CUE lights.

Where the deck-button LEDs are: `0x80` and `0x81` offset 53 (`deck_white`) and
54 (`deck_blue`) (Encdr `ni_kontrol_s8.json`; Protocol CSV lines 77-78 and
120-121). The Mixxx seed has offset 54 = 0x7F on both `0x80` and `0x81`
(`deck_blue` lit) and 53 = 0. These are the A/B (left) and C/D (right)
selector lights of the outer decks. Level: Table, capture.

The mixer's own deck-assign A..D LEDs are different. Their inputs are
`deck_assign_a..d` (bytes 22 and 25, masks 02, 02, 01, 04;
`ni_kontrol_s8.json:73-98`). No source gives an output byte for them. Unknown.

## VU meters

One claim exists, and it is Unconfirmed: the Traktor decompile puts the
meters in `0x82` offsets 0..43, 11 segments each, in the order ch3, ch1, ch2,
ch4 (see the mixer map). Mixxx has no VU writes (grep). Encdr's sweeps in
`ni_kontrol_s8_mixer_leds.md:154-162` produced no response, but
`[INFERENCE]` they used the wrong framing if they sent a 309-byte buffer.

Unknown: segment order, clip LEDs, the master meter, and whether the device
drives the meters itself. The Mixxx README says "standalone mixer and Direct
Thru operation confirmed" as an existing hardware capability (Windows).
`[INFERENCE]` The signal meters are device-local in standalone-mixer mode.
That fits the lack of host VU writes in Mixxx and the partly lit offsets 11
and 12 in the seed. `[INFERENCE]` In that mode a `0x82` write of VU bytes may
be overridden or ignored, which could explain the zero-response sweeps. Offset
72 (seed 0x7F) is also unexplained.

## Contradictions between sources

Rows 1-10, 12, 13 and 20 are in the [deck
note](led-output.md#contradictions-between-sources). "Settled by" names the check or rule.

| #   | Conflict                                 | Verdict    | Settled by                |
| --- | ---------------------------------------- | ---------- | ------------------------- |
| 11  | Mixer on `0x81` 28-57 or abs index       | Resolved   | `0x82` 44-53 (note 1)     |
| 14  | FX assign/SNAP/QUANT on `0x81` 212/213   | Resolved   | `0x82` 44-53; stale json  |
| 15  | FX assign on `0x80` 78-81, collision     | Resolved   | No such outputs; stale    |
| 16  | S8 snap 48, quant 49 vs S5 order         | Resolved   | Different device (note 2) |
| 17  | CUE slots abs 119/118/127/126            | Resolved   | CUE is `0xF4` 0x26        |
| 18  | `0xF3` as 309-byte interrupt OUT         | Resolved   | Descriptor wins           |
| 19  | `0xF4` "33 bytes" vs 32                  | Resolved   | No conflict: ID + 32      |
| 21  | `s8_full.json` interface 0, `0x01/0x81`  | Resolved   | Stale; interface 5        |
| 22  | VU `0x82` 0..43 vs zero response         | Unresolved | VU test (Q24, Q25)        |
| 23  | Mixxx `*_inactive` as off vs 0x14 dim    | Unresolved | Off test (Q17, Q21)       |
| 24  | `0xF3` group light vs no `0xF3` in Mixxx | Unresolved | `0xF3` test (Q28, Q29)    |

Notes:

1. Encdr's decompile theory put the mixer on `0x81` at offsets 28-57. The
   descriptor, Protocol and Mixxx place it on `0x82` at 44-53.
2. The S5 JSON has b at 48-49, d at 50-51, snap 52 and quant 53. The S8 order
   is confirmed by the Mixxx seed and field map.
3. Row 14 also appears as `0x81` 212 and 213 for snap and quant, and `0x80`
   78-81 colliding with `deck_assign` 80-83, in `s8_full.json`.

## Open questions for the hardware run

Only a write on this S8 can answer these. Each is yes/no or one word. Questions 1-18 are in
the [deck note](led-output.md#open-questions-for-the-hardware-run).

- **Q19.** Does `82` plus 73 zeros (74 bytes) turn off every LED a `0x82` byte
  controls? (yes/no)
- **Q20.** With the Mixxx seed written (B0), are FX C-1, FX D-2, SNAP and QUANT lit
  while FX A-1, A-2, B-1, B-2, C-2 and D-1 are dim? (yes/no for each)
- **Q21.** Does `0x00` at `0x82` offset 46 turn FX A-1 fully off, not just 0x14?
  (yes/no)
- **Q22.** Does `0x82` offset 48 control SNAP, and offset 49 QUANT? (yes/no each)
- **Q23.** Which LED, if any, does `0x82` offset 72 control? (name or none)
- **Q24.** Do `0x82` offsets 11..21 light VU ch1 segments from the bottom up? (yes/no)
  Which offset is the lowest segment? (offset)
- **Q25.** Do offsets 0..10, 22..32 and 33..43 light the ch3, ch2 and ch4 meters?
  (yes/no each)
- **Q26.** With the agent stopped, does the device still drive the VU meters itself?
  (yes/no)
- **Q27.** Which LED, if any, does any of offsets 54..71 control, including 62 and 63
  for MIC? (name or none)
- **Q28.** Does `F3 01 00` light all mixer LEDs as a group on this unit? (yes/no)
- **Q29.** Does that group light persist after a `0x82` all-zero write? (yes/no)
- **Q30.** Does `F4 26 01` light CUE A, and `F4 26 0F` all four? (yes/no each)
- **Q31.** Does `F4 25 01` light FILTER A, and `F4 25 0F` all four? (yes/no each)
- **Q32.** Are `0xF4` LED states retained after the sender exits? (yes/no)
- **Q33.** Which byte, if any, lights the mixer deck-assign A..D LEDs? (offset or none)

## Proposed Encdr descriptor corrections

These are proposals for issue #24 against `RufusIbiza/Encdr`. They have not
been filed upstream. They need the user's approval before filing.

Supported by the observed descriptor now:

1. `ni_kontrol_s8.json` LED groups for `0x80`, `0x81` and `0x82`: the declared
   range is 0..127. Encdr's dim and bright bytes exceed it (76/255 for
   `linear_8bit` on `main`; 228/158 `Nhl2` default on the pin where the tags
   are gone). Add a 7-bit protocol tag, or explicit per-group `dim=0x14` and
   `bright=0x7F` for the S8. Evidence: the output rows of the
   [descriptor report table](captures/hid-descriptor-20261111/README.md#reports),
   `descriptor.rs:299-337`, and the `main` to pin diff.
2. `s8_standalone.json:18-28` models `0xF3` as an interrupt-OUT report of 309
   bytes. The descriptor says a feature report with 2 payload bytes. Fix or
   remove.
3. `s8_full.json` is stale: interface 0, endpoints `0x01/0x81`, `0x81`
   snap/quant/cue offsets, and `0x80` FX assign colliding with `deck_assign`.
   Mark it deprecated or fix it.
4. Docs that say a 309 or 310-byte "report": it is `0x80` (118), `0x81` (118)
   and `0x82` (73), three separate reports with IDs.
5. `quirks.feature_report_leds` `payload_length: 33` is correct as ID plus 32.
   Document that the `0xF4` payload is 32 bytes.
6. Add the remaining reports to the descriptor docs, with a "never write"
   note: `0xF0` (82), `0xD0` and `0xD1` (256), `0xD8` and `0xD9` (32), `0xF2`
   (6), `0xF8` and `0xF9` (10) and input `0x03` (96 bytes).

Needs the hardware test first:

1. Rename `mixer_fx_*_inactive` (`0x82` 44-53): the seed suggests the idle
   level is 0x14 dim, not off (Q20, Q21).
2. Confirm the SNAP physical LED (Q22).
3. Add VU groups (`0x82` 0..43 per the claim) only after Q24 and Q25. Segment
   order, clip and master are unknown.
4. Name `0x82` offset 72 and 54..71 only after Q23 and Q27.
5. Add mixer deck-assign A..D LEDs once a byte is found (Q33).
6. Name `0xF3` and document the `0xF4` THRU effect only after Q28-Q32 and a
   THRU test.
7. Check the S5 JSON mixer `0x82` group (cue 0-3, filter 4-7) against the
   S8: confirm the S8 sends cue and filter through `0xF4` only. Do not copy
   S5 offsets.
