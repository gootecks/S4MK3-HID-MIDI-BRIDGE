# S4 MK3 coupling audit

Where the bridge assumes an S4 MK3, as of upstream `ee12026`
(`v0.2.1-beta.1`). Line numbers drift; search for the symbol if they no
longer match.

## Device identity

| Location                          | Assumption                                                                  |
| --------------------------------- | --------------------------------------------------------------------------- |
| `src/main.rs:27-28`               | `S4_VENDOR_ID = 0x17cc`, `S4_PRODUCT_ID = 0x1720`                           |
| `src/main.rs:194-195, 226`        | HID enumeration filters on the S4 constants                                 |
| `src/main.rs:657, 700, 803`       | `registry.find(S4_VENDOR_ID, S4_PRODUCT_ID)` for full, probe and stage paths |
| `src/full/mod.rs:1300, 1380`      | Test helpers use the same lookup                                            |
| `src/screens.rs:159`              | Literal `0x17cc`/`0x1720`, requires exactly one device                      |
| `src/jog_diagnostic/live.rs:184`  | Literal `0x17cc`/`0x1720`                                                   |
| `src/midi.rs:10`                  | Minimal profile endpoint `S4 MK3 MIDI`                                      |
| `src/full/mapping.rs:18`          | Full profile endpoint `S4 MK3 MIDI Full`                                    |

## Input layout

- `src/main.rs:525-539` — `input_payload` accepts report 1 (22 bytes),
  2 (78) and 3 (59); other IDs are ignored.
- `src/main.rs:494-520` — `control_descriptor` corrects the pinned Encdr S4
  descriptor (button/jog packet sizes, 12-bit faders at 4095).
- `src/main.rs:541` onwards — `full_control_descriptor` swaps only the left
  MOVE/LOOP press names (see the encoder table in `AGENTS.md`).
- `src/input.rs` — analog snapshot is a fixed 79-byte array.

## Output layout

- `src/full/leds.rs:11` — 95-byte button/pad report; palette bytes in
  `Color::base()`.
- `src/full/output.rs` — 28-byte `0x30` init per side, 41-byte `0x32` jog
  rings, 79-byte `0x81` meters (4 decks × 15 bytes), pacing 10/20/50 ms.
- `src/jog/model.rs` — 11-byte `0x31` motor report; fader-based assist/brake
  thresholds.
- `src/screens/` — S4 screen transport and pages.

## Mapping and catalog semantics

- `src/full/catalog.rs` — physical names (`ch1_fader`, `left_jog_wheel`,
  `left_wheel_touch`, …) bound to Djay actions; four decks on two sides.
- `src/full/mod.rs` — `Deck { A, B, C, D }`, side routing, held-note and
  jog-touch ownership.
- Channel contract (`AGENTS.md`): decks 1–4, shifted 9–12, mixer 5, deck
  selection 6, feedback 7. The S8 keeps this contract.

## App and packaging

- `macos/S4BridgeApp.swift:724, 793, 801, 941`, `macos/build-app.sh:106` —
  mapping file name and Djay
  instructions name the S4.
- `macos/BridgeStatusMenu.swift`, `ScreenPrompts.swift`,
  `ScreenSettingsView.swift` — S4 screen and quit labels.
- `macos/Info.plist` `CFBundleName`, `macos/build-app.sh` `APP_NAME` —
  `S4 MK3 Bridge`.
- Settings for jog tension, backspin and S4 screens have no S8 equivalent.

## Large files

Over the 500-line guideline: `src/full/mod.rs` (3060),
`macos/S4BridgeApp.swift` (1306), `src/full/mapping.rs` (1210),
`src/full/leds.rs` (946), `src/main.rs` (855), `src/full/feedback.rs` (766),
`src/full/catalog.rs` (598). These are upstream files; add S8 code in new
modules rather than growing them.

## Device-profile seam (#5)

1. Done (#6): `src/device.rs` defines `DeviceProfile` with identity,
   display and endpoint names, input report sizes, mapping file name and a
   descriptor-correction hook. `S4_MK3` keeps upstream values; the S4
   corrections moved to `src/device/s4mk3.rs`.
2. Done (#7): the identity sites in the tables above use profile lookups
   (`S4_MK3.matches`, `full::PROFILE`). The generated S4 mapping is
   byte-identical to upstream, written to a file or a directory.
3. Done (#8): `--device s4mk3|s8` overrides detection; without it the bridge
   uses the single kind of supported controller connected. The S8 profile is
   identification only: the bridge logs its HID collections and exits without
   opening it. The macOS app passes `--device s4mk3`.
4. Open: per-profile catalog and mapping identity. The S8 profile reserves
   `Kontrol S8 MIDI Full` and `Kontrol S8 Bridge.djayMidiMapping`;
   `--generate-mapping --device s8` fails until the S8 catalog exists (#13,
   #14).
5. Open: leave LED, motor and screen byte layouts S4-specific until the S8
   equivalents are verified (#21, #26); then add S8 renderers alongside rather
   than generalising the S4 ones.
