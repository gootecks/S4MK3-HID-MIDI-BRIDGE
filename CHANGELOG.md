# Changelog

All notable changes to the gootecks fork are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow the
upstream release numbering.

Releases up to `0.2.1-beta.1` were published by
[jtoronto/S4MK3-HID-MIDI-BRIDGE](https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE);
their detailed notes stay in `docs/release-*.md`.

## [Unreleased]

### Added

- `--app-events` (Kontrol S8 only) prints the display buttons 2, 3 and 5-8
  and the screen encoders as `S8_CONTROL` lines on stdout. The `S8Host`
  macOS tool runs the bridge and uses them to switch djay Pro's sidebar tabs
  and Info > Related and to move the selection through Accessibility
  ([display-buttons.md](docs/s8/display-buttons.md), #51). Checked once on
  the S8
  ([record](docs/s8/display-buttons-validation.md)).
- `mise run s8:display-buttons` runs the attended check of the display
  buttons and screen encoders on the S8 (#51); it sends nothing to the S8.
- `S8Preview --stream left=N,right=M` writes the live djay deck pages as
  screen records for the bridge's `--s8-screens-stdin`, and
  `mise run s8:deck-page` runs the attended check of those pages on the S8
  panels ([plan and record](docs/s8/deck-pages.md), #85). Run on the S8 for
  decks 1 and 2 and, in Four Decks, for decks 3 and 4.
- Kontrol S8 project notes in `docs/s8/`: status, hardware evidence, S4 MK3
  coupling audit and the upstream contribution workflow.
- Pinned mise tooling with named tasks for the existing Rust, Swift and
  packaging checks. Rust is pinned in `mise.toml` instead of
  `rust-toolchain.toml`.
- hk hooks running formatting, Clippy, plist and shell checks, plus
  conventional-commit messages; hooks never rewrite files.
- SwiftLint with a baseline for existing upstream code, and SwiftFormat for
  Swift files added by the fork.
- Communiqué release-note and changelog tasks for this fork, with the LLM
  Gateway token supplied through fnox and 1Password.
- Deck page for the Kontrol S8 screens: `DjayDecks` reads deck state from
  djay through Accessibility and the analysis cache, and `S8Preview --deck N`
  renders it. Nothing is sent to the S8 yet.
- `--device s4mk3|s8` chooses the controller; without it the bridge uses the
  one kind of supported controller that is connected. `--list` also reports a
  connected Kontrol S8.
- `--device s8 --raw` reads the Kontrol S8 read-only and logs every verified
  control: buttons, pads, touch caps, step encoders, faders, knobs,
  touchstrips, crossfader assign switches and the eight screen encoders
  (#12). The S8 descriptor corrections from the #10 capture live in
  `src/device/s8.rs`; the bridge never writes to the S8, and `--midi` is
  refused for it until a mapping catalog exists. `tests/s8_input.rs` replays
  the captured reports through the decoder. `mise run s8:raw [seconds]` runs
  the decode.
- `--generate-mapping <path> --device s8` writes the default Djay mapping for
  the Kontrol S8 (`Kontrol S8 Bridge.djayMidiMapping`, endpoint
  `Kontrol S8 MIDI Full`) from the new S8 catalog in `src/s8/catalog.rs`
  (#14). Pads: HOTCUE sets and jumps to hot cues, Shift + HOTCUE plays bounce
  loops, FREEZE opens the Slicer and REMIX plays the sampler; each of these
  also switches Djay's pad view. LOOP shows or hides Djay's Looper. FX
  buttons and dials 1-3 are Djay's three effect slots, button 4 toggles FX
  and dial 4 is dry/wet for all slots. BACK switches the library table, FX
  SELECT expands the library and QUANTIZE toggles quantize. SNAP and MIC 2
  send notes with no Djay action, so they can be assigned in Djay.
  `docs/s8/mapping.md` lists every address.
- `--device s8 --midi full` publishes the virtual MIDI source
  `Kontrol S8 MIDI Full` and translates Kontrol S8 input to the default
  mapping (#15): deck and Shift layers, DECK switching, pad modes, FX unit
  targets, FILTER ON, crossfader assign and the touchstrips. It writes
  nothing to the S8 and has no LED or screen feedback. Five attended checks
  on 2026-11-10 covered the layout; CAPTURE and the booth output remain
  open. `docs/s8/translator-validation.md` holds the
  checks and their results, and `mise run s8:midi` runs one: it stops
  NIHardwareAgent, runs the bridge until Ctrl-C and restores the agent. The
  first control moved after start only synchronises positions.
- The touchstrip encoding is settled (#13): an absolute 7-bit position that
  Djay turns into scratch motion, or seek with Shift, and a note while
  touched. A capture of NIHardwareAgent's MIDI next to the raw strip
  position shows the agent sends the same value for the Controller Editor
  template. The user reports that a small back-and-forth motion scratches as
  it does with that setup. The capture is in
  `docs/s8/captures/2026-11-10-agent-strip-midi/`.
- The S8 CUE VOL, CUE MIX, BOOTH and MAIN knobs are decoded and mapped to
  Djay's monitor volume, cue/mix, booth volume and main volume (mixer channel
  CCs 25, 27, 28 and 29). Their report fields come from raw captures kept in
  `docs/s8/captures/2026-11-10-centre-knobs/` and
  `docs/s8/captures/2026-11-10-main-knob/`. CUE VOL, CUE MIX and MAIN are
  reported working in Djay; the booth output is not checked.
- `mise run s8:midi` writes its bridge log to
  `docs/s8/captures/translator-<UTC time>/` instead of `target/`.
- `docs/s8/mappings/Kontrol S8 Bridge r6 2026-11-10.djayMidiMapping` is a
  frozen copy of the S8 mapping that was checked in Djay on 2026-11-10, with
  install steps beside it. The README now says what works with the S8 from
  source and what is missing.
- The Encdr dependency now points at `gootecks/Encdr` branch
  `pin/5d7a689-deviceid-default`: the previous pin `5d7a689` plus upstream
  `675c70c` (`Default` for `DeviceId`), which the S8 corrections need.
- Device profiles (`src/device.rs`) hold each controller's USB identity, input
  report sizes, descriptor corrections, MIDI endpoint and mapping file name.
- `examples/s8-passive-recorder.rs` records S8 HID input and every S8 MIDI
  endpoint to JSONL, following endpoints that appear or vanish, and sends
  nothing to the S8.
- `examples/s8-input-diff` diffs S8 HID input reports per field with names from
  kontrol-s8-protocol and Encdr, step markers and a JSONL log, read-only on
  interface 5; `docs/s8/capture-checklist.md` lists every control for the
  agent-stopped capture (#10). `mise run s8:input-capture` walks that
  checklist at the S8: it stops NIHardwareAgent, stamps each row's label into
  the log and restores the agent.
- 2026-11-09: attended S8 input capture completed (#10). All 191 checklist
  rows were recorded with NIHardwareAgent stopped; the raw logs in
  `target/s8-captures/` were later lost with the build cache, a recovered
  copy of the reports is committed in `docs/s8/captures/`, and the
  per-control results are in
  `docs/s8/input-capture.md`. Touch caps sit in report 1 as
  kontrol-s8-protocol lists them, not where Encdr places them; the
  touchstrips report position only, with no touch bit; channel faders and EQ
  LOW on C and D are 12-bit; the screen encoders wrap at 1000 and have no
  detents; crossfader assign is a three-position switch on two bits, and
  Encdr has channels B and D swapped. A re-check the same day measured the
  screen encoders at 1000 counts per turn and confirmed the crossfader
  assign positions against the panel. Still unverified: the right deck
  fader 1 touch bit.
- `docs/s8/driverkit-evaluation.md` evaluates DriverKit, USBDriverKit,
  HIDDriverKit, AudioDriverKit and MIDIDriverKit for native S8 support, with
  entitlement costs and polish steps that need no driver extension (#55).
- S8 screen phase 0 (#32): `mise run s8:agent-stop` stops NIHardwareAgent,
  asks for presses at the S8 and restores the agent. Result in
  `docs/s8/agent-stop-validation.md`: S8 MIDI needs the agent, so driving
  the screens directly means the bridge must handle every S8 control.
- S8 screen phase 1 (#33): `examples/s8-screen-claim.rs` claims S8 interface
  6 for 5 seconds with no transfers; `mise run s8:screen-claim` runs it
  during an attended agent stop. Result in
  `docs/s8/screen-claim-validation.md`: the claim succeeds and disturbs
  nothing.
- S8 screen phases 2–4 (#34, #35): `examples/s8-screen-frame.rs` sends one
  frame to one S8 panel (`black`, `f800` or a full-panel `pattern`);
  `mise run s8:screen-frame first|pattern rgb` runs it during an attended
  agent stop. Result in `docs/s8/screen-frame-validation.md`: both panels
  display frames from the bridge's process, pixels are RGB565 and full
  uncompressed frames work, so no init report is needed.
- S8 screen phase 5 plan (#43): `examples/s8-screen-frame/` gains `stream`,
  `burst` and `partial` modes on the shared frame encoder, and
  `mise run s8:screen-stream` runs the attended steps. The plan is in
  `docs/s8/screens.md`. The frame runner now writes its logs to
  `docs/s8/captures/`.
- S8 screen phase 5 pictures (#43): `examples/s8-screen-frame/` gains `--png`,
  a dry run that decodes the frames it would send into PNGs;
  `mise run s8:phase5-images` regenerates or checks the expected pictures in
  `docs/s8/images/phase5/`.
- S8 screen phase 5 result (#43), in `docs/s8/screen-stream-validation.md`:
  the S8 takes about 60 full frames per second in total, so 60 on one panel
  or 30 on each of two; 30 per panel on both held for 60 s without a transfer
  error. Partial updates work with the SKIP command and with a sub-rectangle
  header. A tool killed mid-stream left the S8's startup screens on the
  panels, and the next frame displayed correctly.
- `--s8-screens-stdin` (Kontrol S8 only, needs `--midi`): the bridge reads
  RGB565 frame records for each panel from stdin, sends them to the S8
  screens with a latest-wins slot per panel and a 30 frames per
  second cap, and prints `S8_SCREEN_STATUS` lines. The channel is specified
  in `docs/s8/screen-transport.md`. Checked on the S8 in an attended run
  ([record](docs/s8/screen-transport-validation.md)), including a stop with
  Ctrl-C while it streams after the fix below (#44).
- `examples/s8-screen-feed.rs` writes `S8FR` records of the phase 5 test
  pattern to stdout (`--dry-run` touches nothing), and `mise run
  s8:screen-transport [first-step]` runs the attended check of the screen
  channel against the S8. The plan with expected pictures is in
  `docs/s8/screen-transport.md`.

### Changed

- README and `AGENTS.md` describe the fork, its project board and S8 hardware
  boundaries.
- Clippy passes on Rust 1.99.1. Four findings are fixed in code; the new
  `assert_is_empty` style lint is allowed crate-wide, because its suggested
  rewrite (`assert_eq!(value, [] as [T; 0])`) is less readable than
  `assert!(value.is_empty())`.
- The macOS app starts the bridge with `--device s4mk3`, so a connected S8
  does not stop it from finding the S4 MK3.

### Fixed

- The bridge no longer aborts when the reader of its stdout is gone, as
  happened on Ctrl-C in a pipeline during the screen transport check (exit
  status 134, no `S8_SCREEN_STATUS inactive`). Log lines to a closed stdout
  are dropped, on the S4 MK3 as well. With `--s8-screens-stdin` or
  `--app-events` the S8 bridge stops cleanly when a status or `S8_CONTROL`
  line cannot be written: exit frames, release, exit status 0. Covered by
  tests; Ctrl-C while streaming then passed on the S8 (#44).
- The deck page no longer runs the time of a paused deck ahead. djay blinks
  the play button of a paused deck, and the page took the lit half of a
  blink for playback. All decks are now read in one pass of djay's window,
  which keeps the page correct in the Four Decks layout. Checked against
  djay with two and four decks (#80).
- The deck page time no longer runs up to 1 s ahead of djay and no longer
  steps back on pause. A deck at the start of its track shows 0:00. djay
  shows whole seconds, and the page now keeps its time inside the second
  djay shows. Checked by replaying a recorded djay session, not yet seen on
  the S8. The time still stands until playback is confirmed and then moves
  forward once by less than 1 s (#83, #87).

## [0.2.1-beta.1] - 2026-10-26

Renames the 0.2.0-beta.2 build to 0.2.1-beta.1 and refreshes the Jog guide;
its notes cover everything since 0.2.0-beta.1. See
[docs/release-0.2.1-beta.1.md](docs/release-0.2.1-beta.1.md).

## [0.2.0-beta.2] - 2026-10-26

Configurable backspin continuation with motor assist, Shift + FX knob effect
browsing, lamp intensity and FX pickup warning fixes, and tuned backspin
defaults. See [docs/release-0.2.0-beta.2.md](docs/release-0.2.0-beta.2.md).

## [0.2.0-beta.1] - 2026-10-24

S4 screens, Preview/CUE/library control updates, startup channel
synchronization, FX pickup and jump-size restoration. See
[docs/release-0.2.0-beta.1.md](docs/release-0.2.0-beta.1.md).

## [0.1.1-beta.3] - 2026-10-21

See [docs/release-0.1.1-beta.3.md](docs/release-0.1.1-beta.3.md).

## [0.1.1-beta.2] - 2026-10-19

See [docs/release-0.1.1-beta.2.md](docs/release-0.1.1-beta.2.md).

## [0.1.0-beta.1] - 2026-10-06

First published beta of the S4 MK3 Bridge app.

[Unreleased]: https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE/compare/v0.2.1-beta.1...gootecks:S4MK3-HID-MIDI-BRIDGE:main
[0.2.1-beta.1]: https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE/releases/tag/v0.2.1-beta.1
[0.2.0-beta.2]: https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE/compare/v0.2.0-beta.1...v0.2.0-beta.2
[0.2.0-beta.1]: https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE/releases/tag/v0.2.0-beta.1
[0.1.1-beta.3]: https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE/releases/tag/v0.1.1-beta.3
[0.1.1-beta.2]: https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE/releases/tag/v0.1.1-beta.2
[0.1.0-beta.1]: https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE/releases/tag/v0.1.0-beta.1
