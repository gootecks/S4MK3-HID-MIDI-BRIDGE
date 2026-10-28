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
  packaging checks.
- hk hooks running formatting, Clippy, plist and shell checks, plus
  conventional-commit messages; hooks never rewrite files.
- Communiqué release-note and changelog tasks for this fork, with the LLM
  Gateway token supplied through fnox and 1Password.

### Changed

- README and `AGENTS.md` describe the fork, its project board and S8 hardware
  boundaries.

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
