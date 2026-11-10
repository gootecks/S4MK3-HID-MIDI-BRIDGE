# S8Host

Runs the Rust bridge as a child process and drives djay's sidebar from the Kontrol S8 display buttons and screen encoders.

```sh
swift run --package-path macos S8Host --bridge target/release/s4mk3-hid-midi-bridge
```

- Starts the bridge with `--device s8 --midi full --until-stopped --app-events`.
- Applies the bridge's `S8_CONTROL` stdout lines through `DjaySidebar` (Accessibility); every other bridge line is passed through to stdout. Bridge stderr is inherited.
- Needs Accessibility access for the terminal or app that runs it and a running djay Pro. It checks both at start and exits with a message if either is missing.
- Stops the child on SIGINT, SIGTERM and when its own stdin closes. It does not stop or start `NIHardwareAgent`; stop it separately before a real session (see `docs/s8/translator-validation.md`).

Targets: `S8Control` (line parser, button-to-action mapping, encoder accumulator, no Accessibility), `DjaySidebar` (Accessibility reading and actions), `S8Host` (this executable). Controls and behaviour: `docs/s8/display-buttons.md`.
