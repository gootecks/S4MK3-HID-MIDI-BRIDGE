---
summary: "Preview and stream commands for deck pages, and the plan for the attended check on the S8 (#85)."
type: procedure
---

# Deck pages: preview, stream and attended check

How to run the deck page preview and stream, and the plan for the attended check
on the S8 (#85). The page design and limits are in [deck-pages.md](deck-pages.md).

## Run the preview

```sh
swift run --package-path macos S8Preview macos/S8ViewsTests/Fixtures --deck 1
swift run --package-path macos S8Preview macos/S8ViewsTests/Fixtures --deck 1 --no-window --png /tmp/deck
swift run --package-path macos S8Preview macos/S8ViewsTests/Fixtures --deck-snapshot deck.json
```

`--deck N` replaces the right panel with live deck N. It refreshes at 10 Hz.

The key `f` (or `--png`/`--frames`) also writes three files:

- `deck-N.png`
- `deck-N.json`, a `DeckSnapshot` that `--deck-snapshot` can reuse
- `deck-N.s8frame`

On the S8, `S8Host --screens --deck N` shows the page on the right panel
([panel-views.md](panel-views.md)). This is unchecked on hardware.

## Stream to the S8

```sh
swift run --package-path macos S8Preview --stream left=1,right=2 --seconds 20
```

`--stream` writes the live deck pages as `S8FR` records to stdout. The format is in
[screen-transport.md](screen-transport.md). The bridge reads them with
`--s8-screens-stdin`. The command opens no device.

- `left=N` and `right=M` choose the deck for each panel. One panel alone is
  allowed.
- The pages are rendered every 0.1 s. A record is written only when a page
  changed, so a paused deck sends one record.
- Without `--seconds` it runs until its stdout closes.
- Each record is logged on stderr with the deck's time and play state.
- When djay cannot be read, it exits with status 1 before it writes anything.

## Attended check on the S8 (#85)

This section is the plan, not a result. The run sends screen frames to the S8 and
stops NIHardwareAgent, so it needs the user's go-ahead.

It checks the deck pages of the paused-deck fix (#80) on the real panels, next to
djay. It sends full-panel frames to interface 6 through the bridge's
`--s8-screens-stdin` channel. That channel was checked on this S8 on 2026-11-10
([screen-transport-validation.md](screen-transport-validation.md)). It sends no LED
reports, no feature reports and nothing to interface 4.

### Preconditions

- Exactly one Kontrol S8 on USB; Traktor, Maschine and Komplete Kontrol quit.
- djay open with decks 1 and 2 loaded and paused. For the last step, decks 3 and 4
  loaded as well.
- The terminal that runs the check holds Accessibility permission. The runner reads
  djay once before it stops anything and refuses when that fails.
- Run from an interactive terminal: `mise run s8:deck-page`. To continue a run that
  stopped early, name the first step, for example `mise run s8:deck-page b-play`.
- The runner stops `NIHardwareAgent` with `kill -9`. It relaunches it with `open -g`
  on every exit path. While it is stopped, the S8's controls reach djay only
  through the bridge mapping. Start and stop decks in djay's window.

The runner builds the release bridge and `S8Preview`. It writes
`docs/s8/captures/deck-page-<UTC>/` with these files:

- per step, the bridge output (`.txt`) and the stream log (`.stream.txt`)
- `notes.txt` with every answer
- `snapshots.txt` and `events.txt`

### Steps

Every step runs this pipeline:

```sh
S8Preview --stream … --seconds N | bridge --device s8 --midi full --s8-screens-stdin --until-stopped
```

The stream ends after N seconds. The bridge then sends black frames to both panels
and exits.

A step passes when all of these hold:

- The status lines are `pending`, `active`, `inactive`.
- The bridge exits with status 0.
- The observer answers `y` to each question.

| Step           | Panels                    | Time | The observer does                                 |
| -------------- | ------------------------- | ---- | ------------------------------------------------- |
| `a-paused`     | left deck 1, right deck 2 | 20 s | Nothing; both decks stay paused                   |
| `b-play`       | left deck 1, right deck 2 | 60 s | Plays deck 1 for about 10 s, pauses about 10 s    |
| `c-four-decks` | left deck 3, right deck 4 | 60 s | Switches djay to Four Decks, then plays each deck |

What each step must show:

- `a-paused`: each panel shows its deck with title, time, waveform and PAUSED.
  Both times and playheads stand still for the 20 s. Both panels go black at the
  end.
- `b-play`: the observer plays deck 1 about 10 s, pauses about 10 s, plays again
  and pauses. While deck 1 plays, the left panel says PLAYING and its time moves
  with djay's. On pause the time stops without stepping back. While paused it
  stands still. On resume it goes on from there without a jump by the length of
  the pause. The right panel stands still.
- `c-four-decks`: the observer plays deck 3 about 10 s and pauses, then deck 4
  about 10 s and pauses. The panels show decks 3 and 4. Each time moves only while
  its own deck plays and does not step back on pause.

Expected and not a fault (#87): PLAYING appears up to about 1 s after play. Until
then the time stands still. When PLAYING appears, the time moves forward once by
less than 1 s. That is the time played since the press.

These are faults:

- a time that runs on a paused deck
- a time that steps back on pause
- a page of the wrong deck
- a garbled or frozen panel while its deck plays
- a status line `failed`
- a bridge exit status other than 0

On a fault, answer `n` or describe it. The runner skips the remaining steps and
goes to recovery. Typing `stop` at any prompt does the same. So does Ctrl-C during
a step; after Ctrl-C the panels go black.

### Recovery

The runner relaunches NIHardwareAgent and waits 5 s. It asks what the panels show
and whether djay reacts to the left PLAY button. If the agent does not come back,
start it by hand:

```sh
open -g "/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"
```
