# Deck page against djay: cue points, analysis, One Deck view

Taken on 2026-11-10 (UTC 2026-11-11 00:33) at `3038a84` of PR #99 (the same
tree as `453798d` before its rebase; it holds #92, #95, #94 and #93), with
djay Pro in English in Two Decks,
"Duro" on deck 1 and "Cheeni" on deck 2, both paused. Nothing was sent to the
S8 and NIHardwareAgent kept running.

Each step ran `S8Preview --stream left=1,right=2` with its records written to
a file. `*.stream.log.gz` holds its `DECKSTREAM` lines: one per record, and a
record is written when a page changes. A script looked every 0.25 s at the
modification time and size of `MediaLibrary.db`, `MediaLibrary.db-wal` and
the shards of djay's `Metadata` folder. The pictures are the single records
decoded by `s8-screen-png`. The observer acted in djay and answered a form
after each step; the answers are quoted.

| Step           | The observer did                                                                  | The observer reported                                                                                                                   | Log and files                                                                                                                                                                     | Pictures                                                                                                                                                                             |
| -------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `a-cue`        | Cue points on deck 2 with the mouse on djay's cue buttons, deck paused at 82.41 s | "cue 6"; "yes" to: cue 5 at the playhead removed, cue 6 set at the playhead                                                             | Library written 36.2 s into the step, right panel record at 40.6 s. Library written again at 44.6 s and 45.6 s, record at 51.1 s. Elapsed and play state the same in all records. | `a-cue-right-before.png` (green flag at the playhead, magenta flag near 2:05), `a-cue-right-after-1.png` (green flag gone), `a-cue-right-after-2.png` (magenta flag at the playhead) |
| `b-unanalysed` | Loaded "Noche Without You" on deck 2                                              | "it starting analyzing by itself. took 1-2 seconds"                                                                                     | Track on the page at 24.36 s. `c1c5370582694f092d91aca2844cb2a3.djayMetadata` written at 25.07 s. Record at 29.71 s.                                                              | `b-unanalysed-right-before.png` ("No analysis for this track"), `b-unanalysed-right-after.png` (waveform, beat ticks, six cue flags)                                                 |
| `c-one-deck`   | Switched to One Deck view, later back to Two Decks                                | djay showed deck 1. To whether One Deck could be made to show deck 2: "not that i'm aware of. i switched it back to Two Decks manually" | Deck 2 record without title or time at 18.09 s; deck 2 back with the same track and time at 291.36 s. No record for deck 1 after the first.                                       | `c-one-deck-right-hidden.png` ("Deck 2 is not shown in djay"), `c-one-deck-right-back.png`, `c-one-deck-left.png` (deck 1, unchanged)                                                |

## What this shows and what it does not

- A cue point removed and a cue point moved while the track stayed loaded
  reached the page 4.4 s and about 6 s after djay wrote its library. A cue
  point in a slot that was empty before was not set in this check.
- djay analysed the track by itself within about 2 s of the load, and the
  page showed the analysis 4.6 s after the file was written, without a
  reload of the track.
- In One Deck view the page of the deck that is not shown said so and came
  back with its track. Deck 1 was shown although the last actions before the
  switch were on deck 2 (cue points by mouse, a track load). In
  `../one-deck-view-20261111T000921Z/` a scripted press of a cue button on
  deck 2 was followed by deck 2 being shown. What decides the deck is not
  known.
- One track per step, one run. Playing decks, a hidden deck that plays, a
  deck without a track and the S8 itself were not part of it.
