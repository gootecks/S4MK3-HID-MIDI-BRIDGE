---
summary: "S8 LED output for decks: report framing, write API, the 0x80 and 0x81 byte map, values and state."
type: reference
issues: [21, 22, 23]
---

# Kontrol S8 LED output

Status: only the HID report descriptor was read on this Mac, with `ioreg` and
without opening the device. Everything else comes from other people's tests,
tables and notes. Nothing here has been confirmed by a write to this S8. The
attended results will be recorded in
[led-output-validation.md](led-output-validation.md).

This note covers framing, the deck reports `0x80` and `0x81`, value encoding
and state. The mixer report `0x82`, the feature reports, deck selectors, VU
meters and the proposed Encdr corrections are in
[led-output-mixer.md](led-output-mixer.md). The contradiction rows and open
questions keep one numbering across both notes.

The descriptor and its parser are in
[captures/hid-descriptor-20261111/](captures/hid-descriptor-20261111/README.md).
Interface facts and input are in [hardware-notes.md](hardware-notes.md).

<?toc?>

- [Evidence levels](#evidence-levels)
- [Framing](#framing)
  - [How the three accounts fit together](#how-the-three-accounts-fit-together)
  - [What the host must be given](#what-the-host-must-be-given)
- [Deck reports 0x80 and 0x81](#deck-reports-0x80-and-0x81)
- [Value encoding](#value-encoding)
- [State behaviour](#state-behaviour)
- [Contradictions between sources](#contradictions-between-sources)
- [Open questions for the hardware run](#open-questions-for-the-hardware-run)
- [What the bridge will send](#what-the-bridge-will-send)

<?/toc?>

## Evidence levels

Every table row carries one of these levels.

| Level       | Meaning                                                          |
| ----------- | ---------------------------------------------------------------- |
| Observed    | Read on this Mac; the device was not opened                      |
| HW          | A source documents a physical test; the platform is named        |
| Table       | A descriptor or table; no physical test is documented            |
| Unconfirmed | A claim in research notes or a decompile                         |
| Inference   | A deduction made for this note, marked `[INFERENCE]` in the text |

"Capture" next to Table means the byte appears in a frame captured from
NI's own driver on Windows. It is not a write test.

Source names used below:

- Encdr: the `RufusIbiza/Encdr` repository. Paths are inside it.
  `ni_kontrol_s8.json` is `encdr/descriptors/ni_kontrol_s8.json`.
- Protocol: the `kontrol-s8-protocol` repository, an attributed reference
  (see [research-sources.md](research-sources.md)). Paths are inside it.
- Mixxx: the single-author Windows mapping, GPL, so only facts are used.
  `Mixxx:N` is line N of
  [Kontrol-S8-Mixxx-Windows-v1.0.js](https://raw.githubusercontent.com/gusgustavodj/kontrol-s8-mixxx-mappings/main/windows/mappings/HID/Kontrol-S8-Mixxx-Windows-v1.0.js).
  Its repository is
  [gusgustavodj/kontrol-s8-mixxx-mappings](https://github.com/gusgustavodj/kontrol-s8-mixxx-mappings).

## Framing

The S8 declares three output reports. The report ID comes first on the wire.

| Report | Payload | On the wire | Level    |
| ------ | ------- | ----------- | -------- |
| `0x80` | 118     | 119         | Observed |
| `0x81` | 118     | 119         | Observed |
| `0x82` | 73      | 74          | Observed |

Each value is 0..127 (descriptor). They go to interface 5, interrupt OUT
endpoint `0x03`, as a plain hidapi `write` with the ID as the first byte.
Offsets in this note are payload offsets. The wire index is the offset plus 1.

The "309-byte buffer" is not a wire format. 118 + 118 + 73 = 309 is only the
sum of the three payloads (framing note 1).

Corroboration that does not depend on the descriptor:

- Protocol `docs/overview.md:179`: "IF5/EP03 report 0x80 → report 0x81 →
  report 0x82 → EP0 Feature F8 … Historical EP03 reports are 119, 119, and
  74 bytes including ID". HW (Protocol, Windows, capture of Traktor startup).
- Mixxx:15490-15503: seed frames captured from real hardware, each "excludes
  its HID report ID". Decoded here: 118, 118 and 73 bytes. HW (Mixxx, Windows).
- Mixxx:15781-15783: `controller.send(payload, payload.length, reportId)`
  with the log text `endpoint: "IF5/0x03"`. Mixxx passes the payload without
  the ID and its HID layer adds it. HW (Mixxx, Windows).
- Encdr `s8_discovery/src/bin/touchstrip_blue_test.rs:28-36`: a 119-byte
  vector, `buf[0] = prefix`, `buf[1+o] = value`, `iface.interrupt_out(0x03,
  buf)`. HW (Encdr, platform unstated; the transport is nusb, not hidapi).
- Protocol `docs/usb-interfaces-endpoints.csv:11-12`: interface 5 is HID with
  `0x84` IN and `0x03` OUT, both interrupt, max packet 64. The host stack
  splits a 119-byte report across transfers. That is normal for HID. Table.

### How the three accounts fit together

| Account                           | Verdict                                        |
| --------------------------------- | ---------------------------------------------- |
| Descriptor: 118 / 118 / 73 per ID | Correct; matches captures, Mixxx and Encdr     |
| One 309 or 310-byte buffer        | Wrong as a wire format (framing note 1)        |
| Strips need 119-byte reports      | Correct; this is the canonical format (note 2) |

Framing notes:

1. Encdr docs `docs/hardware/ni_kontrol_s8.md:99-151` and
   `s8_discovery/TRAKTOR_MODE_TESTING.md:27-95` describe one buffer. So do the
   `_full` and `_standalone` variants of `ni_kontrol_s8.json`, with prefixes
   `80`/`81`/`82` or `f3` and a buffer of 309. `[INFERENCE]` The 309 figure is
   the Traktor decompile's absolute LED index space: `0x80` is abs 0-117,
   `0x81` abs 118-235, `0x82` abs 236-308
   (`docs/hardware/attempts/ni_kontrol_s8_traktor_decompile*.md:12-18`). It was
   mistaken for one report. The `_full` and `_standalone` descriptors are
   identical on `main` and on the bridge's pin. They were abandoned: they put
   mixer LEDs at `0x81` offset 222 and similar, which produced nothing
   (`docs/hardware/attempts/ni_kontrol_s8_mixer_leds.md:147-162`).
2. Encdr touchstrip attempts 1-7 used 309-byte or 123-byte buffers and lit no
   strip LED (`docs/hardware/attempts/ni_kontrol_s8_touchstrip_leds.md`).
   Attempt 8, test C (lines 183-189), sent `[prefix] + [118]` and lit the
   strip. They failed because of the wrong length, not because strips are
   special. Attempt 7 (lines 73-76) sent 309 bytes with prefix `0x80` and pad 1
   lit. `[INFERENCE]` The device or OS truncated the over-long report instead of
   rejecting it. Do not rely on that.

### What the host must be given

| API                          | First byte            | Length              | Level     |
| ---------------------------- | --------------------- | ------------------- | --------- |
| hidapi `hid_write` (Encdr)   | report ID             | 119 deck, 74 mixer  | Table     |
| IOKit `IOHIDDeviceSetReport` | same ID as `reportID` | 119 with ID (see 3) | Inference |
| Mixxx `controller.send`      | ID is separate        | payload 118, no ID  | HW        |

1. Encdr main's `encdr/src/hid.rs` (via `uses_report_ids`) and
   `encdr/src/led_builder.rs` flush prefix + buffer, 1 + 118 = 119 for a deck.
   hidapi on macOS strips nothing when the ID is not zero.
2. Mixxx uses a different API convention and puts the same bytes on the wire
   (Mixxx:15783).
3. `[INFERENCE]` hidapi's macOS backend passes the buffer including the ID byte
   with length 119 when the ID is not zero, so copy that convention for IOKit.
   This was not read in the research. The first write must return success
   with the length accepted.

These three reports do not use EP0 SET_REPORT. Only the `0xF4` feature report
does: control OUT, `wIndex=5`, `wValue=0x03F4` (Protocol
`docs/hid-output-leds.md:19`; Encdr main `feature_report_leds`). Encdr
`s8_discovery/TRAKTOR_MODE_TESTING.md` also tried SET_REPORT with
`wValue 0x0302 wIndex 0`. That is the wrong interface. Unconfirmed; it is
never documented as working.

Pacing and init:

- Encdr main flushes once per batch with no pacing. Its Vegas example sleeps
  30 ms between frames. It has no `init_writes` for the S8
  (`encdr/src/led_builder.rs`). Table.
- Windows Traktor startup order is `0x80`, `0x81`, `0x82`, then the `0xF8`
  feature report, then screens (Protocol `docs/overview.md:179`). The order is
  not proven necessary.

## Deck reports 0x80 and 0x81

Offsets are payload offsets 0..117. Left and right have the same layout in
every source: Encdr groups `left_deck` and `right_deck`, and Protocol rows are
duplicated for `0x80` and `0x81`.

Offset conventions are as follows:

- Encdr JSON offsets exclude the ID (buffer 118).
- Protocol CSVs carry `payload_offsets` and `raw_offsets_with_report_id`. Raw
  is payload plus 1 (`data/ATLAS_OUTPUT_STATE_INDEX.csv:2`: "raw offset
  includes Report ID").
- Mixxx indexes are payload offsets (Mixxx:15505-15506).

The Encdr map is `ni_kontrol_s8.json`. It is identical on `main` (`3698b09`),
on upstream v0.9.0 (`a541465`) and on the pin. The pin lacks only
`protocol: linear_8bit` and the `ni_service` quirk. Protocol rows come from
`data/ATLAS_OUTPUT_STATE_INDEX.csv` and `data/HID_OUTPUT_LEDS.csv`.

| Offset  | LED                        | Colour | Level                | Note |
| ------- | -------------------------- | ------ | -------------------- | ---- |
| 0-23    | Pads 1-8, 3 bytes each     | RGB    | Conflict; HW (Encdr) | 1    |
| 24      | FX select                  | single | Table, capture       | 2    |
| 25-28   | FX 1-4 buttons             | single | Table; name conflict | 3    |
| 29-32   | Display buttons, left 1-4  | single | Table                | 4    |
| 33-36   | Display buttons, right 1-4 | single | Table                | 4    |
| 37      | BACK                       | single | Table, capture       | 5    |
| 38      | CAPTURE                    | single | Table, capture       | 5    |
| 39      | EDIT                       | single | Table, capture       | 5    |
| 40-43   | Slot / filter buttons 1-4  | single | Table                | 6    |
| 44      | Hotcue, white companion    | white  | Table, capture       | 7    |
| 45      | Hotcue mode button         | blue   | Table, capture       | 7    |
| 46      | Loop, white companion      | white  | Table, capture       | 8    |
| 47      | LOOP button                | blue   | Table                | 8    |
| 48      | FREEZE, white companion    | white  | Table, capture       | 9    |
| 49      | FREEZE                     | blue   | Table, capture       | 9    |
| 50      | REMIX, white companion     | white  | Table, capture       | 10   |
| 51      | REMIX                      | blue   | Table, capture       | 10   |
| 52      | FLUX                       | single | HW (Protocol, Win)   | 11   |
| 53      | Deck indicator, white      | white  | Table, capture       | 12   |
| 54      | Deck indicator, blue       | blue   | Table, capture       | 12   |
| 55      | SHIFT                      | single | Table, capture       | 13   |
| 56      | SYNC, green (Encdr only)   | green  | Unconfirmed          | 14   |
| 57      | SYNC                       | red?   | Conflict (name only) | 15   |
| 58      | CUE                        | single | HW (Protocol, Win)   | 16   |
| 59      | PLAY                       | single | HW (Protocol, Win)   | 16   |
| 60-63   | Loop ring, white 1-4       | white  | Table, capture       | 17   |
| 64-67   | Loop ring, blue 1-4        | blue   | Table                | 18   |
| 68-92   | Touchstrip, 25 LEDs        | blue   | HW (Encdr; Mixxx)    | 19   |
| 93-117  | Touchstrip, 25 LEDs        | orange | HW (Encdr)           | 20   |
| 118-121 | Deck selectors (rejected)  |        | Unconfirmed          | 22   |

Strip direction: no source states a left-to-right order (note 21).

Deck map notes:

1. Pad n starts at `3(n-1)`. Encdr has `r,g,b` at +0, +1, +2. Protocol says
   "THREE_BYTE_COLOR_VALUE_ORDER_UNRESOLVED"
   (`data/ATLAS_OUTPUT_STATE_INDEX.csv:18-25`). Mixxx writes B, G, R
   (Mixxx:15869-15872, 15918-15922). Encdr calls pads 1 and 2 "tested and
   confirmed" (`s8_discovery/LED_MAPPING_STATUS.md:13-15`) and pad 1 "red
   (offset 0, confirmed lit)"
   (`docs/hardware/attempts/ni_kontrol_s8_touchstrip_leds.md:73`). The Mixxx
   README calls per-pad hotcue colours "planned", so its order is not a
   physical result. HW (Encdr, platform unstated) favours R, G, B.
2. Encdr `fx_select`. Protocol `FX_SELECT_CAPTURED_PAIR`: "individual light
   response not independently established". Mixxx `FX_SELECT`, 0x7F/0x14.
   Colour is white with a question mark in the research.
3. Encdr names 25-28 `fx_1` to `fx_4`. Protocol and Mixxx name 25
   `FX_UNIT1_ON` and 26, 27, 28 `BTN1..3`. Protocol: "Validation pending".
   Mixxx binds 25 to effect enabled. Same bytes, different names.
4. Encdr: `screen_left_1..4` at 29-32 and `screen_right_1..4` at 33-36.
   Protocol lists only offset 32, as `DISPLAY_4_OR_8_LEFT` together with 36:
   "four fields change together; D4 and D8 not individually attributed".
   Mixxx writes none. Offsets 29-31 have no source other than Encdr. The
   Mixxx left base capture has 29 = 0x00, 30 = 0x14, 31 = 0x14, 32 = 0x7F and
   33-36 = 0x7F (Mixxx:15493, decoded here).
5. Encdr `back`, `capture`, `edit`. Protocol `BACK_LEFT/RIGHT` 37, then 38
   and 39. Mixxx lights them momentarily on press.
6. Encdr `on_1..4`. Protocol `SLOT_FILTER1..4` at 40-43: "Physical association
   gap". Mixxx `SLOT_FILTER1..4`.
7. Encdr `hotcue_white` 44, `hotcue_blue` 45. Protocol 44
   `DECK_MODE_MULTILED_GROUP` (0x00 to 0x14); 45 `HOTCUE_LEFT` (0x00 to 0x14,
   0x14 to 0x7F). Mixxx 44 `HOTCUE_WHITE` (deck C/D) 0x7F/0x14; 45
   `HOTCUE_BUTTON`.
8. Encdr `loop_white` 46, `loop_blue` 47. Protocol 46 is a multi-LED group;
   47 `LOOP_BUTTON` (off 0x14, on 0x7F). Mixxx 46 `LOOP_BUTTON_WHITE`; 47
   `LOOP_BUTTON` 0x7F/0x14.
9. Encdr `freeze_white` 48, `freeze_blue` 49. Protocol 48 `FREEZE_WHITE`; 49
   `FREEZE`, observed 0x14 to 0x7F to 0x14. Mixxx `FREEZE_WHITE`, `FREEZE`.
10. Encdr `remix_white` 50, `remix_blue` 51. Protocol 50 is a multi-LED group;
    51 `REMIX`, observed 0x14 to 0x7F to 0x14. Mixxx leaves 50 untouched and
    drives 51 as `REMIX`.
11. Encdr `flux`. Protocol `FLUX`: off 0x00, dim 0x14, on 0x7F. Mixxx `FLUX`
    0x7F/0x00. Physical pass for FLUX, CUE and PLAY on Windows with Mixxx
    (Protocol `docs/LED_PHYSICAL_VALIDATION.md:3`: "owner attested").
12. Encdr `deck_white` 53, `deck_blue` 54. Protocol `DECK_WHITE` 53 (0x00 and
    0x7F), `DECK_BLUE` 54. Mixxx `DECK_WHITE` (deck C/D, off 0x00) and
    `DECK_BLUE` (deck A/B). White is C/D and blue is A/B per Mixxx
    `CAPTURE_CONFIRMED_DECK_*` (Mixxx:15561-15567).
13. Encdr `shift`. Protocol `SHIFT_LEFT/RIGHT` 55. Mixxx: momentary.
14. Encdr `sync_green`. Not listed in Protocol. Mixxx leaves it untouched; its
    base frames have 0x14 on the left and 0x00 on the right. Evidence is
    Encdr only.
15. Encdr `sync_red`. Protocol `SYNC` 57: off 0x14, on 0x7F
    (`data/HID_OUTPUT_LEDS.csv`; `docs/hid-output-leds.md:10`: "Android/Windows
    policy differs; physical scope pending"). Mixxx `SYNC` 57, 0x7F/0x14. All
    three agree that 57 is the byte Traktor and Mixxx drive. Only the colour
    naming conflicts. `[INFERENCE]` 57 is red if Encdr is right. Byte 56 has
    no documented physical test.
16. Encdr `cue`, `play`. Protocol `CUE` 58 (off 0x00, on 0x7F) and `PLAY` 59.
    Mixxx `CUE`, `PLAY`. Same physical pass as note 11.
17. Encdr `loop_circle_1..4`, white. Protocol: multi-LED group 60-63 (0x00 to
    0x14). Mixxx `LOOP_RING_WHITE` (deck C/D ring).
18. Encdr `loop_circle`, blue. Protocol `LOOP_RING` 64-67: off 0x14, on 0x7F.
    Mixxx `LOOP_RING` (deck A/B ring), animated. The Mixxx README does not
    commit to the ring being tested.
19. Encdr `touchstrip_blue` (also `strip`), offset 68, count 25. Protocol
    `TOUCH_STRIP_*_SEGMENT_BANK` 68-92, 0x00 and 0x7F. Mixxx phase meter:
    clears 68-92 and 93, then lights 68..92 around centre 80. HW (Encdr,
    platform unstated) lit the strip. HW (Mixxx, Windows) uses it as a phase
    meter.
20. Encdr `touchstrip_orange`, offset 93, count 25. Protocol indexes only the
    blue bank 68-92. Mixxx clears only 93 ("orange center fallback"). Encdr's
    orange-only test is "confirmed working"
    (`s8_discovery/src/bin/touchstrip_blue_test.rs:75-78`). Blue and orange
    together look purple
    (`docs/hardware/attempts/ni_kontrol_s8_touchstrip_leds.md:198-205`, from an
    earlier read).
21. `[INFERENCE]` Mixxx's phase-meter centre at 80 (68 + 12) implies offset 68
    is one end, 92 the other and 80 the middle LED. Which end is physically
    left is unknown.
22. Not in Encdr's JSON, Protocol or Mixxx. Encdr prose only
    (`s8_discovery/LED_MAPPING_STATUS.md:19-34`,
    `docs/hardware/attempts/ni_kontrol_s8_touchstrip_leds.md:163-169`). The
    range lies outside the 118-byte payload. See
    [Deck selectors](led-output-mixer.md#deck-selectors).

## Value encoding

Every output byte has a logical range of 0..127 (Observed, descriptor).

| Item          | Fact                                             | Level       | Note |
| ------------- | ------------------------------------------------ | ----------- | ---- |
| ON            | `0x7F`                                           | HW (Win)    | 1    |
| OFF           | `0x00` for the single-state class                | HW (Win)    | 2    |
| DIM ("off")   | `0x14` for the dim class                         | HW, Table   | 3    |
| Intensity     | One linear byte; buttons only seen at 0x14, 0x7F | Table       | 4    |
| Pad colour    | Three bytes; Traktor values never exceed `0x1F`  | Table       | 5    |
| Above `0x7F`  | Encdr sends 255; conflicts with maximum 127      | Unconfirmed | 6    |
| `linear_8bit` | Encdr main tags dim 76 and bright 255            | Table       | 7    |

Notes:

1. Protocol `ATLAS_OUTPUT_STATE_INDEX.csv` `on_value`, and Mixxx. HW (source,
   Windows).
2. Off is `0x00` for FLUX, CUE, PLAY, DECK white and blue, strips and the
   white loop ring (ATLAS rows). HW (source, Windows).
3. Dim is `0x14` for SYNC, LOOP, FREEZE, REMIX, FX, slot buttons, SHIFT, BACK,
   CAPTURE, EDIT, FX-assign, snap and quantize. Mixxx's comment at
   Mixxx:15507-15509 says "0x00 is intentionally not generalized to them".
   Level: HW (source, Windows capture) for the observed 0x14, 0x7F, 0x14
   steps on FREEZE and REMIX; Table for the rest.
4. Protocol `HID_OUTPUT_LEDS_REFERENCE.md`: "0x00, 0x14, 0x7F remain
   distinct. Intensity values do not establish new colors".
5. Mixxx base frames have pad 1 = `1b 1f 0b` (left) and `10 04 00` on all
   eight pads (right) (Mixxx:15493, 15497). Protocol
   `data/PAD_COLOR_PALETTE.csv:2-18` has 17 entries with a maximum of `0x10`:
   RED `10 00 00`, GREEN `00 10 00`, BLUE `00 00 10`, WHITE `0E 10 05`, PURPLE
   `0A 00 0A`. Capture; palette is Table.
6. Encdr examples and tests send 255 on pads and strips
   (`touchstrip_blue_test.rs:66`; the Vegas example uses `Rgb 255`).
   `ni_kontrol_s8_touchstrip_leds.md:204` claims 0-255 analog brightness, from
   an earlier read. Mixxx also builds pad bytes from a full 24-bit RGB value
   (Mixxx:15869-15872), but its README says per-pad colours are not done. No
   source records how 0x7F and 0xFF differ. `[INFERENCE]` The device clamps,
   wraps bit 7, or uses it as a flag.
7. `main`'s `ni_kontrol_s8.json` quirks. It conflicts with 0x14 and 0x7F.
   For `0x82` the pin drops the tag, so Encdr's default `Nhl2` applies: dim
   228 and bright 158 (`encdr/src/core/descriptor.rs:299-303,316,324,333`). On
   `main` the values are 76 and 255 (`descriptor.rs:308-311,325-326,334-335`).
   The tags were removed on the pin in the diff `main` to
   `pin/5d7a689-deviceid-default` (JSON lines about 237, 304 and 369). All of
   these exceed 127. `[INFERENCE]` The bridge must send its own 0x7F and 0x14,
   not Encdr's `LedValue` mapping.

## State behaviour

- Full snapshot per report. Encdr's strip tests show that a `[prefix] +
  [118 zeros]` report with one offset set lights only that LED, and a
  following all-zero report clears it (`touchstrip_blue_test.rs:40-45`,
  `clear`). Each report acts as a full snapshot of its own bytes. HW (Encdr).
- Reports are independent. Encdr records that "hardware latches state; an
  all-zero buffer does not reset LEDs not included"
  (`docs/hardware/attempts/ni_kontrol_s8_touchstrip_leds.md:201-203`, from an
  earlier read). `[INFERENCE]` A write to `0x80` leaves `0x81` and `0x82`
  alone, and a short report leaves the tail unchanged. Unconfirmed on macOS.
- All offsets of a report share one buffer. Writing one offset in a snapshot
  does not change the others in the device. Offsets omitted from the next
  snapshot are expected to clear (the full-snapshot rule).
- Host exit. `[INFERENCE]` The device keeps the last LED state. Encdr's own
  wording is "hardware latches". On exit Encdr clears its buffers to 0 and
  writes them (`encdr/src/led_builder.rs`, from an earlier read). Mixxx
  re-seeds from captured frames and never rebuilds state, "because that could
  erase a previously updated LED" (Mixxx:15487-15489).
- Init. No source that lit `0x80` or `0x81` needed a handshake. Encdr's strip
  test C sent none, and Encdr main has no S8 `init_writes`. The `[F3, 01]`
  handshake is claimed necessary only for mixer LEDs
  (`docs/hardware/attempts/ni_kontrol_s8_mixer_leds.md:21`;
  [hardware-notes.md](hardware-notes.md) says it "lights the bank as a group
  only").
- NIHardwareAgent stopped. [agent-stop-validation.md](agent-stop-validation.md)
  (lines 67-69) records that panels keep their last labels. LED state during
  that run was not recorded: nothing was sent and no LED observation is
  written down. Recovery is `open -g NIHardwareAgent.app` (lines 106-110),
  which restores panel labels.
  [translator-validation.md](translator-validation.md) (lines 104-106 and
  366-388) records that the S8 was dark at the start with Controller Editor
  quit, cause unknown (issue #71).
- `[INFERENCE]` NI's agent repaints LEDs only from its own state when it
  restarts. Nothing documents it restoring a bridge-written LED pattern.
  Whoever restarts the agent restores whatever the agent holds, so the bridge
  must write zeros before handing back.
- `[INFERENCE]` The agent also writes `0x80`, `0x81` and `0x82`. Concurrent
  writers fight. The bridge design, with the agent stopped, avoids that.

## Contradictions between sources

Rows 11, 14-19 and 21-24 are in the [mixer
note](led-output-mixer.md#contradictions-between-sources). "Settled by" names the check or
rule.

| #   | Conflict                              | Verdict    | Settled by                |
| --- | ------------------------------------- | ---------- | ------------------------- |
| 1   | One 309/310-byte buffer vs 118/118/73 | Resolved   | Descriptor, captures      |
| 2   | Offsets with or without report ID     | Resolved   | Payload offsets; wire +1  |
| 3   | Pad byte order: Encdr RGB, Mixxx BGR  | Unresolved | Pad 1 byte test (Q7)      |
| 4   | SYNC 56 and 57 (Encdr) vs 57 only     | Unresolved | SYNC test (Q5, Q6)        |
| 5   | Offset 25: `fx_1` vs "FX unit on"     | Unresolved | Button test (Q14)         |
| 6   | Offsets 29-36 names vs D1-D8          | Unresolved | Group test (Q12, Q13)     |
| 7   | Brightness 0-255 vs descriptor 0..127 | Unresolved | 0xFF test (Q8)            |
| 8   | Dim 0x14 vs 0x00 as "off"             | Unresolved | Off test (Q17, Q21)       |
| 9   | Orange strip not in Protocol index    | Resolved   | Encdr test accepted       |
| 10  | Deck selectors at 118-121             | Resolved   | Outside payload; rejected |
| 12  | Pad palette max 0x10 vs 255           | Unresolved | 0xFF test (Q8)            |
| 13  | Strip direction, no source            | Unresolved | Strip test (Q9, Q10)      |
| 20  | Encdr dim/bright 76/255 or 228/158    | Resolved   | Out of range; Q8 clamp    |

Notes:

1. Row 7: use 0x7F in the bridge until tested. Row 8: 0x00 is off for FLUX,
   CUE, PLAY, DECK and strips, and 0x14 is dim for the rest. Whether 0x00
   also extinguishes a 0x14-class button is unresolved; Mixxx
   (Mixxx:15508-15509) refuses to generalise.
2. Row 20 is resolved as an Encdr defect, pending a hardware check of
   clamping.

## Open questions for the hardware run

Only a write on this S8 can answer these. Each is yes/no or one word. Questions 19-33 are in
the [mixer note](led-output-mixer.md#open-questions-for-the-hardware-run).

- **Q1.** Does `[80]` plus 118 zeros return 119 from a hidapi write? (yes/no)
- **Q2.** Does that write change any LED? (yes/no)
- **Q3.** After PLAY-only then CUE-only reports, is PLAY dark? (yes/no)
- **Q4.** Does a write to `0x80` change anything on the right deck or mixer? (yes/no)
- **Q5.** Is offset 57 SYNC lit red or green? (red/green)
- **Q6.** Is offset 56 a different LED from 57? (yes/no)
- **Q7.** For pad 1, which colour does byte 0, byte 1 and byte 2 give? (red/green/blue)
- **Q8.** Does `0xFF` on a pad or strip LED look different from `0x7F`? (yes/no)
- **Q9.** Which end of the strip does offset 68 light? (left/right)
- **Q10.** Is offset 80 the middle strip LED? (yes/no)
- **Q11.** What colour is the 68-92 bank, and what colour is 93-117? (one word each)
- **Q12.** Do offsets 29-36 each light a different display-side button? (yes/no)
- **Q13.** Is 29-32 the left cluster? (yes/no)
- **Q14.** Is offset 25 the FX-unit on button or the first FX slot? (on/slot)
- **Q15.** Do offsets 44, 46, 48 and 50 (white) and 45, 47, 49 and 51 (blue) light
  separate LEDs? (yes/no)
- **Q16.** Do `0x80` and `0x81` offsets 53 and 54 light the deck selector LEDs, white
  and blue, for the outer decks A/B and C/D? (yes/no)
- **Q17.** Does `0x00` extinguish a 0x14-class LED (FREEZE, REMIX, LOOP, SYNC)?
  (yes/no)
- **Q18.** After zeros and an agent restart, do all LEDs return without further
  action? (yes/no)

## What the bridge will send

- Reports `0x80` and `0x81`, from the bridge's own byte map, as 119-byte
  hidapi writes with values `0x00`, `0x14` and `0x7F`. Not Encdr's LED
  builder, whose values exceed the declared range.
- Report `0x82` is not sent by the bridge.
- Feature reports are not sent by the bridge: not `0xF3`, `0xF4` or any report
  in the never-write list.
