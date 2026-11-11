# One Deck view: what djay exposes through Accessibility

Read-only dumps of djay Pro's Accessibility tree, taken on 2026-11-10 for
[#93](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/93). Nothing
was sent to the S8. The view was switched through djay's **View** menu. Where
a step names the user, the user did it in djay with the mouse.

Each `.tsv` file has one line per element: depth, role, description, title,
value and identifier. Library tables and the menu bar are left out.
`axdump.swift` is the tool that wrote them (`swiftc axdump.swift -o axdump`,
then `./axdump`); it only reads.

| File                                              | View       | Step before the dump                                                                  | Decks in the labels | Titles                                     |
| ------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------- | ------------------- | ------------------------------------------ |
| `01-two-decks`                                    | Two Decks  | Start. Tracks on decks 1 to 4                                                         | 1, 2                | 1 Anybody, 2 Scut 2                        |
| `02-one-deck-from-two-decks`                      | One Deck   | View switched                                                                         | 1                   | 1 Anybody                                  |
| `03-four-decks`                                   | Four Decks | View switched                                                                         | 1, 2, 3, 4          | 1 Anybody, 2 Scut 2, 3 É o Bonde, 4 Scut 2 |
| `04-one-deck-from-four-decks`                     | One Deck   | View switched                                                                         | 1                   | 1 Anybody                                  |
| `05-two-decks-again`                              | Two Decks  | View switched                                                                         | 1, 2                | 1 Anybody, 2 Scut 2                        |
| `06-one-deck-after-deck-2-menu-action`            | One Deck   | **Deck 2 > Quantize** switched off and on again from the menu bar, then view switched | 1                   | 1 Anybody                                  |
| `07-two-decks-new-track-on-deck-2`                | Two Decks  | User loaded a new track on deck 2                                                     | 1, 2                | 1 Anybody, 2 Cheeni                        |
| `08-one-deck-after-load-on-deck-2`                | One Deck   | View switched                                                                         | 1                   | 1 Anybody                                  |
| `09-one-deck-after-drag-load`                     | One Deck   | User dragged a track onto the deck shown                                              | 1                   | 1 Duro                                     |
| `10-two-decks-after-one-deck-load`                | Two Decks  | View switched                                                                         | 1, 2                | 1 Duro, 2 Cheeni                           |
| `11-two-decks-after-cue-click-deck-2`             | Two Decks  | User clicked a cue point on deck 2, deck paused                                       | 1, 2                | 1 Duro, 2 Cheeni                           |
| `12-one-deck-after-cue-click-deck-2`              | One Deck   | View switched                                                                         | **2**               | 2 Cheeni                                   |
| `13-two-decks-after-axpress-jump-to-start-deck-1` | Two Decks  | Script pressed **Jump to start point, Deck 1** (deck 1 paused at 0:00)                | 1, 2                | 1 Duro, 2 Cheeni                           |
| `14-one-deck-after-axpress-on-deck-1`             | One Deck   | View switched                                                                         | **1**               | 1 Duro                                     |

## What the dumps show

- One Deck view exposes the labels of one deck only. The other decks have no
  element at all, not even an empty one, and the menu bar keeps only that
  deck's menu. The hidden decks keep their tracks: they are back after the
  switch to Two Decks (`05`, `10`).
- Two Decks view exposes decks 1 and 2 only. Decks 3 and 4 keep their tracks
  while hidden (`01`, `03`, `05`).
- Which deck One Deck view shows is not settled. In these dumps a scripted
  cue click on deck 2 was followed by deck 2 (`12`) and a scripted press on
  a deck 1 control by deck 1 again (`14`); loading a track on deck 2 did not
  change it (`08`), and neither did the deck's menu in the menu bar (`06`).
  In a later check the observer set cue points on deck 2 with the mouse and
  loaded a track there, and One Deck view showed deck 1
  (`../djay-check-20261111T003350Z/README.md`).
- A track loaded while One Deck view is open goes to the deck shown (`09`,
  `10`).
- In One Deck view the deck has no tempo-change label (`0.0%, Deck N`) and no
  `Line volume` label. The title, artist, key, tempo, remaining time and
  play button labels are the same as in Two Decks.
- djay's toolbar button has the description and the value of the view's
  name (`One Deck`, `Two Decks`, `Four Decks`).
- The play button's value changed between `Active` and empty on paused decks
  from dump to dump (`07`, `11`, `13`).

## Not checked

- Whether the deck shown changes while One Deck view stays open, for example
  after a MIDI control for the other deck.
- Which deck One Deck view shows right after djay starts.
- A deck that is shown but has no track.
