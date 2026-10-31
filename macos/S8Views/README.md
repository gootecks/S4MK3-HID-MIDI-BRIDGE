# S8Views

Renders djay Pro's right-hand sidebar onto the two Kontrol S8 panels
(480×272 each) and encodes the result as S8 screen frames. Refs #50.
Nothing here talks to the S8 or to djay: it consumes sidebar snapshot JSON
(schema 1) and produces bitmaps and frame buffers.

## Pieces

| File | Role |
| ---- | ---- |
| `SidebarSnapshot.swift` | Decodes snapshot JSON (schema 1). Unknown row kinds become `.unknown`; `bpm` may be a string or a number. |
| `Layout.swift` | Panel metrics, display-button slot map, `ViewState` (selection and scroll). |
| `SidebarNavigator.swift` | `FixtureLibrary` (file or directory of snapshots) and `SidebarNavigator` (button presses and encoder moves swap snapshots and selection). |
| `PanelRenderer.swift` | Draws a panel with CoreGraphics and CoreText (no AppKit, no `ImageRenderer`; macOS 12). |
| `Bitmap.swift` | RGBA bitmap, PNG writer, `S8Frame` (RGB565 big-endian frame builder) and the drawing canvas. |

## Panel content

- **Left panel: navigation.** Tab strip, heading, scrolling row list with
  the selected row highlighted. Header rows are amber labels on a raised bar,
  action rows (for example "Show More") are outlined buttons, track rows show
  title and artist with BPM, key and time columns. A thin scrollbar appears
  when rows overflow.
- **Right panel: detail.** The djay main-library selection at a glance:
  title (two lines), artist and large BPM, key and time tiles, plus "n of m".
  Its strip shows the active tab's sub-tabs (for Info, "Related").

## Selection model

#48 found that MIDI load and queue act on djay's **main library table**
selection, not on sidebar rows, and that Info and Related follow that main
selection. The screen encoder therefore means "move djay's main library
selection": `SidebarNavigator.move(by:)` steps the main selection (the
fixtures' library tracks stand in for the main table), the right panel always
shows it, and the Library tab's highlight follows it. Other tabs keep their
rows unselected. The live bridge should apply the move through AX
(`AXSelectedRows` on the main table) rather than clicking sidebar rows.

## Display-button mapping

Each panel's strip has eight slots, one per display button above the panel.
Slots 1 and 4 belong to NI (colour settings, Controller Editor template
switch) and are drawn as dim "NI". Tabs fill slots 2, 3, 5, 6, 7, 8 in the
order djay exposes them; tabs beyond six are not shown. Left buttons switch
the sidebar tab; right buttons switch the sub-tab (pressing the active
sub-tab returns to the plain tab). The selected tab or sub-tab is filled
blue. The slot number is printed above each label.

## Frame format

`S8Frame.make(screen:bitmap:)` returns 261,148 bytes, as validated in
[`docs/s8/screen-frame-validation.md`](../../docs/s8/screen-frame-validation.md):

1. 16-byte header `84 00 SS 60 00 00 00 00 00 00 00 00 01 E0 01 10`
2. LITERAL command `00 00 FF 00`
3. 480×272 pixels, row-major from the top-left, RGB565 big-endian (261,120 bytes)
4. 8-byte footer `03 00 00 00 40 00 SS 00`

`SS` is `00` for the left panel and `01` for the right. Colours are truncated
to 5/6/5 bits (`r & F8`, `g & FC`, `b >> 3`).

## S8Preview

```sh
swift run --package-path macos S8Preview macos/S8ViewsTests/Fixtures
swift run --package-path macos S8Preview snapshot.json --zoom 2
swift run --package-path macos S8Preview DIR --no-window --png out/png --frames out/frames
```

An AppKit window shows both panels side by side at 1x (`--zoom 2` or `z` for
2x). The argument is one snapshot file or a directory of `*.json` files (the
first file sorted by name is shown first). Keys:

| Key | Stands in for |
| --- | ------------- |
| `1`–`8` | Left display buttons 1–8 (switch tab; loads the fixture whose selected tab matches) |
| `q w e r t y u i` | Right display buttons 1–8 (switch sub-tab) |
| Up / Down | Screen encoder (move selection, skipping header rows) |
| `z` | Toggle 1x / 2x |
| `f` | Write the current state to the `--frames` / `--png` directories as `current-left` and `current-right` |

`--frames DIR` writes `NN-left.s8frame` and `NN-right.s8frame` and `--png DIR`
writes matching PNGs, for every snapshot in the library at startup.
`--no-window` exits after writing.

## Fixtures

`macos/S8ViewsTests/Fixtures/` is a copy of the real snapshots #48 captured
from djay (`docs/s8/fixtures/sidebar/`): `library`, `queue`, `match`,
`session` (labelled History), `info` and `info-related`, using the 13-track
SOMA library. Tab ids are `library`, `queue`, `match`, `session`, `info`;
Info's sub-tabs are `info` and `related`. The navigator picks a tab's
snapshot by its selected tab and sub-tab ids and lands on the Info sub-tab
that is first in the strip. Info shows its fields as header rows
(`TITLE: …`); an empty queue, match or Related shows a `Drop …` hint row.

## Tests

```sh
swift test --package-path macos --filter S8ViewsTests
```
