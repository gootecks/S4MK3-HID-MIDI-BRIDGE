---
summary: "The attended run of the panel views on the S8."
type: validation
issues: [27, 89]
---

# Views on the S8 panels: validation

Plan, expected pictures and limits: [panel-views.md](panel-views.md). The
tests and the measurement without the S8 are listed there under _Validation
state_. This page holds the one attended run on the S8.

## Observed on the S8: attended run, 2026-11-10

`mise run s8:panel-views` (`panel-views-check.sh`) at commit `3065cd7`.
The user ran it at the S8 with djay Pro and Controller Editor open. The
sidebar was on Library and a track was loaded on deck 1. The runner stopped
NIHardwareAgent. It ran `S8Host --screens` (21:09 to 21:20 UTC) and
`S8Host --screens --deck 1 --log-deck-polls` (21:20 to 21:23 UTC). It asked
one question per row and relaunched the agent. S8Host was the debug build,
the bridge the release build.

Sent to the S8: full-panel screen frames and the black exit frames through
the bridge's `--s8-screens-stdin` transport. Nothing else.

Record:
[captures/panel-views-20261110T210831Z](captures/panel-views-20261110T210831Z/).
`notes.txt` holds the answers and the runner's notes, `library.log.gz` and
`deck.log.gz` the output of S8Host and the bridge with a time stamp per
line, `snapshots.txt` the agent and USB state at each stage. S8Host does not
print the `S8_CONTROL` lines it handles.

The "Reported" column is the user's answer, unchanged. Before the stop the
panels showed "Controller Editor labels".

| Row | Action                                                                                                                    | Expected                                    | Reported                                |
| --- | ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | --------------------------------------- |
| 1   | Start (`--screens`): both panels as `01-library`?                                                                         | list without highlight, "No track selected" | "y"                                     |
| 2   | Left encoder 1 until a track is selected, then two rows on: do highlight and right panel follow each row?                 | yes, as `02-library-selected`               | "y"                                     |
| 3   | Keep turning past the last visible row: does the list scroll and keep the highlight visible?                              | yes, as `03-library-scrolled`               | "y"                                     |
| 4   | Is "n of m" the position of the row among the sidebar's tracks?                                                           | yes                                         | "y"                                     |
| 5   | Left button 3: Queue?                                                                                                     | as `04-queue` or the current queue          | "y"                                     |
| 6   | Left button 5: Match?                                                                                                     | as `05-match`                               | "y"                                     |
| 7   | Left button 6: History?                                                                                                   | as `06-history` with the current sessions   | "y"                                     |
| 8   | Left button 7: Info, with Info and Related on right buttons 2 and 3?                                                      | as `07-info`                                | "y"                                     |
| 9   | Right button 3: Related? Right button 2: Info again?                                                                      | as `08-related`, then Info                  | "y"                                     |
| 10  | Left button 2: Library again?                                                                                             | yes                                         | "y"                                     |
| 11  | Select another sidebar row with the mouse: how long until the panels follow?                                              | about 1 s                                   | "sometimes instant, sometimes about 1s" |
| 12  | Close djay's sidebar: "Open the djay sidebar"? Open it: does the list return?                                             | as `09-sidebar-closed`, then the list       | "y"                                     |
| 13  | Encoder turn to panel change: instant, a short lag, or seconds?                                                           | –                                           | "instant"                               |
| 14  | Spin an encoder fast through a long list: do the panels end on the row djay selected?                                     | yes                                         | "y"                                     |
| 15  | Left PLAY: did djay react while the screens run?                                                                          | yes                                         | "y"                                     |
| 16  | Any flicker, tearing, wrong colours or a frozen panel?                                                                    | no                                          | Long answer, quoted below the table     |
| 17  | After SIGINT to S8Host, agent still stopped: what do the panels show?                                                     | both black                                  | "both black"                            |
| 18  | Start (`--screens --deck 1`): deck 1 page on the right with title, time, BPM, key and waveform, library list on the left? | yes, layout as `10-deck`                    | "y"                                     |
| 19  | PLAY on deck 1: PLAYING with moving time and playhead? Again: PAUSED? Then left playing                                   | yes                                         | "y"                                     |
| 20  | Left screen encoder through at least twenty rows: does the left panel follow while the deck page keeps moving?            | yes                                         | "y"                                     |
| 21  | Left buttons 3, 6 and 2: Queue, History and Library in turn while the deck page keeps moving?                             | yes                                         | "y"                                     |
| 22  | After S8Host's stdin closed: what do the panels show?                                                                     | both black                                  | "both black"                            |
| 23  | After the agent was relaunched: what do the panels show?                                                                  | Controller Editor labels                    | "Controller Editor labels"              |
| 24  | Left PLAY: did djay react?                                                                                                | yes                                         | "y"                                     |
| 25  | Display button 1 or 4: does the NI action work again?                                                                     | yes                                         | "y"                                     |

Row 16, reported in full:

> "scrolling fast with a display encoder is smooth in djay because you see
> each row being passed through, but on the panel, it's choppy and skips to
> wherever the encoder ends. it still ends on the same row as djay, it's
> just choppy if you go fast. but if you go slow or medium, it's smooth"

Noted by the runner:

- Both runs: S8Host exit status 0, status lines `S8_SCREEN_STATUS pending`,
  `active`, `inactive`, no bridge process left.
- The logs hold no warning or error line.

### Deck reads during the second run (#89)

From the `deck poll` lines of `deck.log.gz`, summed by the runner when each
row was answered. A row's numbers cover the time from the previous answer to
its own, so row 18 includes the start.

| Until the answer to row | Deck reads | Longest gap | Gaps over 0.45 s | Longest read |
| ----------------------- | ---------- | ----------- | ---------------- | ------------ |
| 18 (start)              | 150        | 0.544 s     | 1                | 0.544 s      |
| 19 (play and pause)     | 84         | 0.253 s     | 0                | 0.089 s      |
| 20 (scrolling)          | 310        | 0.252 s     | 0                | 0.093 s      |
| 21 (tab switching)      | 106        | 0.251 s     | 0                | 0.104 s      |

Over the whole run: 651 reads, mean gap 0.250 s. The one gap over 0.45 s is
the first of the run: the first read took 0.544 s and the second started
when it ended.

## Limits of this run

- Row 16: with a fast encoder turn the panel jumps to the row where the turn
  ends instead of passing through the rows as djay does. That is how the
  session draws ("once no further event is waiting",
  [panel-views.md](panel-views.md#updates)); the user finds it choppy. Slow
  and medium turns were reported as smooth. Followed up in #90.
- Row 16 asked about flicker, tearing, wrong colours and a frozen panel. The
  answer names none of them and does not say "no" to them either.
- Rows 20 and 21 were answered "y" as a whole. How many rows were scrolled
  and how long each tab stayed open is not recorded, since S8Host does not
  log the control events it handles.
- Rows 2 to 12 compare the panels with pictures of the 13-track test
  library; the user's library differs, so "y" covers the layout and the
  behaviour, not identical text.
- The deck read gaps were measured on deck 1 only, with the default sidebar
  poll of 1 s and whatever list the sidebar showed. Lists near the 300-row
  limit, `--poll` under 1 s and decks 2 to 4 were not tried.
- No fault was provoked: a bridge that stops reading, a killed bridge, djay
  quitting, the S8 unplugged and the SIGINT and SIGTERM escalation were not
  exercised on the S8. The blocked-write path is covered by the tests and
  the measurement without the S8 only.
- One run, one S8, two-deck djay layout.
