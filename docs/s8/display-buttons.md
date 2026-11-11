---
summary: "Display buttons and screen encoders switching djay's sidebar tabs and moving the selection (S8Host)."
type: reference
issues: [51]
---

# Display buttons and screen encoders

The eight buttons and four encoders beside each S8 screen drive djay Pro's
right-hand sidebar. Buttons 1 and 4 stay with Native Instruments (colour
settings and template switching) and are never used.

This is a source-build feature of the fork: `S8Host` runs the bridge and
applies the controls through Accessibility. The buttons and encoders were
checked once on the S8; see
[display-buttons-validation.md](display-buttons-validation.md).

## Run

1. Stop NIHardwareAgent and follow the setup in
   [translator-validation.md](translator-validation.md) (S8Host does not stop
   or start it).
2. Allow the terminal (or app) that starts S8Host in System Settings >
   Privacy & Security > Accessibility. S8Host checks this at start.
3. Open djay Pro with its library sidebar showing.
4. `cargo build --release`, then
   `swift run --package-path macos S8Host --bridge target/release/s4mk3-hid-midi-bridge`.

S8Host starts the bridge with
`--device s8 --midi full --until-stopped --app-events`, passes its other
output through, and stops it on Ctrl-C, SIGTERM, or when its own stdin closes.
With `--screens` it also draws the sidebar on the S8 panels
([panel-views.md](panel-views.md)); without the flag nothing is sent to the
S8.

## Buttons

Left screen, sidebar tab (the tab is pressed on button down; button up is
ignored):

| Button | Tab     |
| ------ | ------- |
| 2      | Library |
| 3      | Queue   |
| 5      | Match   |
| 6      | History |
| 7      | Info    |
| 8      | none    |

Right screen, Info sub-tabs (only while the Info tab is showing them):

| Button | Sub-tab |
| ------ | ------- |
| 2      | Info    |
| 3      | Related |
| 5-8    | none    |

The assignment is `ButtonSlots.tabIndex(forSlot:)` from `S8Views`, the same
one the screen renderer uses for its labels.

## Screen encoders

Any of the eight encoders moves the selection in the visible list. Turning
clockwise moves down. A step is 50 counts (1000 counts per turn, so 20 rows
per turn); the remainder is kept between reports and dropped when the
direction or the encoder changes. With nothing selected, the first step down
selects the first row and the first step up the last row. Header rows are
skipped and the selection stops at the ends.

The selection is set on djay's sidebar table through Accessibility
(`AXSelectedRows`; [finding](djay-sidebar-access.md#sidebar-selection-by-axselectedrows-51)).
The list highlights and the Info panel follows. Whether load and queue act
on that row, as for a click, is unverified.

## Bridge output

With `--app-events` the bridge prints one line per event, at column 0:

```text
S8_CONTROL display_button <left|right> <2|3|5|6|7|8> <down|up>
S8_CONTROL screen_encoder <left|right> <1-4> <signed counts>
```

Without the flag, output is unchanged. The flag is S8 only and needs `--midi`.
The first report after start only records the resting state and emits
nothing. Other stdout lines (logging, status) are not parsed. When a line
cannot be written because the reader of stdout is gone, the bridge stops as
on Ctrl-C.

## Targets

- `S8Control`: line parser, button-to-action mapping, encoder accumulator.
  No Accessibility, so it is unit-tested directly. It is separate from
  `S8Views` because that target is the renderer and navigator on fixtures.
- `DjaySidebar`: Accessibility reading into `SidebarSnapshot` and the actions,
  behind a small protocol and tested against synthetic element trees.
- `S8Host`: the executable ([README](../../macos/S8Host/README.md)).
