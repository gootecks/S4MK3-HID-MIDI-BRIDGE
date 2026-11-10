# Display buttons and screen encoders: validation (#51)

Two kinds of evidence, kept apart. Nothing here was run against the S8.

## Observed: stand-in smoke test, live djay Pro, 2026-11-10

`S8Host` was started with `--bridge` pointing at a shell script that ignores
its arguments and prints the lines below 2.5 s apart (the real bridge was not
run; no S8 traffic). djay Pro was running with the sidebar open on Library,
nothing selected in the sidebar table. Only tabs and selection were touched;
nothing was loaded, queued or played. A separate read-only probe
(`examples/djay-sidebar-probe.swift --watch`) logged what djay showed.

| Line fed | Observed in djay |
| --- | --- |
| `S8_CONTROL display_button left 3 down` | Queue tab selected |
| `S8_CONTROL display_button left 3 up` | no change |
| `S8_CONTROL display_button left 7 down` | Info tab selected, Info sub-tab, track Diwali |
| `S8_CONTROL display_button right 3 down` | Info > Related selected |
| `S8_CONTROL display_button right 2 down` | Info sub-tab selected again |
| `S8_CONTROL display_button left 2 down` | Library tab selected |
| `S8_CONTROL screen_encoder left 1 100` | sidebar row 1 (second track, Cheeni) selected, two rows from no selection |
| `S8_CONTROL screen_encoder left 1 -50` | sidebar row 0 (Anybody) selected |
| `S8_CONTROL screen_encoder right 2 20` | no change (under 50 counts) |

Also seen: the script's own lines (`fake bridge started: --device s8 --midi
full --until-stopped --app-events`, `fake bridge done`) were passed through to
S8Host's stdout and not parsed. With stdin closed, S8Host ended the child
(exit status 15, no child left); on SIGTERM the child was gone 1.5 s later; a
missing bridge path gives a message and exit 4.

After the Info panel showed the selected row's track instead of the main
table's (Anybody instead of Diwali), a second stand-in line
`S8_CONTROL screen_encoder left 1 150` selected Diwali again; djay was left
on Library with Info showing Diwali.

Automated: `cargo test --locked app_events` (button and encoder mapping, buttons
1 and 4, resting reports, wrap), `swift test` for `S8ControlTests`
(parser, accumulator, router) and `DjaySidebarTests` (element trees built in
code, not recorded from djay).

## Not checked: needs the S8

- [ ] Each display button, left 2, 3, 5, 6, 7 and right 2, 3, does its action on
      the S8 and button up does nothing; buttons 1 and 4 still do the NI actions.
- [ ] Left 8 and right 5-8 do nothing visible.
- [ ] All eight screen encoders move the list in the right direction; feel
      of 50 counts per row (20 rows per turn): too fast or slow?
- [ ] The first movement after start does not jump the selection.
- [ ] Rapid turning keeps up with djay (each step re-reads the sidebar tree).
- [ ] Load and queue act on a row selected by the encoders (unverified even
      in the stand-in test; see [the finding](djay-sidebar-access.md#sidebar-selection-by-axselectedrows-51)).
- [ ] NIHardwareAgent stopped as required; S8Host stops cleanly on Ctrl-C and
      the bridge restores nothing it should not.
