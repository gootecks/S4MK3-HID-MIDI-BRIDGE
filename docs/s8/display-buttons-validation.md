---
summary: "The stand-in test and the attended run of the display buttons on the S8 (#51)."
type: validation
---

# Display buttons and screen encoders: validation (#51)

Three kinds of evidence, kept apart: a stand-in test without the S8, the
automated tests, and one attended run on the S8.

## Observed: stand-in smoke test, live djay Pro, 2026-11-10

`S8Host` was started with `--bridge` pointing at a shell script that ignores
its arguments and prints the lines below 2.5 s apart (the real bridge was not
run; no S8 traffic). djay Pro was running with the sidebar open on Library,
nothing selected in the sidebar table. Only tabs and selection were touched;
nothing was loaded, queued or played. A separate read-only probe
(`examples/djay-sidebar-probe.swift --watch`) logged what djay showed.

| Line fed                                 | Observed in djay                                                          |
| ---------------------------------------- | ------------------------------------------------------------------------- |
| `S8_CONTROL display_button left 3 down`  | Queue tab selected                                                        |
| `S8_CONTROL display_button left 3 up`    | no change                                                                 |
| `S8_CONTROL display_button left 7 down`  | Info tab selected, Info sub-tab, track Diwali                             |
| `S8_CONTROL display_button right 3 down` | Info > Related selected                                                   |
| `S8_CONTROL display_button right 2 down` | Info sub-tab selected again                                               |
| `S8_CONTROL display_button left 2 down`  | Library tab selected                                                      |
| `S8_CONTROL screen_encoder left 1 100`   | sidebar row 1 (second track, Cheeni) selected, two rows from no selection |
| `S8_CONTROL screen_encoder left 1 -50`   | sidebar row 0 (Anybody) selected                                          |
| `S8_CONTROL screen_encoder right 2 20`   | no change (under 50 counts)                                               |

Also seen: the script's own lines were passed through to S8Host's stdout and
not parsed. They were `fake bridge started: --device s8 --midi full
--until-stopped --app-events` and `fake bridge done`. With stdin closed,
S8Host ended the child (exit status 15, no child left). On SIGTERM the child
was gone 1.5 s later. A missing bridge path gives a message and exit 4.

After the Info panel showed the selected row's track instead of the main
table's (Anybody instead of Diwali), a second stand-in line selected Diwali
again. The line was `S8_CONTROL screen_encoder left 1 150`. djay was left
on Library with Info showing Diwali.

Automated:

- `cargo test --locked app_events`: button and encoder mapping, buttons 1
  and 4, resting reports, wrap.
- `swift test`: `S8ControlTests` (parser, accumulator, router) and
  `DjaySidebarTests` (element trees built in code, not recorded from djay).

## Observed on the S8: attended run, 2026-11-10

`mise run s8:display-buttons` (`display-buttons-check.sh`), run by the user at
the S8 with djay Pro and Controller Editor open. The runner sends nothing to
the S8. It stops NIHardwareAgent, starts `S8Host`, asks one question per row,
stops `S8Host` with SIGINT and restores the agent. Record:
[captures/display-buttons-20261110T161510Z](captures/display-buttons-20261110T161510Z/).
`notes.txt` holds the answers. `S8Host` does not print the `S8_CONTROL` lines
it handles, so the answers are the record of what djay did; `host.log.gz`
holds the bridge's own log of the buttons and encoder touches.

The "Reported" column is the user's answer, unchanged.

| Row | Action                                                                                      | Expected         | Reported                                                                |
| --- | ------------------------------------------------------------------------------------------- | ---------------- | ----------------------------------------------------------------------- |
| 1   | Left screen encoder 1, one small click, first movement after start: did the selection jump? | at most one row  | "yes"; asked again after the run: "1 row"                               |
| 2   | Left button 3                                                                               | Queue            | "Queue"                                                                 |
| 3   | Hold and release left button 3: did the release change anything?                            | no               | "no"                                                                    |
| 4   | Left button 5                                                                               | Match            | "Match"                                                                 |
| 5   | Left button 6                                                                               | History          | "History"                                                               |
| 6   | Left button 7                                                                               | Info             | "Info"                                                                  |
| 7   | Right button 3                                                                              | Info > Related   | "Info > Related"                                                        |
| 8   | Right button 2                                                                              | Info sub-tab     | "Info sub-tab"                                                          |
| 9   | Left button 2                                                                               | Library          | "Library"                                                               |
| 10  | Left button 8, right buttons 5, 6, 7, 8: any change in djay?                                | no               | "no"                                                                    |
| 11  | Buttons 1 and 4 on both sides, agent stopped: any change in djay or on the panels?          | no               | "no"                                                                    |
| 12  | Left encoders 1 to 4: which moved the selection down on clockwise?                          | all four         | "all four"                                                              |
| 13  | Right encoders 1 to 4: which moved down on clockwise?                                       | all four         | "all four"                                                              |
| 14  | 20 rows per full turn: too fast, too slow or right?                                         | –                | "right"                                                                 |
| 15  | Fast spin through a long list: keep up, lag or stop?                                        | –                | "it lags a little bit"                                                  |
| 16  | Select a row with an encoder in Queue or History, then load or queue it                     | acts on that row | "i pushed the right Browse encoder and it loaded on Deck B as expected" |
| 17  | Did either panel change since the agent stop?                                               | no               | "no"                                                                    |

After the rows:

- `S8Host` stopped within 5 s of SIGINT with exit status 15 and left no
  bridge process. Status 15 is the bridge ending on the SIGTERM that `S8Host`
  sends it, as in the stand-in test.
- With `S8Host` stopped and the agent still stopped, the panels showed the
  Controller Editor labels.
- After the agent was relaunched: Controller Editor labels on the panels,
  Djay reacted to left PLAY, and the NI action of button 1 or 4 worked again.

Limits of this run:

- Row 1: the screen encoders have no clicks. The user: "they just go 360
  degrees with no clicks or stops or center points". "One small click" in the
  question therefore meant a small turn; the runner now asks for a few
  degrees and for the number of rows.
- Row 15: the selection lags a little behind a fast spin. Each step re-reads
  djay's sidebar tree. It was not measured.
- Row 16 covers loading with the right Browse encoder press. Queueing a
  selected row was not tried.
- Row 11 shows that buttons 1 and 4 do nothing in djay while the agent is
  stopped. Their NI actions were tried once, after recovery, on one button.
- One run, one S8, two-deck djay layout.
