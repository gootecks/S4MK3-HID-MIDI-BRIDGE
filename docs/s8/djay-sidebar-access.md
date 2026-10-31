# djay Pro sidebar through Accessibility

Research for #48 (part of #25). Can the bridge read djay Pro's right-hand
sidebar and switch its tabs? Yes: the whole sidebar is in the macOS
Accessibility (AX) tree, every tab and sub-tab answers `AXPress`, and rows are
readable as plain static text. No S8 output was involved. All results are from
djay Pro on this Mac (arm64, one 2560×1440 display) with an empty deck pair,
the Skrillex SOMA test library (13 songs in `Music`) and the sidebar open.

Probe: [`examples/djay-sidebar-probe.swift`](../../examples/djay-sidebar-probe.swift).
Fixtures for the #50 renderer: [`fixtures/sidebar/`](fixtures/sidebar/), one real
snapshot per tab.

## Summary

| Question | Answer |
| --- | --- |
| Reachable through AX? | Yes, via `AXFocusedWindow` only. `AXWindows` is empty and `AXMainWindow` is nil. |
| Works while djay is full screen, in the background, on another Space? | Yes, all three were true during every test. |
| Sidebar open/closed | Detectable: closed removes the tab buttons and the content area from the tree. The toggle button and the View menu item both flip it. |
| Switch tabs | `AXPress` on the tab button. 140 presses over 20 cycles, 0 failures. |
| Info > Related | `AXPress` on the Related button inside the Info content. Works. |
| Read rows | Yes: title, artist, BPM, key, time, date headers, the "Show More" button, per-row `AXSelected`. |
| Which tab is active | **Not exposed** by the buttons. Infer it from the content area's title. |
| Latency | Tab press to new content: median 120 ms, p95 126 ms, max 212 ms. Snapshot read: median 18 ms, p95 36 ms. |
| Does a sidebar row select the library row that load/queue act on? | **No.** Only the main library table does. See [Row selection](#row-selection). |

## Reaching the window

```swift
let app = AXUIElementCreateApplication(pid)          // process name "djay Pro"
AXUIElementCopyAttributeValue(app, "AXWindows")      // []        (empty)
AXUIElementCopyAttributeValue(app, "AXMainWindow")   // nil
AXUIElementCopyAttributeValue(app, "AXFocusedWindow")// AXWindow  (works)
```

The application element's `AXChildren` holds only the menu bar. That is why
System Events (`count of windows`) reports 0 for djay. `AXFocusedWindow`
returns the window (`AXStandardWindow`, id `_NS:8`, frame `0,52 2560×1388`)
even though:

- djay is in full screen (View menu offers "Exit Full Screen"),
- its window is not on the active Space (`CGWindowListCopyWindowInfo` reports
  `onscreen: false`), and
- another app (Tern) was frontmost.

Reads and `AXPress` succeeded under all three conditions, with no focus
change. The probe falls back to `AXMainWindow`, then `AXWindows[0]`.

## AX paths

Anchors are from the test machine. Identifiers are `_NS:n` Interface Builder
ids and are stable between runs, but the probe finds elements by structure
(see [Locating elements](#locating-elements)) so it does not depend on them.

```text
AXApplication "djay Pro"
└─ AXFocusedWindow  AXWindow/AXStandardWindow _NS:8
   ├─ AXGroup "Decks"                      deck controls, not the sidebar
   ├─ AXSplitGroup [61,317 2499×1078]      library and sidebar
   │  ├─ AXSplitGroup …                    left playlists + main track table
   │  │  └─ AXScrollArea > AXTable "Playlist"   main library table, x=311
   │  ├─ AXButton "Library"  _NS:16  [2257,314 62×40]   sidebar tabs
   │  ├─ AXButton "Queue"    _NS:34
   │  ├─ AXButton "Match"    _NS:42
   │  ├─ AXButton "Session"  _NS:49   AXHelp "History"
   │  ├─ AXButton "Info"     _NS:56
   │  └─ AXScrollArea  [2260,352 300×1043]              sidebar content
   ├─ AXButton "Hide library inspector" _NS:69  [2501,1403 50×30]  sidebar toggle
   └─ AXButton "Collapse Library" _NS:26, "Show Automix panel" _NS:59, …
```

### Tabs

Left to right, as the issue described them (note the third icon is **Match**
and the clock icon is called **Session** in AX; its tooltip says History):

| Position | Icon | `AXDescription` | `AXHelp` | Probe id | Content `AXTitle` |
| --- | --- | --- | --- | --- | --- |
| 1 | track | `Library` | `Library` | `library` | `Playlist` |
| 2 | list | `Queue` | `Queue` | `queue` | `Queue` |
| 3 | third | `Match` | `Match` | `match` | `Match` |
| 4 | clock | `Session` | `History` | `session` | `History` |
| 5 | i | `Info` | `Info` | `info` | empty (first static text is `Info`) |

Each is an `AXButton`, role description `button`, enabled, actions
`[AXPress]`, empty `AXTitle` and `AXValue`. There is **no** `AXValue`,
`AXSelected` or `AXFocused` that marks the active tab, so the active tab must
be inferred. The probe reads the content area's `AXTitle` (falling back to its
first static text) and maps `playlist` to `library` and `history` to `session`.

The probe emits `id` from `AXDescription` lowercased (`session`) and `label`
from `AXHelp` (`History`), so the History tab is `{"id":"session","label":"History"}`.
`--press history` is accepted as an alias.

### Info sub-tabs

Inside the Info content area, two `AXButton`s with `AXTitle` `Info`
(`_NS:236`) and `Related` (`_NS:243`) sit at `y=480`, 153 px wide. They also
expose no selection state. The probe infers it from the content: an
`AXOutline` (the metadata form) means Info, an `AXTable` means Related.

### Content per tab

All content is children of the one sidebar `AXScrollArea` (a few Match items
are siblings of it, not children).

| Tab | Children |
| --- | --- |
| Library | `AXButton` description `Source`, title `Music`; `AXTable` "Playlist" |
| Queue | static texts `Queue`, `0 Songs` (becomes `1 Song  •  3m`); `AXTable` "Queue" with a `Drop songs here` placeholder row |
| Match | static text `Match`; `AXPopUpButton` "Active Deck"; `AXPopUpButton` description `Source`; static `Drop song here to match`; empty `AXTable`; **siblings** `AXImage` "icon empty match" and static `Match finds songs that go well with what is currently playing.` |
| Session | static `History`; `AXMenuButton` help `New Session`; `AXTable` "History" |
| Info | static `Info`; `AXPopUpButton` "Selected"; `AXMenuButton`; two `AXImage` (artwork); statics for title, `150 BPM`, key, time; the two sub-tab buttons; then `AXOutline` "Info" or `AXTable` (Related) |

### Rows

Track rows (Library, Queue, Session; the Related table uses the same
layout) are `AXRow`/`AXTableRow`, 300×47, with one `AXCell` holding:

| Child | Role | Example | Notes |
| --- | --- | --- | --- |
| `icon origin local` | `AXImage` | | source badge |
| `default artwork song`, plain image | `AXImage` | | artwork, no pixel data |
| title | `AXStaticText` `_NS:76` | `Anybody` | top line, left |
| artist | `AXStaticText` `_NS:152` | `Skrillex, ISOxo` | second line, left |
| BPM | `AXStaticText` `_NS:217` | `150 BPM` | top line, right; suffix to strip |
| key | `AXStaticText` `_NS:213` | `7A` | second line, right |
| time | `AXStaticText` `_NS:210` | `02:43` | second line, far right |

Static texts carry their text in `AXValue`; `AXTitle` and `AXDescription` are
empty. Fields are told apart by position and pattern (left half = title, then
artist; ` BPM` suffix; `m:ss` pattern; the remainder is the key), not by
identifier, so a missing BPM does not shift the others. Missing fields do not
appear as empty elements in this library.

Other row types:

| Kind | AX shape | Example |
| --- | --- | --- |
| date header (Session) | row, 29 px high, one static text | `10/28/26, 12:40 AM` (the space before `AM` is U+202F) |
| action (Session) | row with one `AXButton` titled `Show More`, actions `[AXPress]` | `Show More` |
| empty state | row with an `AXImage` "icon drop placeholder" and one static text | `Drop songs here`, `Drop song here to link songs` |
| Info field | `AXRow`/`AXOutlineRow` > `AXCell` > label static + value | `TITLE` + `AXTextField` `Anybody` |

Selection is `AXSelected` on the `AXRow` (`1` or `0`). `AXCell`s inherit it.

The Info form shows 22 fields: TITLE, ARTIST, GENRE, COMMENTS, ALBUM, TIME,
BPM, KEY, COLOR, RATING, EXPLICIT, YEAR, LABEL, COMPOSER, GROUPING, ADDED,
RELEASED, ALBUM TRACK #, BIT RATE, SAMPLE RATE. Values are `AXTextField`,
`AXStaticText`, `AXPopUpButton` (KEY), `AXCheckBox` (EXPLICIT) or
`AXLevelIndicator` (RATING). COLOR is eight unlabelled buttons.

### Sidebar toggle and menus

Sidebar button, bottom right of the window:

| State | `AXDescription` | `AXHelp` |
| --- | --- | --- |
| open | `Hide library inspector` | `Show sidebar` |
| closed | `Show library inspector` | `Show sidebar` |

When closed, the tab buttons and the content area are gone from the tree
(only a zero-width `AXSplitter` at the window edge remains), so tab ids cannot
be listed until it is reopened. Reopening restored the previously selected tab
and sub-tab.

Menu route, an alternative press path (menu items answer `AXPress` and do not
need djay frontmost):

| Menu | Item | Shortcut | Notes |
| --- | --- | --- | --- |
| View | `Show Library Sidebar` | K | `AXMenuItemMarkChar` is `✓` when open, so this is a state read that needs no tree walk |
| View | `Expand Library` | ⌘↩ (as listed) | checked in the test state |
| View | `Split Library` | B | |
| View | `Show Library Automix` | | |
| Library | `Search`, `Locate Track`, `Analyze`, `Automix`, `Preview Track` | F, L, A, §… | |
| Library | `Add Selection to Queue` | ↩ | acts on the main library selection |
| Library | `Remove Selection from Queue` | ⌫ | acts on the selected Queue row |
| Library | `Refresh Library` | R | |
| Deck 1 / Deck 2 | `Load Previous Track`, `Load Next Track`, `Load Scratch Tools`, `Load Double` | | no "load selected track" item |

There is **no menu item for the sidebar tabs**, so tab switching needs
`AXPress` on the buttons.

## Locating elements

`findSidebar` in the probe walks `AXSplitGroup`/`AXGroup` containers (depth 4) looking for a parent that has at least
three `AXButton`s with a non-empty description, 40–100 px wide, 25–60 px high,
on one `y`, plus an `AXScrollArea` below them. That finds the tab strip and
its content area without identifiers or the tab count. Median 8 ms (p95 13 ms)
including the AX calls.

## Using the probe

```sh
swift examples/djay-sidebar-probe.swift                      # pretty JSON of the current tab
swift examples/djay-sidebar-probe.swift --json               # one line
swift examples/djay-sidebar-probe.swift --press queue        # press, then snapshot
swift examples/djay-sidebar-probe.swift --press info/related # tab/sub-tab
swift examples/djay-sidebar-probe.swift --press related      # shorthand for info/related
swift examples/djay-sidebar-probe.swift --open               # open the sidebar first
swift examples/djay-sidebar-probe.swift --watch              # print on change, read time on stderr
swift examples/djay-sidebar-probe.swift --bench 20           # latency and reliability
```

`--limit N` caps rows (default 100). Exit codes: 2 djay missing, no
permission or no window; 3 sidebar closed for `--press`; 4 press failed or
unknown tab. Snapshots follow the shared schema 1 (`schema`, `capturedAt`,
`sidebarOpen`, `tabs`, `subtabs`, `heading`, `rows`).

Mapping notes where djay's UI does not fit the schema:

- `heading` is the Info sub-tab label (`Info`, `Related`) when sub-tabs exist,
  else the heading static text, else the source button title (`Music` for
  Library).
- `kind: "header"` carries date headers, empty-state hints (`Drop songs here`)
  and, on the Info sub-tab, one `NAME: value` row per metadata field.
- `kind: "action"` carries `Show More`.
- Info and Related start with one `track` row, `selected: true`, built from the
  summary strip (title, BPM, key, time). `artist` is filled from the ARTIST
  field only on Info; it is `null` on Related because that view omits it.
- A closed sidebar yields `sidebarOpen: false` with empty `tabs` and `rows`.

## Row selection

Experiments (library, Anybody selected in the main table to start):

| Step | Result |
| --- | --- |
| `AXUIElementSetAttributeValue(sidebarRow, "AXSelected", true)` on sidebar row 1 | returns success; that sidebar row shows selected |
| main table right after | unchanged (still Anybody); Info "Selected" still showed Anybody |
| `AXSelectedRows` set on the main table | returns success; main selection moves |
| `Library > Add Selection to Queue` with sidebar row 1 (Cheeni) selected and main row 2 (Diwali) selected | Diwali was queued (the Queue showed `1 Song`, Diwali) |
| select the Queue row through the sidebar, then `Library > Remove Selection from Queue` | the Queue returned to empty |

So a sidebar row's selection state is real and writable, but it does not
retarget the main-library actions that `Add Selection to Queue` and, by the
issue's account, MIDI load use. The main library `AXTable` (`Playlist`, x=311)
is the target, and its rows accept `AXSelected`/`AXSelectedRows`. Sidebar rows
expose no actions (no `AXPress`, no double-click equivalent).

MIDI load itself was not exercised (see [Not exercised](#not-exercised)).

## Reliability and latency

`swift examples/djay-sidebar-probe.swift --bench 20`: per cycle, press every
tab and wait until the content title changes, read a snapshot, then press
Related and Info on the Info tab. djay stayed in the background.

| Measure | n | min | median | p95 | max |
| --- | --- | --- | --- | --- | --- |
| tab press to new content | 100 | 21 ms | 120 ms | 126 ms | 212 ms |
| sub-tab press to new content | 40 | n/a | 131 ms | 137 ms | 187 ms |
| full snapshot read | 100 | n/a | 18 ms | 36 ms | 92 ms |
| locate sidebar | 100 | n/a | 8 ms | 13 ms | n/a |

140 presses, 0 failures, 0 timeouts (3 s messaging timeout). A first run showed 30
"failures" that were probe bugs: it re-pressed stale sub-tab elements after
the first press. Re-find buttons after every press; element references go
stale when the content swaps.

Polling: `--watch` re-reads on a timer and prints only on change; the first
read printed `read 89 ms` (cold), later reads are near the 18 ms median. Each
read is a tree walk, which is why rows should be capped (`--limit`).

## Failure cases

| Case | Observed |
| --- | --- |
| djay not running | probe exits 2 |
| Accessibility not granted | `AXIsProcessTrusted()` false; probe exits 2 |
| `count of windows` via System Events | 0, because `AXWindows` is empty; use AX directly |
| full screen | no effect; this is the state djay was in |
| window on another Space | no effect on reads or presses (`onscreen: false`) |
| djay in background | no effect |
| sidebar closed | tabs and content disappear; snapshot says `sidebarOpen: false`; `--open` restores the old tab |
| active tab unknown | cannot be read from the buttons; infer from content title, so an unrecognised content title returns no selected tab |
| stale element after a press | a second press with a kept reference fails (`AXPress` error); re-find |
| Related | empty here (`Drop song here to link songs`); linking needs drag and drop, so populated Related rows were not captured |
| long lists | only 13 songs available, so row virtualisation is untested; `Show More` in History is a button, not auto-loaded rows |
| deck playing | not tested (see below) |

## Not exercised

- A playing deck. Both decks were empty, and no track was loaded because no
  menu item loads a selected track and nothing was started. [INFERENCE] AX
  reads should be unaffected; kyleawayan/djay-pro-bridge reads the Decks group
  at about 8 Hz during playback, but the sidebar under load was not measured.
- MIDI load and play. The only action observed on the selection is `Add
  Selection to Queue`.
- Pressing `Show More`, the Info `Selected` pop-up, and the Related link flow.
- Libraries large enough to scroll the sidebar tables.

## Side effects and residue

Everything changed was restored: sidebar tab and sub-tab (Info > Related, as
found), main selection (Anybody), sidebar selection (none), Queue (the test
song was added and removed; `0 Songs`), sidebar open. Presses are UI state
only; nothing was written to djay's library.

## Prior art

[kyleawayan/djay-pro-bridge](https://github.com/kyleawayan/djay-pro-bridge)
(MIT) reads deck state (title, artist, key, BPM, time, play state) from the
same AX tree with a background poll at about 8 Hz, and documents the
`Property, Deck N` labelling. It does not cover the sidebar. No code was
copied; the probe is written from the AX dumps in this note.

## Recommendation

For #50 (renderer): consume the schema-1 snapshots in
[`fixtures/sidebar/`](fixtures/sidebar/) (`library`, `queue`, `match`,
`session`, `info`, `info-related`). Do not hard-code five tabs; use the
`tabs` array. Render `header` rows as section labels, including the Info
field rows. Expect `Drop …` hint rows when a list is empty.

For #51 (display buttons): use `AXPress` on the tab buttons.

- Find the strip structurally on each press (median 8 ms) and press the
  element from that lookup; do not cache element references.
- Confirm the switch by polling the content title (about 120 ms), then read
  rows once. Expect about 150 ms from button to repaint, not counting USB.
- Poll at 2–4 Hz only for changes (for example the main selection moving), and
  skip polling when the S8 pages are not showing the sidebar. For sidebar-open
  detection, the View > Show Library Sidebar check mark is cheapest.
- If the sidebar is closed, either show a "sidebar closed" panel or press the
  toggle (`AXHelp` `Show sidebar`); tab ids are only available when open.
- Run AX on a worker thread: calls are synchronous IPC and set
  `AXUIElementSetMessagingTimeout` (the probe uses 3 s) so a hung djay cannot
  stall the HID loop.
- Treat Info > Related as a second press after Info. The first press leaves
  the last used sub-tab selected.
- Selecting a sidebar row will not change what MIDI load or queue acts on.
  If a panel needs "choose a row, then load" behaviour, drive the **main
  library table** (`AXSelectedRows`), which also moves the Info view, or keep
  a bridge-side cursor and issue main-table selection when the user confirms.
  Check this against MIDI load itself (not tested here) before relying on it.
- A bridge process needs Accessibility permission (System Settings > Privacy
  & Security > Accessibility). It is a separate grant from the Terminal one
  used for this research.

Library data beyond what the visible sidebar shows (full history, unplayed
tracks) still comes from the read-only `MediaLibrary.db` snapshot path, not
AX.
