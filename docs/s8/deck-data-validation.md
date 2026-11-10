# Deck data on the S8 panels: validation (#45)

What the deck pages are built from, and their limits:
[deck-pages.md](deck-pages.md). This page holds the attended check of the
live djay deck data on the S8.

## State before this check

Issue #45 asks for a snapshot source that reads the decks from djay, with
two acceptance points. Both were met by PR #77: `swift test --package-path
macos` covers snapshot parsing, and the deck page renders from live djay
data in the preview window.

On the S8 so far ([deck-pages.md](deck-pages.md#deck-pages-on-the-s8-85),
[panel-views-validation.md](panel-views-validation.md)):

- Decks 1 and 2 paused, deck 1 played and paused, decks 3 and 4 in Four
  Decks played and paused: the page of the right deck, a time that moves
  only while the deck plays.
- `S8Host --screens --deck 1` with the library list on the other panel.

Not yet looked at on the S8, and the subject of this check: whether the
content matches djay (title, artist, BPM, key, remaining time, cue flags,
playhead), two decks playing at once, a jump inside a track, a tempo
change, a loop, a track change, a deck that is not in djay's layout, a
layout change while the stream runs, and `S8Host --deck 2`.

## Attended check

This section is the plan. Every run needs explicit approval because it
stops NIHardwareAgent and sends screen frames to the S8.

`mise run s8:deck-data` (`deck-data-check.sh`), at the S8 with djay Pro and
Controller Editor open, djay in its Two Decks layout, a track with cue
points loaded and paused on decks 1 and 2, and the sidebar on Library.

What is sent: full-panel frames to interface 6, at most 30 per second per
panel, through the bridge's `--s8-screens-stdin` transport checked on the
S8 in #44 ([screen-transport-validation.md](screen-transport-validation.md)),
and the black exit frames. Nothing else is written: no LED reports, no
feature reports, no control transfers, nothing on interface 4. djay is only
read, through Accessibility and a copy of its library.

The runner reads djay once before it stops anything and refuses when that
fails. It then stops the agent with SIGKILL, runs three parts, asks one
question per row and relaunches the agent on every exit path. Unlike
`s8:deck-page` the stream is not timed: each part runs until its rows are
answered. Answering `stop` to any row goes straight to recovery. Stop on any
of these: a panel that stays frozen or garbled, an S8 that goes dark, a
status line `failed`.

| Part | Command | Panels |
| --- | --- | --- |
| A | `S8Preview --stream left=1,right=2` piped into `bridge --device s8 --midi full --s8-screens-stdin --until-stopped` | left deck 1, right deck 2 |
| B | the same with `left=3,right=4` | left deck 3, right deck 4 |
| C | `S8Host --screens --deck 2 --log-deck-polls` | left library list, right deck 2 |

Parts A and B end when the runner sends SIGTERM to S8Preview: the bridge's
stdin closes, it sends the black exit frames and exits. Part C ends when the
runner closes S8Host's stdin. Start, stop and load decks in djay's window.

### Expected pictures

Decoded from the records `S8Preview --stream … --seconds 2` wrote on
2026-11-10, read from djay as it was then, with nothing sent to the S8:
`s8-screen-png` reads the records with the bridge's stdin parser and decodes
the frame the bridge would transfer. The tracks in the run will differ, so
the pictures show the layout and the kind of content, not the text.

| State | Left panel | Right panel |
| --- | --- | --- |
| `01-decks-1-2`: part A at its start, both decks paused | ![01 left](images/deck-data/01-decks-1-2-left.png) | ![01 right](images/deck-data/01-decks-1-2-right.png) |
| `02-decks-3-4-two-deck-layout`: part B while djay shows Two Decks | ![02 left](images/deck-data/02-decks-3-4-two-deck-layout-left.png) | ![02 right](images/deck-data/02-decks-3-4-two-deck-layout-right.png) |
| `10-deck` of [panel-views.md](panel-views.md): part C, with deck 2 in place of deck 1 and a synthetic track | ![10 left](images/panel-views/10-deck-left.png) | ![10 right](images/panel-views/10-deck-right.png) |

### Rows

After each of rows 1 to 14 and 16 to 19 the runner notes how many records
S8Preview wrote for each panel since the previous row, with the deck, time,
play state and title of the last one.

| Row | Action | Expected |
| --- | --- | --- |
| 1 | Part A starts | Layout as `01-decks-1-2`; title, artist, BPM and key of each panel as djay shows them for that deck |
| 2 | Compare REMAINING with djay | The same on both decks |
| 3 | Compare the cue flags and the playhead with djay | Same number of flags, same colours and order as the deck's cue points; playhead at the same place as in djay's overview waveform |
| 4 | Play both decks about 15 s, pause both | Both panels say PLAYING, both times move with djay's, no panel stalls |
| 5 | Deck 1 paused: move to another position | Time and playhead on the left panel follow within about 1 s |
| 6 | Deck 1 playing: jump again, then pause | The left panel follows the jump and goes on with djay's time |
| 7 | Deck 2 tempo to about +8 % | BPM on the right panel changes to djay's BPM |
| 8 | Play deck 2 about 20 s at that tempo, pause, reset tempo | REMAINING stays the same as djay's |
| 9 | Deck 1: 4-beat loop for about 10 s while playing | Reported; looping is not modelled |
| 10 | Load another track on deck 1 | Left panel changes to its title, artist, BPM, key, waveform and cue flags, playhead at the start; delay reported |
| 11 | Optional: set a new cue point on deck 2 | No new flag: the cue points are read when the track is loaded |
| 12 | Optional: load a track without analysis, or with a title in a non-Latin script | Reported |
| 13 | Optional: a djay view that hides the decks or their times, about 10 s, and back | Reported |
| 14 | Part A as a whole | No flicker, tearing, wrong colours or frozen panel |
| 15 | Runner ends the stream | Both panels black, bridge exit status 0 |
| 16 | Part B starts, djay in Two Decks | "No track on deck 3" and "No track on deck 4", as `02-decks-3-4-two-deck-layout` |
| 17 | Switch djay to Four Decks, load decks 3 and 4 | The panels fill with decks 3 and 4, content as in djay; delay reported |
| 18 | Play decks 1 and 3 together about 10 s, pause both | Left panel moves with deck 3; right panel (deck 4) stands still |
| 19 | Switch djay back to Two Decks | Both panels return to "No track on deck N" |
| 20 | Runner ends the stream | Both panels black, bridge exit status 0 |
| 21 | Part C starts | Right: deck 2 page, layout as `10-deck`; left: library list |
| 22 | Play deck 2; turn a left screen encoder through some rows; pause | PLAYING with moving time and playhead; the left panel follows while the deck page keeps moving |
| 23 | Runner closes S8Host's stdin | Both panels black, exit status 0 |
| 24 | Agent relaunched | Controller Editor labels on the panels |
| 25 | Left PLAY | djay reacts through the Controller Editor setup |
| 26 | Display button 1 or 4 | NI action works again |

Rows 9, 12 and 13 have no pass criterion: they record what the page does in
a case the code does not handle or was not tried with. Row 11 states what
the code does; a new flag would be a surprise, not a fault.

The record goes to `captures/deck-data-<UTC>/`: `notes.txt` with the answers
and the runner's notes, per part the stream log and the bridge output
(`decks-1-2.stream.log`, `decks-1-2.bridge.log`, the same for `decks-3-4`,
and `host-deck-2.log`) with a time stamp per line, gzipped before a commit,
`snapshots.txt` and `events.txt`.

## Checked without the S8

On 2026-11-10, with a script in place of the bridge that reads and discards
its stdin, so nothing was sent to the S8: the runner's stream functions
started `S8Preview --stream` for both parts against the running djay Pro,
the per-row record counts appeared in the notes, and SIGTERM to S8Preview
closed the stand-in's stdin (522,264 bytes read for the two records of part
A) and let it exit with status 0.
