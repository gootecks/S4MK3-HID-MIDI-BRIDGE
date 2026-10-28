# Changelog

All notable changes to the gootecks fork are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow the
upstream release numbering.

Releases up to `0.2.1-beta.1` were published by
[jtoronto/S4MK3-HID-MIDI-BRIDGE](https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE);
their detailed notes stay in `docs/release-*.md`.

## [Unreleased]

### Added

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
- `--device s4mk3|s8` chooses the controller; without it the bridge uses the
  one kind of supported controller that is connected. `--list` also reports a
  connected Kontrol S8, and the bridge identifies its HID collections, but it
  does not open or write to the S8 yet.
- Device profiles (`src/device.rs`) hold each controller's USB identity, input
  report sizes, descriptor corrections, MIDI endpoint and mapping file name.
- `examples/s8-passive-recorder.rs` records S8 HID input and every S8 MIDI
  endpoint to JSONL, following endpoints that appear or vanish, and sends
  nothing to the S8.
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

### Changed

- README and `AGENTS.md` describe the fork, its project board and S8 hardware
  boundaries.
- Clippy passes on Rust 1.99.1. Four findings are fixed in code; the new
  `assert_is_empty` style lint is allowed crate-wide, because its suggested
  rewrite (`assert_eq!(value, [] as [T; 0])`) is less readable than
  `assert!(value.is_empty())`.
- The macOS app starts the bridge with `--device s4mk3`, so a connected S8
  does not stop it from finding the S4 MK3.

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
