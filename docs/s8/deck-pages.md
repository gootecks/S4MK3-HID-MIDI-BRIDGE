# Deck pages (#45)

A 480×272 page for one djay deck: title, artist, BPM, key, elapsed and
remaining time, play state, the overview waveform with a playhead, bar and
beat ticks, and cue markers. Nothing here is sent to the S8: the page renders
to a bitmap, a PNG or an `.s8frame` file through `S8Preview`.

## Data sources

| Data | Source | Notes |
| ---- | ------ | ----- |
| Title, artist, BPM, key, elapsed, remaining, playing | djay window through Accessibility (`DjayDecks`, polled by `DeckMonitor`) | Needs Accessibility permission for the process. One poll walks the window once and splits the labels by deck number, so the cost does not grow with the number of decks (about 0.1 s measured with four decks). Title, artist, key and BPM are read from English and French labels (`Deck n` and `Platine n`); the time and play labels are matched in English only. |
| Play state | `PlayStateDebouncer` (ported, extended) | The play button of a paused deck blinks at 1 Hz, lit for 0.5 s and unlit for 0.5 s; a playing deck stays lit. Playing is reported after the button has been lit for 0.7 s, counted only over samples at most 0.45 s apart, because the unlit half of a blink fits in a longer gap. Paused is reported on the first unlit sample. |
| Time between AX updates | `TimeInterpolator` (ported, extended) | Extrapolates elapsed and remaining while the debounced state is playing, scaled by the pitch percentage, and eases corrections. When playing is confirmed it counts from the first lit sample, not from the last label change. |
| Duration, cue points | Read-only `DjayLibrary` snapshot (copy of `MediaLibrary.db` with `-wal` and `-shm`) | The track is matched by title and artist. Used to derive elapsed when only remaining is shown. |
| Overview waveform, 3-band samples, beats, bars | `~/Library/Group Containers/VJXTL73S8G.com.algoriddim.userdata/Library/Application Support/Algoriddim/Metadata/**/<trackID>.djayMetadata` | Binary plist. Amplitude: zlib bytes at `lowRateWaveFinalSampleRate`. Bands: zlib, bfloat16 triples at `lowRateSampleRate`, normalised by the global peak. Beats: zlib big-endian float32 seconds. Bars: every 4th beat starting at `firstDownBeatIndex % 4`. |

djay's live database and cache are only read; the snapshot is a temporary copy.

## `DeckSnapshot` (schema 1, `S8Views`)

`schema`, `deck`, `capturedAt`, `title`, `artist`, `bpm` (string as djay shows
it), `key` (string as djay shows it), `elapsed`, `remaining`, `duration`
(seconds), `playing`, `notice`, `analysis` (`amplitude` 0–255, `amplitudeRate`,
`bands` as three 0–255 channels, `bandRate`, `beats`, `bars`, `bpm`) and `cues`
(`time`, `name`, `color`). Optional fields are omitted when unknown.

Empty states: no track shows "No track on deck n" (or `notice`, for example
"djay is not running" or "Accessibility permission needed"); a track without
analysis shows its header and "No analysis for this track". Beat ticks are
drawn only when beats are at least 5 px apart; bar ticks always are.

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
- The cached waveform is about 2–11 samples per second (the sampled tracks:
  5.1–5.6 colour samples/s and about 20–22 amplitude samples/s). It is an
  overview of the whole track, not a zoomed scrolling waveform.
- Band channel 0 is bass (confirmed in the prototype); channels 1 and 2 are
  assumed mid and treble and are unverified. The page draws the amplitude
  envelope tinted by the bands.
- The key stored in the analysis blob is not decoded; the AX key is used.
- Cue points come from `mediaItemUserData` (`cuePoints`). Unit tests cover
  synthetic data; the live run showed 8 cues, not compared with djay's list.
- Playback position is a prediction between AX updates; it can drift by the
  AX update interval and while scratching or looping.
- With only the remaining time shown, the page is ahead of djay by up to
  1 s, because djay cuts the fraction off the label (#83).
- Decks 3 and 4 are read in djay's Four Decks layout. A deck that is not in
  the current layout looks the same as an empty deck.

## Run the preview

```sh
swift run --package-path macos S8Preview macos/S8ViewsTests/Fixtures --deck 1
swift run --package-path macos S8Preview macos/S8ViewsTests/Fixtures --deck 1 --no-window --png /tmp/deck
swift run --package-path macos S8Preview macos/S8ViewsTests/Fixtures --deck-snapshot deck.json
```

`--deck N` replaces the right panel with live deck N and refreshes at 10 Hz;
`f` (or `--png`/`--frames`) also writes `deck-N.png`, `deck-N.json` (a
`DeckSnapshot`, reusable with `--deck-snapshot`) and `deck-N.s8frame`.

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
  and 0.70 s. Both come from the cut-off label (#83) and are not fixed.
- A first recording of F was lost before it was saved; the user repeated the
  steps.

Not exercised: French labels on a live djay, a layout that shows the elapsed
time, a changed pitch, scratching and looping, and the 10 Hz preview window.

Automated: `swift test --package-path macos` covers label parsing (English and
French), splitting one walk by deck, time interpolation and debouncing
including a blink sampled once per period, analysis decoding against a real
trimmed `.djayMetadata` fixture, deck tracking and the page render including
the empty state. Not verified: anything on the S8 hardware and frame upload of
deck pages.
