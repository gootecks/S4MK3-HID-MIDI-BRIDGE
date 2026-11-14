# Agent guide

## Project goal

Use the Native Instruments Traktor Kontrol S4 MK3 with Djay Pro on macOS.

The current product is an Apple Silicon menu-bar app bundling a Rust
HID-to-MIDI bridge, generated Djay mapping, and LED preferences. The DJ
laptop must not require Rust or Cargo.

A standalone hardware bridge is a later goal. Current source has optional
screens and motor-assisted jog tension. More screen pages, platter rotation
that follows playback and new haptics wait until someone asks for them.

## Architecture

- `src/main.rs`: CLI, HID opening, report framing, descriptor corrections,
  and bridge lifecycle.
- `src/input.rs`: full-profile input snapshots and stale-interrupt rejection.
- `src/midi.rs`: minimal MIDI profile.
- `src/full/catalog.rs`: shared physical-control and Djay-action catalog.
- `src/full/mod.rs`: deck/layer routing, held-note ownership, and MIDI emission.
- `src/full/mapping.rs`: generated native `.djayMidiMapping` plist.
- `src/full/feedback.rs`: virtual MIDI destination, decoding, and state cache.
- `src/full/leds.rs`: validated preferences and button/pad rendering.
- `src/full/output.rs`: native LED reports, ring presentation, and meters.
- `macos/S4BridgeApp.swift`: AppKit status item, SwiftUI settings, child process.
- `macos/build-app.sh`: standalone bundle and transferable archive.
- `examples/`: MIDI diagnostics and example LED configuration.
- `reference/`: supplied controller documentation.
- `docs/`: design, mapping references, capture evidence, and validation records.

## Read before changing behavior

- `docs/user-guide.md`
- `docs/mapping-audit.md`
- `docs/encoder-capture.md`
- `docs/djay-action-reference.md`
- `docs/bridge-function-reference.md`
- `docs/traktor-djay-comparison.md`
- `docs/led-feedback.md`
- `macos/README.md`

Historical validation records describe their tested revisions. Do not assume
the packaged app or older test counts describe the current working tree.

## Documentation audiences

- `docs/user-guide.md` is the canonical, workflow-oriented user guide. Keep it
  understandable without MIDI/development knowledge. When user-visible
  controls, mappings, feedback, setup or preferences change, update the
  relevant guide section in the same change alongside focused references.
  Check descriptions against the catalog, generator and runtime; keep current
  source-build behavior distinct from published-download availability.
- `README.md` is public-facing product documentation. Keep it focused on
  requirements, installation, current controls, usage, and known limitations.
  Distinguish published downloads from source-only features when necessary.
- Do not use README files as development diaries. Keep dated local-build
  updates, temporary artifact paths, test counts and transcripts, commit or
  approval history, and user-acceptance quotes out of them.
- Put internal build, investigation, and acceptance history in clearly named
  validation records under `docs/`, such as
  `docs/app-screen-integration-validation.md` and
  `docs/screen-driver-handover-validation.md`.
- A successful set is general usage evidence, not proof that every individual
  control or fault/recovery path was exercised. Preserve focused verification
  limits in the corresponding validation records.
- Keep user setup guides separate from those records. README links for setup
  and troubleshooting must lead to current instructions, not session history.
  `macos/README.md` may document developer build commands, but not internal
  progress updates.

## Commands

Run from the repository root:

       cargo fmt --all -- --check
       cargo clippy --locked --all-targets --all-features -- -D warnings
       cargo test --locked
       cargo build --locked --all-targets
       swift test --package-path macos
       bash -n macos/build-app.sh
       plutil -lint macos/Info.plist
       git diff --check

Build the portable app:

       bash macos/build-app.sh

Outputs:

- `dist/S4 MK3 Bridge.app`
- `dist/S4 MK3 Bridge-arm64.zip`
- `dist/S4 MK3 Bridge-0.2.1-beta.1-arm64.dmg`

The Cargo package and executable are `s4mk3-hid-midi-bridge`, release
`0.2.1-beta.1`. The bundle has version `0.2.1` and build `7`. Its custom
`S4BridgeReleaseVersion` key holds the full prerelease name.

Packaging makes the bundled `.icns` from `macos/Assets/AppIcon.png`.

The build script refuses to overwrite an existing app bundle. Preserve an
owned previous build before rebuilding. Do not delete unrelated artifacts.

Verify a rebuilt package:

       codesign --verify --deep --strict "dist/S4 MK3 Bridge.app"
       unzip -tq "dist/S4 MK3 Bridge-arm64.zip"
       hdiutil verify "dist/S4 MK3 Bridge-0.2.1-beta.1-arm64.dmg"

Signing is currently ad-hoc, not Developer ID signing or notarization.

## Mapping contracts

Keep runtime emission and generated mappings consistent. Read both the catalog
and generator when changing an address or action.

User-facing MIDI channels are one-based; plist channels are zero-based:

- Decks A-D: channels 1-4.
- Shifted deck actions: channels 9-12.
- Mixer: channel 5.
- Deck selection: channel 6.
- Dedicated playback/meter/loading feedback: channel 7.

A MIDI address includes endpoint, direction, channel, message type, and numeric
note/CC. Pitch names such as C2 are not sufficient identifiers.

Preserve the minimal profile. Preserve held-note and jog-touch ownership across
deck, Shift, pad-mode, and FX-target changes.

## Verified encoder fields

Payload offsets exclude the report ID:

| Physical knob | Rotation            | Press             |
| ------------- | ------------------- | ----------------- |
| Left MOVE     | Byte 19 low nibble  | Byte 6 mask 0x04  |
| Left LOOP     | Byte 19 high nibble | Byte 6 mask 0x20  |
| Right MOVE    | Byte 20 high nibble | Byte 15 mask 0x20 |
| Right LOOP    | Byte 21 low nibble  | Byte 15 mask 0x04 |

Clockwise is +1 and counterclockwise -1 modulo 16. Shift does not change these
fields. The pinned upstream rotation names are reversed on both sides, but
only the left press names are reversed. Do not blindly swap right presses.

Current custom Djay behavior: LOOP selects loop length; pressing LOOP activates
it. MOVE uses an independent jump length, jumping without a loop and moving
the loop when active. MOVE press toggles per-deck jump-size selection, with
Shift+MOVE always jumping one beat. The earlier shared-length proposal is
superseded. An encoder rename alone does not prove software behavior.

## Hardware and verification boundaries

- Check for competing bridge processes before opening HID.
- Do not claim audio interfaces for control translation.
- Master, booth, and headphone controls remain hardware-local.
- Do not send motor commands or screen traffic without explicit scope.
- Do not equate physical platter position with Djay playback position.
- Do not infer software state from button presses when real feedback is needed.
- Successful HID writes, generated keys, and passing unit tests do not establish
  physical behavior. Label untested behavior plainly.

Use deterministic regressions at existing test boundaries. Preserve real
decoder, translator, mapping, and feedback behavior in tests. Subscribe before
triggering asynchronous actions and use bounded timeouts; do not use fixed
sleeps to make tests pass.

## Workspace discipline

The working tree is shared and can contain user edits and unfinished work.

- Read files before editing; preserve changes you did not make.
- Use `apply_patch` for source and documentation edits.
- Never rewrite history or discard changes unless requested. Commit and push
  only as _Approvals_ in the fork section allows.
- Preserve user-edited Djay mappings under their existing names.
- Keep generated bundles, local capture logs, and large packet captures out
  of commits.

## Fork context (gootecks)

This section is fork-only. Keep it out of upstream pull requests.

- Repository: <https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE>, a fork of
  <https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE> (remote `upstream`).
  Encdr research fork: `../Encdr` (gootecks/Encdr; upstream RufusIbiza/Encdr).
- Goal: add Traktor Kontrol S8 (`0x17CC:0x1370`) support without changing
  S4 MK3 behaviour. `INDEX.md` lists every document. Start with `docs/s8/README.md`,
  `docs/s8/hardware-notes.md` and `docs/s8/s4-coupling-audit.md`. The
  screens epic (#25) is closed; what is left of it is low priority and
  collected in #105.
- Project board: **Kontrol S8 Bridge**, gootecks project #4
  (<https://github.com/users/gootecks/projects/4>). Use the `gootecks` gh
  account for issues and the board. A `GH_TOKEN` or `GITHUB_TOKEN` in the
  environment overrides it: unset both, take the token from
  `gh auth token --user gootecks`, and pass
  `-R gootecks/S4MK3-HID-MIDI-BRIDGE` to every command.
- Labels: `feat`, `fix`, `chore`, `docs`, `research`, `epic`,
  `priority:high|medium|low`, `device:s8`, `device:s4mk3`, `upstream:bridge`,
  `upstream:encdr`, `fork-only`, `hardware-write`.
- Workflow: issue, then a `type/N-description` branch from `main`,
  conventional commits, PR into the fork with `Closes #N`, or `Refs #N`
  while a check from _Verification_ below is still to be recorded. Never
  commit on `main` directly.
- After a merge: check that the issue closed and move its card to Done.
  Give each follow-up issue a card. Update status rows in
  `docs/s8/README.md` that still name the issue as open. GitHub closes
  neither a parent epic nor a milestone on its own. When the merge closes
  the last sub-issue of an epic, say so, and close the epic with a summary
  comment once the user agrees. Close a milestone once it has no open
  issues.
- Upstream friendliness: follow `docs/s8/upstream-workflow.md`. Never mix
  fork-only files (`docs/s8/`, `CHANGELOG.md`, `mise.toml`, `hk.pkl`,
  `.mdsmith.yml`, `.gitattributes`, `communique.toml`, `fnox.toml`, this
  section) with feature changes in one commit. Do not reformat or rename
  upstream code or docs without need.
- Record user-visible changes under `Unreleased` in `CHANGELOG.md` in a
  separate commit from upstream-candidate feature commits.
- S8 hardware: reading interface 5 HID input in shared mode is allowed for
  scoped work. Any S8 output (LED reports, feature reports, control transfers,
  screen/bulk traffic) needs a written test plan and explicit approval. Never
  touch interface 4 (DFU), firmware or USB mode.
- Tooling: `mise tasks` lists named tasks. `mise run check` runs the hk gates
  read-only; `mise run ci` adds the full test suites. Install hooks with
  `mise run hooks:install`. Release notes: `mise run release:notes`.
- Commits apply fixes: the pre-commit hook runs each step's fixer (cargo fmt,
  SwiftFormat, shfmt, oxfmt, mdsmith) and stages what it changed. Unstaged
  edits are set aside under `refs/hk/stash` while it runs and come back
  afterwards, uncommitted. A finding no fixer can repair blocks the commit.
- Toolchains: `mise.toml` pins Rust (replacing upstream's
  `rust-toolchain.toml`), SwiftLint and SwiftFormat. Swift itself comes from
  Xcode, not mise: the Swift.org toolchain mise installs is separate from the
  Xcode SDK and XCTest the app needs.
- Fork-owned files: a file is the fork's own when it is not in
  `upstream/main`. `docs/s8/upstream-files.txt` lists upstream's Markdown,
  Swift and shell files; `hk.pkl` reads it, `.mdsmith.yml` carries the same
  Markdown paths in a generated `ignore:` block, and `.gitattributes` takes
  the merge driver off them in another. A Markdown file created anywhere is
  therefore formatted, linted and merged without being added to a glob. Do
  not edit the list or those blocks by hand. After merging from upstream run
  `mise run upstream:files`; the hooks fail while any of them is out of date. The
  fork rewrites the root `README.md` and this file: the script leaves both off
  the list, so they are gated like the fork's own files. mdsmith matches an
  ignore pattern without a slash against the file name in every folder, so
  never ignore a root `README.md` there; it would ignore every README. Swift
  and shell steps do not use the list yet.
- Swift style: SwiftLint grandfathers upstream violations in
  `.swiftlint-baseline.json`; new violations fail. SwiftFormat checks only
  Swift files the fork adds; `hk.pkl` lists the upstream files it skips. Do
  not reformat upstream Swift. Regenerate the baseline only for deliberately
  accepted violations (`mise run lint:swift:baseline`).
- Docs and script style: oxfmt formats and mdsmith lints every fork-owned
  Markdown file (GitHub convention, `.mdsmith.yml`). `CHANGELOG.md` is
  formatted only. Everywhere lines are up to 95 characters and paragraphs
  are checked for readability. Every file in a folder needs front matter
  with `summary` and `type`; the root `README.md` has none, because GitHub
  would show it as a table on the front page, and this file has none either.
  A README next to the code uses `type: readme`. Every note also carries
  `issues`, the list of issue numbers it belongs to (`issues: [51]`, or
  `[]`): the index links them, and it fails on a note without the field.
  Do not write "(#51)" into a summary or an H1. Leave `title` out: the H1
  is the title. mdsmith pads a right-aligned table
  column differently from oxfmt, so keep table columns left-aligned. The
  index in the root `INDEX.md` is generated from the front matter by
  `<?catalog?>`, READMEs next to the code included, and notes over
  200 lines carry a `<?toc?>`. `mdsmith check` reports a stale index or table
  of contents and `mdsmith fix` regenerates it. A commit that touches a note
  or `.mdsmith.yml` lints every fork-owned note, not only the staged ones: a
  new or renamed note changes the index and the links in other notes. It
  stages the notes its fixer changed. A commit that only deletes notes is
  checked and not fixed: run `mdsmith fix .` and stage `INDEX.md`
  first.
  Pushing runs the same check on the committed tree, which also covers
  merges, rebases and `--no-verify` commits. Untracked and unstaged notes
  are set aside for both. mdsmith output is coloured; `NO_COLOR=1` turns
  that off. `mise run hooks:install` also registers mdsmith's merge driver.
  `.gitattributes` gives it every Markdown file except upstream's (a block
  written by `docs/s8/upstream-files.sh`), so generated sections no longer
  conflict. It writes the driver to the Git config and leaves
  `.gitattributes` alone: `mdsmith merge-driver install` and the
  `git-hook-sync` rule rewrite that file from the `ignore:` list and turn
  the driver off for every note. After a merge or rebase generated sections
  can still be stale: run
  `mdsmith fix .` again (`mise run check` reports it). shfmt formats
  `docs/s8/*.sh` (`docs/s8/.editorconfig`); shellcheck checks every script.
  oxfmt is the native binary from the oxc GitHub release, so no npm download
  is needed. Upstream Markdown and scripts keep upstream style.
- What `mdsmith fix` cannot repair (missing `summary`, long lines, link text,
  broken links) goes to the `repo-markdown-fixer` omp agent
  (`~/.omp/agent/agents/repo-markdown-fixer.md`, Sonnet) through
  `docs/s8/mdsmith-fix.sh`, the mdsmith step's fixer. It edits only the
  notes with findings, and its edits are staged into the commit; hk prints its
  report. Review them with `git show`. Without omp or the agent file, or with
  `MDSMITH_AGENT=0`, the commit stays blocked until the findings are fixed by
  hand. `mise run check` never runs the agent.

### S8 code and commands

- `src/device.rs`, `src/device/s4mk3.rs`, `src/device/s8.rs`: device
  profiles, and the S8 corrections to the pinned Encdr descriptor.
- `src/s8/catalog.rs`: every S8 MIDI address and Djay action. The mapping
  generator and the translator both read it; change an address there only.
- `src/s8/mapping.rs`: generated `Kontrol S8 Bridge.djayMidiMapping`.
- `src/s8/translator/`: S8 input events to the catalog's MIDI messages.
- `src/s8/strip.rs`: touchstrip position and ownership of one touch.
- `src/s8/bridge.rs`: the `--device s8 --midi full` loop. It sends nothing
  to the S8 except with `--s8-leds` (deck LED reports) or
  `--s8-screens-stdin`, which shows frame records from stdin on the panels
  (`docs/s8/screen-transport.md`).
- `src/s8/catalog/outputs.rs`: the feedback addresses Djay returns; the
  mapping generator and `src/s8/feedback.rs` both read them.
- `src/s8/feedback.rs`: decodes Djay feedback into per-deck state.
- `src/s8/led_state.rs`: Djay feedback plus translator state to the LEDs of
  one side. `src/s8/led_palette.rs` turns cue colour tokens into RGB.
- `src/s8/leds.rs`, `src/s8/leds/`: the model and the deck reports `0x80`
  and `0x81`; `map.rs` holds the byte offsets, `strip.rs` the strip banks.
- `src/s8/led_output.rs`: writes changed reports, paced, and clears on exit.
  A write error in the bridge turns the LEDs off for the run and logs
  `S8_LEDS_DISABLED`; MIDI translation goes on.
- `tests/s8_input.rs`: replays captured reports through the decoder.
- `docs/s8/mappings/`: frozen copies of mappings checked on the S8.
- `macos/Package.swift` builds the S8 screen tools, all the fork's own:
  - `S8Host`: runs the bridge, applies display buttons and screen encoders
    to djay's sidebar and, with `--screens`, sends the panel pages.
  - `S8Preview`: renders pages to pictures and, with `--stream`, streams
    deck pages without `S8Host`.
  - `S8Views`: page layout, renderer and the frame record format.
    `S8Session`: which page each panel shows. `S8Control`: control events
    from the bridge.
  - `DjayDecks`: the decks read through Accessibility, with track analysis.
    `DjayLibrary`: a read-only copy of djay's library. `DjaySidebar`: djay's
    sidebar read and driven through Accessibility.

Read before changing S8 behaviour: `docs/s8/mapping.md`,
`docs/s8/input-capture.md`, `docs/s8/translator-validation.md` and
`docs/s8/hardware-notes.md`. For the screens: `docs/s8/panel-views.md`,
`docs/s8/deck-pages.md` and `docs/s8/deck-data-validation.md`.

- `mise run s8:raw [seconds]`: read-only log of every control change.
- `mise run s8:midi`: stops NIHardwareAgent, runs the bridge until Ctrl-C,
  restores the agent. The log goes to `docs/s8/captures/translator-<UTC>/`.
- `mise run s8:panel-views`, `s8:deck-data`, `s8:deck-page`: attended checks
  of the panels. Each stops NIHardwareAgent, sends screen frames to the S8,
  asks its rows and restores the agent. `mise tasks` lists the older
  screen checks.
- `mise run s8:panel-views-images [check]`: the expected panel pictures from
  recorded snapshots; sends nothing.
- `cargo run --locked -- --device s8 --generate-mapping <dir>`: writes the
  mapping; it does not overwrite an existing file.
- `cargo test --locked translator`: translator tests. Run the full suite with
  no bridge process running.
- `swift test --package-path macos`: the Swift tests, the screen tools
  included.

### S8 notes

One line per reference note and procedure. Open a note when the task needs
it. The root `INDEX.md` lists every document, validation records included.
`mdsmith fix` writes this list from each note's `summary`; do not edit it.

<?catalog
glob: "docs/s8/**/*.md"
where: 'type: "reference" | "procedure"'
sort: path
header: "\n"
row: "- [`{filename}`]({filename}): {summary}"
footer: "\n"
?>

- [`docs/s8/capture-checklist.md`](docs/s8/capture-checklist.md): Recorder usage and the ordered control-by-control checklist for the agent-stopped input capture.
- [`docs/s8/capture-recorder.md`](docs/s8/capture-recorder.md): Usage and verification status of the read-only S8 input recorder used for the agent-stopped capture.
- [`docs/s8/deck-pages-attended-check.md`](docs/s8/deck-pages-attended-check.md): Preview and stream commands for deck pages, and the plan for the attended check on the S8.
- [`docs/s8/deck-pages.md`](docs/s8/deck-pages.md): Deck page data from Accessibility and the analysis cache, snapshot fields, limits, preview commands and live validation.
- [`docs/s8/display-buttons.md`](docs/s8/display-buttons.md): Display buttons and screen encoders switching djay's sidebar tabs and moving the selection (S8Host).
- [`docs/s8/djay-sidebar-access.md`](docs/s8/djay-sidebar-access.md): Reading and switching djay Pro's sidebar tabs through Accessibility, with a probe and fixtures.
- [`docs/s8/driverkit-evaluation.md`](docs/s8/driverkit-evaluation.md): DriverKit, USB, HID, audio and MIDI driver extensions for a more native S8 setup: verdicts, entitlement cost and driverless polish.
- [`docs/s8/hardware-notes.md`](docs/s8/hardware-notes.md): S8 USB identity, interfaces, input and output evidence, and open questions.
- [`docs/s8/kit.md`](docs/s8/kit.md): The S8 kit: prebuilt bridge, S8Host, launcher and mapping for a second Apple Silicon Mac without the repository; build, transfer, run and limits.
- [`docs/s8/led-output-mixer.md`](docs/s8/led-output-mixer.md): S8 mixer LED output: report 0x82, VU meters, deck selectors, feature reports and Encdr corrections.
- [`docs/s8/led-output-questions.md`](docs/s8/led-output-questions.md): S8 deck LED open questions Q1-18 and Q34-39 with the answers the 2026-11-11 probe run and two djay-driven LED checks gave.
- [`docs/s8/led-output.md`](docs/s8/led-output.md): S8 LED output for decks: report framing, write API, the 0x80 and 0x81 byte map, values and state.
- [`docs/s8/mapping.md`](docs/s8/mapping.md): The default Djay layout generated from the S8 catalog: addresses, pad modes, local and unassigned controls.
- [`docs/s8/mappings/README.md`](docs/s8/mappings/README.md): Frozen copies of Djay mappings that were checked on the S8, with install steps.
- [`docs/s8/panel-views.md`](docs/s8/panel-views.md): The library and deck views on the panels (S8Host --screens): run, expected pictures, state at stop, limits and check plan.
- [`docs/s8/research-sources.md`](docs/s8/research-sources.md): Survey of kontrol-s8-protocol, Encdr and Mixxx, with licensing notes.
- [`docs/s8/s4-coupling-audit.md`](docs/s8/s4-coupling-audit.md): S4 MK3 assumptions in the bridge with file anchors, and the device-profile seam.
- [`docs/s8/screen-transport.md`](docs/s8/screen-transport.md): The app-to-bridge channel for screen frames and status: record layout, status lines and worker policy.
- [`docs/s8/screens.md`](docs/s8/screens.md): Display protocol summary, source conflicts, macOS ownership and the phased first-frame test plan.
- [`docs/s8/upstream-workflow.md`](docs/s8/upstream-workflow.md): Remotes, branches, commit separation and the upstream pull-request procedure.

<?/catalog?>

### Verification

Check a change at the first tier that can show it wrong. Name in the PR the
tiers that ran and what stayed unchecked.

1. Tests, before the PR: decoding, parsing, translation and rendering.
2. Scripted check against the running djay, without the S8, before the PR
   is marked ready: anything read from djay or drawn on a panel page. Drive
   djay with `osascript` or Accessibility, render the page with `S8Preview`
   and put the picture next to a capture of djay's window. Keep the
   pictures under `docs/s8/captures/<name>/` and embed them in the PR.
3. Attended run on the S8, before the issue closes: what is sent to the S8,
   physical controls, panel timing, exit and recovery. The PR may merge
   before the run. Until the run is recorded it says `Refs #N`, and its
   changelog entry says "not yet on the S8". A PR that changes the bytes
   sent to the S8 gets its run before the merge.

- Attended runs are done in batches. Collect the rows that merged PRs still
  need and ask the user for one session. Ask only for the rows the changes
  need, not for a full plan (row subsets: #108).
- This Mac is not used to DJ, and its djay library is a test library. A
  scripted check may load tracks, set and delete cue points, switch the
  deck layout and change the library.
- A scripted check against djay does not make a row checked on the S8.

### Approvals

These rules cover the named actions only. Do not extend them to similar
actions.

No go-ahead needed:

- Creating issues, issue comments, board moves and branches from `main`.
- Driving djay on this Mac for a scripted check, including changes to its
  state and library, and capturing its window.
- On a `type/N-description` branch: committing, pushing, and opening or
  updating its PR. After a rebase, pushing that branch with
  `--force-with-lease`.
- Reading S8 HID input, `mise run s8:raw`, and generating a mapping under a
  new file name.
- Repeating a read-only command or a check that failed.

Get the user's go-ahead first, then do it:

- Merging a PR.
- Closing an issue by hand, an epic or a milestone.
- Sending anything to the S8 (see _S8 hardware_ above).
- Stopping NIHardwareAgent, which `mise run s8:midi` does. Djay then gets no
  MIDI from the Controller Editor setup until the agent is restored.
- Overwriting, renaming or deleting a file in `~/Music/djay/MIDI Mappings/`.
- Rewriting history on `main`, and discarding uncommitted changes you did not
  make.
- Opening an issue or PR on an upstream repository.

### Code ownership

- The S8 code and notes are this fork's own: `src/s8/`, `src/device/s8.rs`,
  `tests/s8_input.rs`, `docs/s8/` and the S8 tasks in `mise.toml`. What they
  are for and how they are built is our call, and we keep building on them.
  Merge repeated code, fix nearby problems and restructure when that makes
  the next change simpler. Name each such change in the PR description.
- Everything else is upstream's code. Change it as little as possible and
  only where the S8 needs it, so that merges from upstream stay simple and
  S4 MK3 behaviour stays the same.

### Working rules from S8 sessions

- The S8 has no jog wheels. Skip jog, platter and motor items that come from
  S4 MK3 documents.
- Write capture logs under `docs/s8/captures/<name>/`, never under `target/`:
  a capture was lost with the build cache. `*.log` is ignored globally, so
  commit a capture as `.log.gz`.
- For an attended check, install the mapping as
  `Kontrol S8 Bridge rN.djayMidiMapping` with the next N. Do not overwrite an
  installed revision; Djay may have it selected. When the user confirms a
  revision, copy it into `docs/s8/mappings/`.
- In a validation record, write only what the user reported in that check. A
  passing unit test does not make a row checked on hardware.
- Before an attended check that draws on the S8 panels, attach pictures of
  what each step should show to the step's GitHub issue, and embed them in
  the PR description. Three to five pictures are enough for a continuous
  step. Make them from the bytes that will be sent, not from a second
  rendering. A file path in backticks does not show a picture: embed it with
  `![...](https://raw.githubusercontent.com/<owner>/<repo>/<commit>/<path>)`.
  For a PR that draws nothing on the panels, say so on the issue.
- Stacked PRs with squash merges: merge the base PR first. Then rebase the
  child with `git rebase --onto origin/main <old base tip>` and push it with
  `--force-with-lease`.
- Before removing a worktree, move its ignored capture logs out. Removing the
  worktree deletes them.
- Keep the branch `pin/5d7a689-deviceid-default` in gootecks/Encdr.
  `Cargo.toml` pins a commit on it by revision, and the branch keeps that
  commit available.
- `apply_patch` in _Workspace discipline_ names upstream's edit tool. Use the
  edit tool of your own harness.
