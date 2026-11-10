# S8Host

Runs the Rust bridge as a child process, drives djay's sidebar from the Kontrol S8 display buttons and screen encoders, and with `--screens` shows the sidebar on the S8 panels.

```sh
swift run --package-path macos S8Host --bridge target/release/s4mk3-hid-midi-bridge
swift run --package-path macos S8Host --bridge target/release/s4mk3-hid-midi-bridge --screens [--deck N [--log-deck-polls]] [--poll SECONDS]
swift run --package-path macos S8Host --replay docs/s8/fixtures/panel-views --records /tmp/records
```

- Starts the bridge with `--device s8 --midi full --until-stopped --app-events`, plus `--s8-screens-stdin` with `--screens`.
- Applies the bridge's `S8_CONTROL` stdout lines through `DjaySidebar` (Accessibility); every other bridge line is passed through to stdout. Bridge stderr is inherited.
- `--screens` sends screen frames to the S8: after each control event and every `--poll` seconds (default 1) it reads the sidebar, renders both panels with `S8Views` and writes the changed ones to the bridge's stdin as `S8FR` records. `--deck N` puts deck N on the right panel, read every 0.25 s through `DjayDecks`. `--log-deck-polls` writes one stderr line per deck read, `S8Host: deck poll gap=<s since the previous read started> read=<s the read took>`.
- `--replay DIR --records OUT` needs neither djay nor the S8: it runs the snapshot files of `DIR` in name order through the same panel code and writes the records of each as `OUT/<name>.s8fr`. `DIR/decks/<name>.json` is the deck snapshot for that state.
- Needs Accessibility access for the terminal or app that runs it and a running djay Pro. It checks both at start and exits with a message if either is missing.
- Stops the child on SIGINT, SIGTERM and when its own stdin closes. The session stops first: its timers are cancelled and no further record is written, without waiting for a read or write that is in progress. With `--screens` it then closes the bridge's stdin, so the bridge sends its black exit frames and exits by itself; SIGINT follows after 5 s and SIGTERM after 10 s if it has not. It does not stop or start `NIHardwareAgent`; stop it separately before a real session (see `docs/s8/translator-validation.md`).
- Exit status: the bridge's; 2 for wrong arguments, 3 without Accessibility access or djay, 4 when the bridge cannot be started, 1 when a replay fails.

| File | Role |
| ---- | ---- |
| `main.swift` | Argument check, start-up checks, signals, bridge output loop. |
| `HostOptions.swift` | Command-line flags. |
| `Bridge.swift` | The child process: arguments, stdin records, stop order. |
| `LiveWiring.swift` | Connects `DjaySidebar`, `DeckMonitor` and the bridge to a `PanelSession`. |
| `Replay.swift` | Snapshot files to record files. |

## Queues

`PanelSession` (target `S8Session`) runs a session on three serial queues, so that a slow part does not hold up the others:

| Queue | Work | Blocks on |
| ----- | ---- | --------- |
| Sidebar | Applies control events to djay, reads the sidebar after the last waiting event and every `--poll` seconds. | Accessibility calls for the sidebar. |
| Deck | `DeckMonitor.poll()` every 0.25 s. | Accessibility calls for the decks. |
| Output | Renders both panels and writes the changed records to the bridge. | The bridge's stdin pipe. |

- The sidebar queue hands over one sidebar snapshot; a newer one replaces an unsent one. The deck queue only asks for a redraw; the output queue takes the deck state from `DeckMonitor.snapshot`, which reads no Accessibility.
- At most one redraw runs and one waits. While a write is blocked, any number of updates become one redraw of the newest state.
- All three serve one djay process, so Accessibility calls of the sidebar and deck queues can still slow each other inside djay. `--log-deck-polls` measures it.

Targets: `S8Control` (line parser, button-to-action mapping, encoder accumulator, no Accessibility), `DjaySidebar` (Accessibility reading and actions), `DjayDecks` (deck state), `S8Views` (rendering and records), `S8Session` (queues, no Accessibility and no child process), `S8Host` (this executable). Controls: `docs/s8/display-buttons.md`. Panels: `docs/s8/panel-views.md`.
