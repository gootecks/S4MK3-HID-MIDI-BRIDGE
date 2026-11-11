# Deck pages (#45)

A 480×272 page for one djay deck: title, artist, BPM, key, elapsed and
remaining time, play state, the overview waveform with a playhead, bar and
beat ticks, and cue markers. Nothing here is sent to the S8: the page renders
to a bitmap, a PNG or an `.s8frame` file through `S8Preview`.

## Data sources

| Data | Source | Notes |
| ---- | ------ | ----- |
| Title, artist, BPM, key, elapsed, remaining, playing | djay window through Accessibility (`DjayDecks`, polled by `DeckMonitor`) | Needs Accessibility permission for the process. One poll walks the window once and splits the labels by deck number, so the cost does not grow with the number of decks (about 0.1 s measured with four decks). Title, artist, key and BPM are read from English and French labels (`Deck n` and `Platine n`); the time and play labels are matched in English only. |
| Play state | `PlayStateDebouncer` (ported, extended) | The play button of a paused deck blinks at 1 Hz, lit for 0.5 s and unlit for 0.5 s; a playing deck stays lit. Playing is reported after the button has been lit for 0.7 s, counted only over samples at most 0.45 s apart, because the unlit half of a blink fits in a longer gap. It is reported at once when the time label moves one second in the play direction while the button is lit. Paused is reported on the first unlit sample. |
| Time between AX updates | `TimeInterpolator` (rewritten for #83) | djay cuts the fraction off a time label, so a label names a whole second and not a moment. The page time always stays inside the second of the label last read. A paused label rests at the middle of its second. A remaining label equal to the track length cut to a whole second rests at the track start, so elapsed is 0. While the debounced state is playing the time counts on, scaled by the pitch percentage, and waits at the end of the label's second until the label moves. A label that moves one second in the play direction marks the boundary between the two seconds. Any other label change is taken as a jump and rests at the middle of the new second. On pause the counted time stays. When playing is confirmed, the time since the first lit sample is added. |
| Duration, cue points | Read-only `DjayLibrary` snapshot (copy of `MediaLibrary.db` with `-wal` and `-shm`) | The track is matched by title and artist; the track length read from the deck labels picks among entries with one title (see Limits). Used to derive elapsed when only remaining is shown. |
| Overview waveform, 3-band samples, beats, bars | `~/Library/Group Containers/VJXTL73S8G.com.algoriddim.userdata/Library/Application Support/Algoriddim/Metadata/**/<trackID>.djayMetadata` | Binary plist. Amplitude: zlib bytes at `lowRateWaveFinalSampleRate`. Bands: zlib, bfloat16 triples at `lowRateSampleRate`, normalised by the global peak. Beats: zlib big-endian float32 seconds. Bars: every 4th beat counting from `firstDownBeatIndex` (see Limits for gaps). |

djay's live database and cache are only read; the snapshot is a temporary copy.

## `DeckSnapshot` (schema 1, `S8Views`)

`schema`, `deck`, `capturedAt`, `title`, `artist`, `bpm` (string as djay shows
it), `key` (string as djay shows it), `elapsed`, `remaining`, `duration`
(seconds), `playing`, `notice`, `analysis` (`amplitude` 0–255, `amplitudeRate`,
`bands` as three 0–255 channels, `bandRate`, `beats`, `bars`, `bpm`) and `cues`
(`time`, `name`, `color`). Optional fields are omitted when unknown.

Empty states: a deck that djay shows without a track says "No track on deck
n"; a deck that djay's current view does not show says "Deck n is not shown
in djay" (#93); other notices are, for example, "djay is not running" or
"Accessibility permission needed"; a track without analysis shows its header
and "No analysis for this track". Beat ticks are drawn only when beats are at
least 5 px apart; bar ticks always are.

## Layout and dependencies

`S8Views` holds `DeckSnapshot` and `DeckPanelRenderer` and stays free of
Accessibility and file access. `DjayDecks` depends on `S8Views` and
`DjayLibrary` and builds snapshots directly, so no adapter type is needed.
Attribution for ported files is in `macos/DjayDecks/NOTICE` and
`macos/DjayDecks/README.md` (ported versus new files).

## Limits

- Accessibility is view-dependent: with djay's library expanded or in other
  layouts the time labels can be missing. A remaining-only layout derives
  elapsed from the library duration.
- djay can fail to answer an Accessibility read while it is busy. When the
  system reports "cannot complete" and the process holds permission, the
  page shows "djay is not answering" instead of the track (the notice page,
  as for the other notices: centred text, same position as "No track on deck
  n"). The deck keeps its track, library entry (analysis, cues) and last
  reading, so no new library lookup is needed. The time does not count on
  while waiting, and the play-state debouncer and time interpolator start
  fresh with the next answer, so the play state is confirmed again from new
  samples. The notice goes away with the first good read. A missing
  permission, or "cannot complete" without permission, still shows
  "Accessibility permission needed", and a missing djay shows "djay is not
  running"; both reset the deck. Each read waits at most 3 s (the sidebar
  reader uses the same limit). Read from code; not seen in djay. The page as
  `S8Host --screens --deck 1` would send it, made with `S8Host --replay`
  from a deck snapshot that holds only the notice and decoded by
  `s8-screen-png`:

  ![Deck page while djay is not answering](images/deck-data/04-djay-not-answering-right.png)
- Bars come from `firstDownBeatIndex` and the beat list; djay's own bar
  lines were not compared. The index may be negative or beyond the list (the
  bar phase is the index modulo 4, mathematically). Where the list skips
  beats, the skipped beats are counted so bars keep their phase: the gap is
  divided by the median of the 9 beat intervals around it (a local window,
  so a tempo change inside a track does not distort the count) and rounded.
  Only a gap of at least 1.5 intervals within 0.25 interval of a whole number
  counts as skipped beats; any other irregular spacing (a restarted grid) is
  counted as one beat. The one real analysis file in the tests (478 beats,
  spacing 0.417–0.437 s at 140 BPM) has no gaps, so its bars are unchanged;
  gap handling is covered by synthetic lists only.
- The cached waveform is about 2–11 samples per second (the sampled tracks:
  5.1–5.6 colour samples/s and about 20–22 amplitude samples/s). It is an
  overview of the whole track, not a zoomed scrolling waveform.
- Band channel 0 is bass (confirmed in the prototype); channels 1 and 2 are
  assumed mid and treble and are unverified. The page draws the amplitude
  envelope tinted by the bands.
- The key stored in the analysis blob is not decoded; the AX key is used.
- A cue point's colour is the `color` djay stores. djay stores none for many
  cues (about half of those numbered 2 to 8 in one real library), so the
  page takes the default for the cue's number (a cue with no number is
  cue 1): 1 red `#CC0000`, 2 orange `#CC8800`, 3 blue `#0000CC`, 4 yellow
  `#CCCC00`, 5 green `#00CC00`, 6 magenta `#CC00CC`, 7 teal `#00CCCC`,
  8 purple `#8800CC`. In that library each is the stored colour of all but a
  few of the cues of its number that have one. A cue numbered above 8 keeps
  the amber fallback.

  ![Cues 1 to 8 with their default colours](images/deck-cue-colours-after.png)
- Cue points come from `mediaItemUserData` (`cuePoints`). Unit tests cover
  synthetic data. On the S8 the flags of two tracks were compared with
  djay's cue points by eye
  ([deck-data-validation.md](deck-data-validation.md), row 3).
- Cue points and analysis are read again while the track stays on the deck
  (#92, #95; before that they were read once per track, and a cue point set
  later got no flag on the S8, row 11 of the same record). Every 5 s per
  deck with a track the monitor asks the catalog again. A look costs a
  directory listing and, until the file is found, one plist read for the
  analysis (kept once found), and two `stat` calls (modification date and
  size of `MediaLibrary.db` and `MediaLibrary.db-wal`), at most once per 5 s.
  The database is copied only when those changed since the last load, and the
  copy runs on the catalog's own background queue, so the 0.25 s deck poll
  never waits for it; the first load is synchronous. A failed reload keeps
  the previous library and is tried again at the next check. Adopting a
  refreshed entry for the same track does not touch the position or the
  play state. From the write of a change to the page: up to 5 s until the
  files are checked, the reload (about 0.4 s measured for a 34.5 MB
  database), then up to 5 s until the next look, so about 10 s at most.
  Seen against djay once
  ([captures](captures/djay-check-20261111T003350Z/README.md)): a removed
  and a moved cue point reached the page 4.4 s and about 6 s after djay
  wrote its library, and the analysis of a track djay analysed by itself
  after the load 4.6 s after its file was written.
  Not measured: how long after the action in djay the write to disk comes
  (the observer gave no times for the cue points; the analysis was reported
  as "1-2 seconds" and its file was written 0.7 s after the track was on
  the page). A change djay keeps only in memory cannot appear on the page.
- djay shows whole seconds, so the page knows the position exactly only at
  the moment a label moves. In the tests, with a poll every 0.25 s: a
  playing deck is 0 to 0.25 s behind djay once a poll has seen its label
  move, and never ahead; a deck paused after it played is within 0.25 s; a
  deck that has not played since it was first read, or that jumped, is
  within 0.5 s. Scratching and looping are not modelled.
- From the press of play until playing is confirmed the time stands still.
  It then moves forward once by the time played, less than 1 s (#87).
- The track start is recognised only when the track length is known from the
  library. The remaining label is taken as the remaining time cut to a whole
  second, which matches capture C of the #80 record.
- djay exposes only the decks its current view shows
  ([captures](captures/one-deck-view-20261111T000921Z/README.md), #93). Two
  Decks exposes decks 1 and 2, Four Decks all four. One Deck exposes one
  deck, and which one is not settled: in the captures a scripted cue click
  on deck 2 was followed by deck 2 and a scripted press on a deck 1 control
  by deck 1, while in the check against djay deck 1 was shown after cue
  points set by mouse and a track load on deck 2
  ([captures](captures/djay-check-20261111T003350Z/README.md)). Loading a
  track or opening the Deck menu did not change it in the captures, and a
  track dragged in while in One Deck went to the deck shown. A deck
  that is not shown keeps its track in djay. Its page says "Deck n is not
  shown in djay" and keeps the track, library entry and last reading, as for
  "djay is not answering" above; the page returns with the first read that
  has the deck again, without a new library lookup. The time of a hidden
  deck is not known while it is hidden, also when it plays. Seen against
  djay once, with deck 2 hidden and paused; not on the S8. Not looked at:
  the shown deck changing while One Deck stays open, One Deck right after
  djay starts, and the labels of a shown deck without a track (the "No
  track" case is covered by fake labels only).
- Library entries are matched by title and artist. When no entry has the
  deck's artist text, the entries with the title are used, and the deck's
  length (#94) decides among them. With both time labels the length is
  elapsed plus remaining, accepted within 2 s because each label is cut to a
  whole second. With only the remaining label the track must be at least
  remaining minus 1 s long. An entry without a stored duration is never
  ruled out by length. Entries of the artist that matches are kept even if
  their length disagrees with the labels. A title-only match gives no entry
  when no entry fits the length, or when the entries that fit differ by
  more than 2 s in stored duration; the same track twice in the library
  (durations within 2 s) still gives the first entry that has an analysis,
  else the first. A deck whose track cannot be told apart shows its djay
  labels with "No analysis for this track" and no cue flags, instead of
  another track's length, cues and waveform. The length cannot separate
  entries with one title, different artist text on the deck and in the
  library, and lengths within 2 s of each other; the page then takes the
  first of them that has an analysis, which may be the wrong one. Unit
  tests cover these cases through `DjayLibrarySnapshot` and
  `LibraryCatalog` with synthetic libraries and through `DeckMonitor` with
  fake labels; this was not seen in djay.

## Run the preview

```sh
swift run --package-path macos S8Preview macos/S8ViewsTests/Fixtures --deck 1
swift run --package-path macos S8Preview macos/S8ViewsTests/Fixtures --deck 1 --no-window --png /tmp/deck
swift run --package-path macos S8Preview macos/S8ViewsTests/Fixtures --deck-snapshot deck.json
```

`--deck N` replaces the right panel with live deck N and refreshes at 10 Hz;
`f` (or `--png`/`--frames`) also writes `deck-N.png`, `deck-N.json` (a
`DeckSnapshot`, reusable with `--deck-snapshot`) and `deck-N.s8frame`.

On the S8, `S8Host --screens --deck N` shows the page on the right panel
([panel-views.md](panel-views.md)); unchecked on hardware.

## Stream to the S8

```sh
swift run --package-path macos S8Preview --stream left=1,right=2 --seconds 20
```

`--stream` writes the live deck pages as `S8FR` records to stdout, in the
format of [screen-transport.md](screen-transport.md), for the bridge's
`--s8-screens-stdin`. It opens no device. `left=N` and `right=M` choose the
deck for each panel; one panel alone is allowed. The pages are rendered every
0.1 s and a record is written only when a page changed, so a paused deck
sends one record. Without `--seconds` it runs until its stdout closes. Each
record is logged on stderr with the deck's time and play state. When djay
cannot be read it exits with status 1 before it writes anything.

## Attended check on the S8 (#85)

This section is the plan, not a result. The run sends screen frames to the
S8 and stops NIHardwareAgent, so it needs the user's go-ahead.

What it checks: the deck pages of the paused-deck fix (#80) on the real
panels, next to djay. What it sends: full-panel frames to interface 6 through
the bridge's `--s8-screens-stdin` channel, which was checked on this S8 on
2026-11-10 ([screen-transport-validation.md](screen-transport-validation.md)).
It sends no LED reports, no feature reports and nothing to interface 4.

### Preconditions

- Exactly one Kontrol S8 on USB; Traktor, Maschine and Komplete Kontrol quit.
- djay open with decks 1 and 2 loaded and paused. For the last step, decks 3
  and 4 loaded as well.
- The terminal that runs the check holds Accessibility permission. The runner
  reads djay once before it stops anything and refuses when that fails.
- Run from an interactive terminal: `mise run s8:deck-page`. To continue a
  run that stopped early, name the first step, for example
  `mise run s8:deck-page b-play`.
- The runner stops `NIHardwareAgent` with `kill -9` and relaunches it with
  `open -g` on every exit path. While it is stopped, the S8's controls reach
  djay only through the bridge mapping, so start and stop decks in djay's
  window.

The runner builds the release bridge and `S8Preview`, and writes
`docs/s8/captures/deck-page-<UTC>/`: per step the bridge output (`.txt`) and
the stream log (`.stream.txt`), plus `notes.txt` with every answer,
`snapshots.txt` and `events.txt`.

### Steps

Every step runs `S8Preview --stream … --seconds N | bridge --device s8 --midi
full --s8-screens-stdin --until-stopped`. The stream ends after N seconds,
the bridge then sends black frames to both panels and exits. A step passes
when the status lines are `pending`, `active`, `inactive`, the bridge exits
with status 0 and the observer answers `y` to each question.

| Step | Panels | Time | The observer does | Pass |
| ---- | ------ | ---- | ----------------- | ---- |
| `a-paused` | left deck 1, right deck 2 | 20 s | Nothing; both decks stay paused | Each panel shows its deck with title, time, waveform and PAUSED. Both times and playheads stand still for the 20 s. Both panels go black at the end. |
| `b-play` | left deck 1, right deck 2 | 60 s | Plays deck 1 for about 10 s, pauses about 10 s, plays again, pauses | While deck 1 plays the left panel says PLAYING and its time moves with djay's. On pause the time stops without stepping back. While it is paused the time stands still, and on resume it goes on from there without a jump by the length of the pause. The right panel stands still. |
| `c-four-decks` | left deck 3, right deck 4 | 60 s | Switches djay to Four Decks first; plays deck 3 about 10 s and pauses, then deck 4 about 10 s and pauses | The panels show decks 3 and 4. Each time moves only while its own deck plays and does not step back on pause. |

Expected and not a fault (#87):

- PLAYING appears up to about 1 s after play. Until then the time stands
  still; when PLAYING appears the time moves forward once by less than 1 s,
  which is the time played since the press.

Fault: a time that runs on a paused deck, a time that steps back on pause, a
page of the wrong deck, a garbled or frozen panel while its deck plays, a
status line `failed`, or a bridge
exit status other than 0. On a fault answer `n` or describe it; the runner
skips the remaining steps and goes to recovery. `stop` at any prompt and
Ctrl-C during a step do the same; after Ctrl-C the panels go black.

### Recovery

The runner relaunches NIHardwareAgent, waits 5 s, and asks what the panels
show and whether djay reacts to the left PLAY button. If the agent does not
come back, start it by hand:
`open -g "/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"`.

## Validation record

Observed on 2026-11-10 with djay Pro running and the terminal holding
Accessibility permission, using `--deck 1 --no-window --png /tmp/deck`:

- Deck 1 was loaded and paused. Populated: title "Cheeni", artist, BPM
  "140.0", key "7 B", elapsed 54.9 s, remaining 131 s, duration 185.9 s,
  playing false, 8 cue points, amplitude (22.05 samples/s), 3 band channels
  (5.56 samples/s), beats, bars. The playhead and cue markers matched djay's
  position and cue list on screen to the eye only; no frame-accurate comparison.

  ![Deck page for deck 1, loaded and paused](images/deck-page-live.png)

- Decks 2, 3 and 4 produced no title or time through the same labels, so the
  empty page was rendered.

  ![Deck page for a deck with no track](images/deck-page-empty.png)

  djay's layout was not changed and no track was loaded. The record below
  covers loaded decks 2, 3 and 4.
- Playing state, live time extrapolation and the 10 Hz window refresh were
  not exercised on live data (the deck was paused). The record below covers
  the playing state and the extrapolation.

### Paused deck time and Four Decks (#80)

Observed on 2026-11-10 with djay Pro in English and the terminal holding
Accessibility permission. A test read `DeckMonitor` about every 0.2 s while a
second process sampled djay's play value and remaining label at 100 Hz. The
layout showed only the remaining time. Captures:
`captures/deck-pages-20261110/`, one `-page` and one `-djay` file per check.

| Check | Revision | Setup | Observed |
| ----- | -------- | ----- | -------- |
| A | before the fix | Two Decks, deck 1 played, paused and resumed by the user | djay only: a playing deck's play value stays `Active`; a paused deck's value alternates every 0.5 s, in phase on all decks; the remaining label changes once a second. |
| B | `f76b2d1` | Two Decks, both paused, 9.4 s | Deck 1 held 27.91 s and deck 2 held 0.70 s; neither was reported playing. |
| C | `f76b2d1` | Two Decks, deck 1 played, paused and resumed by the user, 45 s | Playing was reported 1.1 to 1.5 s after play and paused within 0.7 s of pause. After about 18 s paused the time continued from the paused position. Deck 2 held 0.70 s. |
| D | `f76b2d1` | Four Decks, all paused, 9.4 s | Failed. Polls were 0.93 s apart, decks 2, 3 and 4 were reported playing for 6 to 8 polls and their time ran ahead by 4.6 to 8.4 s. |
| E | `9eff1fb` | Four Decks, all paused, 16 s, 107 samples per deck | All four decks held their time and none was reported playing. |
| F | `9eff1fb` | Four Decks, the user played and paused deck 3, then deck 4, then deck 3 again, 75 s, 474 samples per deck | Decks 1 and 2 held. Deck 3 was reported playing 1.2 s and 1.3 s after play and paused 0.6 s and 0.5 s after pause; after 32 s paused it continued from 44.04 s. Deck 4 was reported playing 0.8 s after play and paused 0.6 s after pause. A poll took 0.105 s (median of 238). |

- `--deck N --no-window --png` on `9eff1fb` for decks 1 to 4, all paused:
  each page showed title, artist, BPM, key, remaining time, waveform and cue
  markers (8, 7, 8 and 5 cues).

  ![Deck page for deck 3 in Four Decks](images/deck-page-deck3.png)
  ![Deck page for deck 4 in Four Decks](images/deck-page-deck4.png)

- In F the page time stepped back when a playing deck was paused (59.88 s to
  59.04 s on deck 3), and decks at the start of a track showed 0.04 s, 0.34 s
  and 0.70 s. Both come from the cut-off label (#83) and were not fixed at
  that revision; see "Time inside djay's second (#83)" below.
- A first recording of F was lost before it was saved; the user repeated the
  steps.

Not exercised: French labels on a live djay, a layout that shows the elapsed
time, a changed pitch, scratching and looping, and the 10 Hz preview window.

Automated: `swift test --package-path macos` covers label parsing (English and
French), splitting one walk by deck, time interpolation and debouncing
including a blink sampled once per period, analysis decoding against a real
trimmed `.djayMetadata` fixture, deck tracking and the page render including
the empty state. These checks sent nothing to the S8; the next section covers
the panels.

### Deck pages on the S8 (#85)

Run once on 2026-11-10 at `23b8d28` with `mise run s8:deck-page`, following
the plan in "Attended check on the S8" above. The observer's answers are in
`captures/deck-page-20261110T192200Z/notes.txt`; the same folder holds the
bridge output and the stream log of each step.

| Step | Bridge | Records sent | The observer reported |
| ---- | ------ | ------------ | --------------------- |
| `a-paused` | `pending`, `active`, `inactive`; exit status 0 | 2 | `y` to all three questions: the left panel showed deck 1 and the right panel deck 2 with title, time, waveform and PAUSED; both times and playheads stood still for the 20 s; both panels went black at the end. |
| `b-play` | `pending`, `active`, `inactive`; exit status 0 | 117 | `y`: while deck 1 played the left panel said PLAYING and its time moved with djay's. To the question whether the paused time stood still and went on without a jump by the length of the pause: "on resume, there was a small jump in the time on the S8 display". The runner took that as a fault and stopped; the question about the right panel was not asked. |
| `c-four-decks` | Not run in this run; see the second run below | | |

After recovery the panels showed the Controller Editor labels and djay
reacted to the left PLAY button.

What the stream log of `b-play` shows was sent for deck 1 (sent, not seen):

| Time in step | Event | Page elapsed |
| ------------ | ----- | ------------ |
| 0.0 s to 11.1 s | Paused, one record | 47.91 s |
| 11.1 s | First PLAYING record | 48.78 s (+0.87 s) |
| 25.0 s | Last PLAYING record | 62.39 s |
| 25.1 s | PAUSED record | 61.91 s (-0.48 s) |
| 25.1 s to 37.1 s | Paused for 12.0 s, no record | 61.91 s |
| 37.1 s | First PLAYING record after resume | 62.83 s (+0.92 s) |
| 52.8 s | Last PLAYING record | 78.75 s |
| 52.9 s | PAUSED record | 77.91 s (-0.84 s) |

Deck 2 was sent once, at 0.70 s, and never again.

Reading of the reported jump, from the log and not confirmed by the observer:
the step on resume was 0.92 s after a pause of 12.0 s, so it is not the jump
by the length of the pause that #80 fixed. It has the size of the step #83
describes when play is confirmed, and the steps back on pause are the
cut-off label of #83. The observer saw it on the panel; whether a step of
that size is acceptable is not decided.

After this run the runner asks the paused time and the resume in separate
questions and names the expected step.

Second run on 2026-11-10 at `d27bf56` with `mise run s8:deck-page
c-four-decks`, which skips the first two steps. Answers and logs:
`captures/deck-page-20261110T193343Z/`.

| Step | Bridge | Records sent | The observer reported |
| ---- | ------ | ------------ | --------------------- |
| `c-four-decks` | `pending`, `active`, `inactive`; exit status 0 | 91 | `y` to both questions: the left panel showed deck 3 and the right panel deck 4; each time moved only while its own deck played and stood still while it was paused. The question named one step of up to about 1 s when PLAYING appears as expected. Nothing else was reported on the panels. |

After recovery the panels showed the Controller Editor labels and djay
reacted to the left PLAY button.

Not checked on the S8: whether the right panel stood still during `b-play`,
and a second run of `b-play` with the reworded questions.

### Time inside djay's second (#83)

Automated: checked on 2026-11-10 at `0c3264e` with
`swift test --package-path macos`. Both test classes feed `DeckTracker` a
poll every 0.25 s at five poll phases and read the page every 0.1 s.

`DeckTimeReplayTests` replays the djay side of capture C
(`captures/deck-pages-20261110/c-two-decks-play-djay.txt.gz`, copied to
`macos/DjayDecksTests/Fixtures/`). The true position comes from the moments
the remaining label moved in that capture.

| Deck 1 in capture C | Page in the replay, over the five poll phases |
| ------------------- | -------------------------------------------- |
| First play (label moved at 9.48 s) | PLAYING at 9.1 to 9.3 s. The time moved forward once by 0.5 s, from the middle of the second to its end. |
| Playing, from 0.25 s after a label move | 0 to 0.25 s behind djay, never ahead. |
| Pauses at 21.41 s and 42.88 s | PAUSED 0.0 to 0.3 s later. No step back. |
| Paused | Within 0.25 s of the paused position. |
| Resumes at 40.00 s and 47.87 s | PLAYING 0.1 to 0.45 s later. The time moved forward once by 0 to 0.3 s. |
| Whole capture | The page second equals djay's label at the last poll, and the time never steps back. |

Deck 2 of capture C stood at the start of its track and showed 0.70 s before
the fix. In the replay it shows 0 s elapsed for the whole capture.

For comparison, the stream log of `b-play` before the fix (another session,
see the #85 record) has steps back of 0.48 s and 0.84 s on pause and steps
forward of 0.87 s and 0.92 s on resume.

`DeckTimeSimulationTests` runs a simulated deck that cuts its label the same
way, with presses that fall at other moments in the second and the blink at
other phases. It holds the same bounds, and bounds the time before playing
is confirmed: at most 0.75 s ahead and at most 0.7 s plus two polls behind.

On the S8: run once on 2026-11-10 with `mise run s8:deck-page b-play` at
`71470ad`, the same code before it was rebased onto `main`. Answers and logs:
`captures/deck-page-20261110T201431Z/`.

| Step | Bridge | Records sent | The observer reported |
| ---- | ------ | ------------ | --------------------- |
| `b-play` | `pending`, `active`, `inactive`; exit status 0 | 102 | `y` to all four questions: while deck 1 played the left panel said PLAYING and its time moved with djay's; on pause the left time stopped without stepping back and stood still; on resume it did not jump by the length of the pause; the right panel stood still. |
| `c-four-decks` | `pending`, `active`, `inactive`; exit status 0 | 120 | `y` to both questions: deck 3 left and deck 4 right; each time moved only while its own deck played, and on pause it stopped without stepping back and stood still. |

Both resume questions named one move forward of less than 1 s as known
(#87). Nothing else was reported on the panels: "everything looked good".
After recovery the panels showed the Controller Editor labels and djay
reacted to the left PLAY button.

What the stream logs show was sent (sent, not seen):

| Deck | Event | Page elapsed |
| ---- | ----- | ------------ |
| 1 | First PLAYING record | 79.41 s to 79.91 s (+0.50 s) |
| 1 | PAUSED record | 91.76 s to 91.91 s (+0.15 s) |
| 1 | First PLAYING record after 15.9 s paused | 91.91 s to 92.76 s (+0.85 s) |
| 1 | PAUSED record | 111.33 s to 111.66 s (+0.33 s) |
| 2 | One record, at the start of its track | 0.00 s |
| 3 | First PLAYING record | 80.54 s to 81.23 s (+0.69 s) |
| 3 | PAUSED record | 99.22 s to 99.54 s (+0.32 s) |
| 4 | First PLAYING record | 44.84 s to 45.34 s (+0.50 s) |
| 4 | PAUSED record | 60.05 s to 60.09 s (+0.04 s) |

No record stepped back. The moves when PLAYING appears, 0.50 s to 0.85 s,
are the step of #87.

Not checked: the page time against djay's true position on a live djay, a
layout that shows the elapsed label, a changed pitch, a jump, scratching and
looping.
