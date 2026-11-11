# Index

Where the documents are. `mdsmith fix` writes every list below from the files
themselves; do not edit a list by hand. The fork's notes show the `summary` and
the `issues` of their front matter. Upstream's documents have neither, so they
are listed by file name.

For the code, see [Architecture](AGENTS.md#architecture) and
[S8 code and commands](AGENTS.md#s8-code-and-commands) in `AGENTS.md`.

## Start here

- [`README.md`](README.md): what the app does, installing and using it.
- [`AGENTS.md`](AGENTS.md): rules, commands and code layout for work on this fork.
- [`CHANGELOG.md`](CHANGELOG.md): what changed in the fork.
- [`docs/s8/README.md`](docs/s8/README.md): status of the Kontrol S8 work.
- [`docs/user-guide.md`](docs/user-guide.md): upstream's guide to the S4 MK3 controls.

## Kontrol S8 notes

### Reference

<?catalog
glob:
  - "docs/s8/**/*.md"
where: 'type: "reference"'
sort: path
header: "\n"
row-expr: |
  "- [`\(filename)`](\(filename)): \(summary)" + strings.Join(
    [for i in issues {" [#\(i)](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/\(i))"}],
    ""
  )
footer: "\n"
?>

- [`docs/s8/deck-pages.md`](docs/s8/deck-pages.md): Deck page data from Accessibility and the analysis cache, snapshot fields, limits, preview commands and live validation. [#45](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/45)
- [`docs/s8/display-buttons.md`](docs/s8/display-buttons.md): Display buttons and screen encoders switching djay's sidebar tabs and moving the selection (S8Host). [#51](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/51)
- [`docs/s8/djay-sidebar-access.md`](docs/s8/djay-sidebar-access.md): Reading and switching djay Pro's sidebar tabs through Accessibility, with a probe and fixtures. [#48](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/48)
- [`docs/s8/driverkit-evaluation.md`](docs/s8/driverkit-evaluation.md): DriverKit, USB, HID, audio and MIDI driver extensions for a more native S8 setup: verdicts, entitlement cost and driverless polish. [#55](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/55)
- [`docs/s8/hardware-notes.md`](docs/s8/hardware-notes.md): S8 USB identity, interfaces, input and output evidence, and open questions.
- [`docs/s8/mapping.md`](docs/s8/mapping.md): The default Djay layout generated from the S8 catalog: addresses, pad modes, local and unassigned controls. [#14](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/14)
- [`docs/s8/mappings/README.md`](docs/s8/mappings/README.md): Frozen copies of Djay mappings that were checked on the S8, with install steps.
- [`docs/s8/panel-views.md`](docs/s8/panel-views.md): The library and deck views on the panels (S8Host --screens): run, expected pictures, state at stop, limits and check plan. [#27](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/27)
- [`docs/s8/research-sources.md`](docs/s8/research-sources.md): Survey of kontrol-s8-protocol, Encdr and Mixxx, with licensing notes.
- [`docs/s8/s4-coupling-audit.md`](docs/s8/s4-coupling-audit.md): S4 MK3 assumptions in the bridge with file anchors, and the device-profile seam. [#5](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/5)
- [`docs/s8/screen-transport.md`](docs/s8/screen-transport.md): The app-to-bridge channel for screen frames and status: record layout, status lines and worker policy. [#44](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/44)
- [`docs/s8/screens.md`](docs/s8/screens.md): Display protocol summary, source conflicts, macOS ownership and the phased first-frame test plan. [#26](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/26)

<?/catalog?>

### Procedures

<?catalog
glob:
  - "docs/s8/**/*.md"
where: 'type: "procedure"'
sort: path
header: "\n"
row-expr: |
  "- [`\(filename)`](\(filename)): \(summary)" + strings.Join(
    [for i in issues {" [#\(i)](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/\(i))"}],
    ""
  )
footer: "\n"
?>

- [`docs/s8/capture-checklist.md`](docs/s8/capture-checklist.md): Recorder usage and the ordered control-by-control checklist for the agent-stopped input capture. [#10](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/10)
- [`docs/s8/capture-recorder.md`](docs/s8/capture-recorder.md): Usage and verification status of the read-only S8 input recorder used for the agent-stopped capture. [#10](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/10)
- [`docs/s8/deck-pages-attended-check.md`](docs/s8/deck-pages-attended-check.md): Preview and stream commands for deck pages, and the plan for the attended check on the S8. [#85](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/85)
- [`docs/s8/upstream-workflow.md`](docs/s8/upstream-workflow.md): Remotes, branches, commit separation and the upstream pull-request procedure.

<?/catalog?>

### Validation records

<?catalog
glob:
  - "docs/s8/**/*.md"
where: 'type: "validation"'
sort: path
header: "\n"
row-expr: |
  "- [`\(filename)`](\(filename)): \(summary)" + strings.Join(
    [for i in issues {" [#\(i)](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/\(i))"}],
    ""
  )
footer: "\n"
?>

- [`docs/s8/agent-stop-validation.md`](docs/s8/agent-stop-validation.md): Phase 0: what changes when NIHardwareAgent stops, and the attended check (agent-stop-check.sh). [#32](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/32)
- [`docs/s8/deck-data-validation.md`](docs/s8/deck-data-validation.md): The attended check of live djay deck data on the panels: plan, expected pictures and the run on the S8. [#45](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/45)
- [`docs/s8/deck-pages-time-validation.md`](docs/s8/deck-pages-time-validation.md): The deck page time handling: the paused-deck fix and Four Decks, and the cut-off time label. [#80](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/80) [#83](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/83)
- [`docs/s8/display-buttons-validation.md`](docs/s8/display-buttons-validation.md): The stand-in test and the attended run of the display buttons on the S8. [#51](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/51)
- [`docs/s8/input-capture-rows.md`](docs/s8/input-capture-rows.md): Per-control results of the agent-stopped S8 input capture: field, observed range and status for all 191 checklist rows. [#10](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/10)
- [`docs/s8/input-capture.md`](docs/s8/input-capture.md): Passive HID and MIDI capture in Controller Editor MIDI mode: controls without MIDI and undocumented bytes. [#10](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/10)
- [`docs/s8/led-feedback-validation.md`](docs/s8/led-feedback-validation.md): Attended check plan and record for the S8 deck LEDs driven by --s8-leds from djay feedback (led-feedback-check.sh); not run yet. [#22](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/22) [#23](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/23)
- [`docs/s8/panel-views-validation.md`](docs/s8/panel-views-validation.md): The attended run of the panel views on the S8. [#27](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/27) [#89](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/89)
- [`docs/s8/screen-claim-validation.md`](docs/s8/screen-claim-validation.md): Phase 1: claiming interface 6 without transfers (screen-claim-check.sh). [#33](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/33)
- [`docs/s8/screen-frame-validation.md`](docs/s8/screen-frame-validation.md): Phases 2 to 4: first frames, colour order and a full uncompressed frame (screen-frame-check.sh). [#34](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/34) [#35](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/35)
- [`docs/s8/screen-stream-expected.md`](docs/s8/screen-stream-expected.md): Pictures of what each phase 5 step should show, decoded from the frames the tool sends (mise run s8:phase5-images). [#43](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/43)
- [`docs/s8/screen-stream-validation.md`](docs/s8/screen-stream-validation.md): Phase 5: repeated frames with a held claim, the transfer ceiling, partial updates and the exit state (screen-stream-check.sh, shared helpers in screen-lib.sh). [#43](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/43)
- [`docs/s8/screen-transport-validation.md`](docs/s8/screen-transport-validation.md): The attended check of the screen transport on the S8. [#44](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/44)
- [`docs/s8/translator-validation.md`](docs/s8/translator-validation.md): What the input-to-MIDI translator tests cover, their limits, and the attended checks of 2026-11-10. [#15](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/15)

<?/catalog?>

### Captures

<?catalog
glob:
  - "docs/s8/**/*.md"
where: 'type: "capture"'
sort: path
header: "\n"
row-expr: |
  "- [`\(filename)`](\(filename)): \(summary)" + strings.Join(
    [for i in issues {" [#\(i)](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/\(i))"}],
    ""
  )
footer: "\n"
?>

- [`docs/s8/captures/2026-11-09-input-capture/README.md`](docs/s8/captures/2026-11-09-input-capture/README.md): Recovered logs of the agent-stopped input capture of 2026-11-09. [#10](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/10)
- [`docs/s8/captures/djay-check-20261111T003350Z/README.md`](docs/s8/captures/djay-check-20261111T003350Z/README.md): Deck page against djay on 2026-11-10: cue points, analysis and the One Deck view. [#92](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/92) [#93](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/93) [#94](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/94) [#95](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/95)
- [`docs/s8/captures/one-deck-view-20261111T000921Z/README.md`](docs/s8/captures/one-deck-view-20261111T000921Z/README.md): Read-only Accessibility dumps of djay's One Deck view, taken on 2026-11-10. [#93](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/93)

<?/catalog?>

## READMEs next to the code

<?catalog
glob:
  - "**/*.md"
  - "!docs/s8/**"
where: 'type: "readme"'
sort: path
header: "\n"
row-expr: |
  "- [`\(filename)`](\(filename)): \(summary)" + strings.Join(
    [for i in issues {" [#\(i)](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/\(i))"}],
    ""
  )
footer: "\n"
?>

- [`macos/DjayDecks/README.md`](macos/DjayDecks/README.md): Reads djay Pro's decks for the S8 deck page: Accessibility state per deck plus per-track analysis from djay's metadata cache. [#45](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/45)
- [`macos/DjayLibrary/README.md`](macos/DjayLibrary/README.md): Read-only model of djay Pro's media library for the S8 screens. [#49](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/49)
- [`macos/S8Host/README.md`](macos/S8Host/README.md): Runs the Rust bridge as a child process and drives djay's sidebar from the Kontrol S8 display buttons and screen encoders. [#27](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/27) [#51](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/51)
- [`macos/S8Views/README.md`](macos/S8Views/README.md): Renders djay Pro's right-hand sidebar onto the two Kontrol S8 panels and encodes the result as S8 screen frames. [#50](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/50)

<?/catalog?>

Upstream's own, without summaries:

<?catalog
glob:
  - "macos/*.md"
  - "src/**/*.md"
sort: path
header: "\n"
row: "- [`{filename}`]({filename})"
footer: "\n"
?>

- [`macos/DESIGN.md`](macos/DESIGN.md)
- [`macos/README.md`](macos/README.md)
- [`src/screens/DESIGN.md`](src/screens/DESIGN.md)
- [`src/screens/text/README.md`](src/screens/text/README.md)

<?/catalog?>

## Upstream documents

<?catalog
glob:
  - "docs/*.md"
  - "docs/*/*.md"
  - "!docs/s8/**"
sort: path
header: "\n"
row: "- [`{filename}`]({filename})"
footer: "\n"
?>

- [`docs/app-screen-integration-validation.md`](docs/app-screen-integration-validation.md)
- [`docs/backspin-assist-validation.md`](docs/backspin-assist-validation.md)
- [`docs/backspin-detection-validation.md`](docs/backspin-detection-validation.md)
- [`docs/beta-release-validation.md`](docs/beta-release-validation.md)
- [`docs/bridge-function-reference.md`](docs/bridge-function-reference.md)
- [`docs/configurable-controls.md`](docs/configurable-controls.md)
- [`docs/connectivity-test.md`](docs/connectivity-test.md)
- [`docs/controls-state-sync-validation.md`](docs/controls-state-sync-validation.md)
- [`docs/controls-state-sync.md`](docs/controls-state-sync.md)
- [`docs/design.md`](docs/design.md)
- [`docs/dj-laptop-set-validation.md`](docs/dj-laptop-set-validation.md)
- [`docs/djay-action-reference.md`](docs/djay-action-reference.md)
- [`docs/encoder-capture.md`](docs/encoder-capture.md)
- [`docs/encoder-layout-validation.md`](docs/encoder-layout-validation.md)
- [`docs/full-midi-test.md`](docs/full-midi-test.md)
- [`docs/fx-knob-browsing-validation.md`](docs/fx-knob-browsing-validation.md)
- [`docs/hardware-bridge-research.md`](docs/hardware-bridge-research.md)
- [`docs/images/README.md`](docs/images/README.md)
- [`docs/issues.md`](docs/issues.md)
- [`docs/jog-fader-braking-protocol.md`](docs/jog-fader-braking-protocol.md)
- [`docs/jog-settings-validation.md`](docs/jog-settings-validation.md)
- [`docs/jog-tension-diagnostic.md`](docs/jog-tension-diagnostic.md)
- [`docs/jog-tension-integration.md`](docs/jog-tension-integration.md)
- [`docs/jog-wheel-investigation.md`](docs/jog-wheel-investigation.md)
- [`docs/key-feedback-order-validation.md`](docs/key-feedback-order-validation.md)
- [`docs/led-feedback.md`](docs/led-feedback.md)
- [`docs/led-fx-state-fixes-validation.md`](docs/led-fx-state-fixes-validation.md)
- [`docs/led-mapping-plan.md`](docs/led-mapping-plan.md)
- [`docs/macos-app-qa.md`](docs/macos-app-qa.md)
- [`docs/mapping-audit.md`](docs/mapping-audit.md)
- [`docs/midi-test.md`](docs/midi-test.md)
- [`docs/neural-mix-eq-check.md`](docs/neural-mix-eq-check.md)
- [`docs/release-0.1.1-beta.2.md`](docs/release-0.1.1-beta.2.md)
- [`docs/release-0.1.1-beta.3.md`](docs/release-0.1.1-beta.3.md)
- [`docs/release-0.2.0-beta.1-validation.md`](docs/release-0.2.0-beta.1-validation.md)
- [`docs/release-0.2.0-beta.1.md`](docs/release-0.2.0-beta.1.md)
- [`docs/release-0.2.0-beta.2-validation.md`](docs/release-0.2.0-beta.2-validation.md)
- [`docs/release-0.2.0-beta.2.md`](docs/release-0.2.0-beta.2.md)
- [`docs/release-0.2.1-beta.1-validation.md`](docs/release-0.2.1-beta.1-validation.md)
- [`docs/release-0.2.1-beta.1.md`](docs/release-0.2.1-beta.1.md)
- [`docs/screen-driver-handover-validation.md`](docs/screen-driver-handover-validation.md)
- [`docs/screen-driver-handover.md`](docs/screen-driver-handover.md)
- [`docs/screen-feedback-investigation.md`](docs/screen-feedback-investigation.md)
- [`docs/screen-prototype-validation.md`](docs/screen-prototype-validation.md)
- [`docs/screens.md`](docs/screens.md)
- [`docs/shift-sync-validation.md`](docs/shift-sync-validation.md)
- [`docs/tension-timeout-recovery.md`](docs/tension-timeout-recovery.md)
- [`docs/traktor-djay-comparison.md`](docs/traktor-djay-comparison.md)
- [`docs/traktor-ring-capture.md`](docs/traktor-ring-capture.md)
- [`docs/user-guide.md`](docs/user-guide.md)

<?/catalog?>
