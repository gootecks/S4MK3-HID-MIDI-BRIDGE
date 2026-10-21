# S4 MK3 bridge issues

Current checklist as of October 20, 2026. This is the source of truth for issue
status; [the mapping audit](mapping-audit.md) retains detailed findings and
historical snapshots.

Update this document as work completes. Check an item only when the agreed
behavior is implemented and verified. For physical controller behavior,
software checks alone are insufficient: record live confirmation as well.
Keep completed items and add their completion date and a short evidence note.

## Completed

- [x] **Preview encoder scrubbing** - Preview toggles playback and
  Shift+BROWSER emits directional preview seek pulses. The user confirmed
  the current scrubber works on October 21, 2026. A proposed one-handed
  replacement is tracked below.
- [x] **Saved hardware wheel tension** - Jog > Hardware exposes a shared
  Assist / Neutral / Brake setting with independent per-wheel motion/touch
  response. User-confirmed assistance/braking, Djay/MIDI/LED coexistence,
  Stop/Quit neutral, saved-setting restore while stopped, and explicit restart
  after USB reconnect, October 20, 2026. See
  [normal-app integration](jog-tension-integration.md).
- [x] **Mixer ON / Neural Mix EQ selection** - Filter is now a one-shot
  EQ deck-selection latch, not reset-to-neutral. The next ON toggles that deck's
  native EQ mode without retargeting Instant FX; normal ON still selects the
  FX target. White/cyan mode, armed brightness, active-effect color, and dim
  cyan idle Filter replace the former deck colors/reset indication.
  Live MIDI feedback was captured independently for A-D; the user accepted
  the physical behavior and idle refinement on October 20, 2026.
  The exhaustive [edge-case check](neural-mix-eq-check.md) remains available
  and is not claimed complete.

- [x] **Default Djay color-token correspondence** - All eight selectable
  hotcue colors physically matched the user's saved correspondence,
  October 6, 2026. Rust and bundled JSON defaults now match it; saved
  preferences are preserved. White is not an option in Djay's hotcue picker:
  token 9 retains its existing unverified white fallback.
  See [palette evidence](led-feedback.md#verified-hotcue-palette).

- [x] **LOOP/MOVE layout and beat jump** - Correct physical encoder pairing;
  independent per-deck loop/jump sizes; MOVE press toggles jump-size selection
  with a visible selector pulse; Shift+MOVE jumps one beat. Implemented and
  user-tested, October 6, 2026.
- [x] **Reloop** - Shift+LOOP press restores the stored loop range; manual
  loop in/out remains unmapped. User confirmed "It works," October 6, 2026.
- [x] **Momentary REV** - Reverse ends on release, retaining original deck
  ownership across deck/Shift changes and bridge Stop. Slip is independent.
  User confirmed the three-fix build tests good, October 6, 2026.
- [x] **Browse direction** - Correct direction on both sides and both
  modifier layers, with one mapping inversion. User confirmed the three-fix
  build tests good, October 6, 2026.
- [x] **Crossfader startup state** - Apply the latest physical assignments
  and curve after Djay playback feedback, then emit physical changes only.
  User confirmed the three-fix build tests good, October 6, 2026.
  Mapping reconnect/reselection still requires bridge Stop/Start.
- [x] **Shift+Sync tempo-range cycling** - Either side cycles Djay's global
  tempo-fader range for all decks via `application.tempoSliderRangeNext`;
  normal Sync remains deck-specific. Djay owns the choices and cycling order.
  Software routing/export checks passed and the user confirmed "Works,"
  October 6, 2026. See [validation](shift-sync-validation.md).

## Open

- [ ] **Shift+CUE: jump to beginning** - Implemented October 21, 2026:
  `turntableN.gotoZero` on Note 1, channels 9-12. Normal CUE is unchanged.
  Generated mapping and ownership regressions cover it; physical acceptance
  remains pending.
- [ ] **Swap A#-1 / B-1 actions** - Implemented October 21, 2026 on input
  endpoint S4 MK3 MIDI Full, Note 10/11, channels 1-4: Playlist now
  marks/unmarks selected songs; Star focuses the Queue. Generated-mapping
  regression covers every deck; physical acceptance remains pending.
- [ ] **Practice-session freshness exit** - A 50 ms motor-input timeout
  stopped the entire full-profile bridge during practice on October 20, 2026.
  The beta.3 fix clears force, discards old movement, and automatically
  resumes tension after a fresh baseline and newly qualified motion while
  keeping control/feedback service running. The underlying input delay
  is unconfirmed and physical recovery testing remains pending.
  See [timeout recovery](tension-timeout-recovery.md).
- [ ] **Jog feel and backspin** - Reduce excessive slow-movement sensitivity
  while improving fast-backspin response. Measure raw counts,
  emitted MIDI, and Djay response before choosing tuning behavior.
  [October 6 comparison](jog-wheel-investigation.md): Traktor's target is
  2.5 beats per revolution at either speed, including released backspin,
  with no reversal after the platter stops. The user calibrated scratch
  Speed/Reaction to 2.7% / 150% and pitch bend to 2.7% / 17%; these are now
  configurable exported defaults in the app's Jog tab.
  Backspin still ends on touch release, especially at Reaction 50%.
  Released-spin routing and final stop-response acceptance remain unresolved;
  the separate tension setting is now available, but equal-force flicks are
  not equivalent across the two setups.
- [ ] **Leftmost FX button duplicates slot 1** - Verify native bank-enable
  behavior and investigate a whole-bank bypass that preserves individual
  slot states.
- [ ] **REC / Record Sample** - The user reports on October 21, 2026 that
  the action did not record a sample in their testing. Its actual behavior
  remains unknown; establish what it does before choosing a useful REC
  assignment rather than assuming it supports sample recording.
- [ ] **One-handed held preview** - Implemented October 21, 2026: pressing
  Preview opens/starts the preview player, releasing it closes/stops preview,
  and BROWSER scrubs while Preview is held, without Shift. Normal BROWSER
  browsing resumes on release. Use paired preview toggles on press/release;
  the user accepts occasional state mismatch if preview was already active
  or changes externally, with closing preview using the mouse as sufficient
  recovery. Preview-state feedback is not required for this design.
  Duplicate snapshots, simultaneous press/turn, deck/Shift changes,
  overlapping holds, release, and shutdown are regression-covered.
  Physical acceptance of this replacement remains pending.
- [ ] **EXT / microphone / line inputs** - Establish S4 input channels and
  hardware source switching, then determine which routing Djay can support.

## Deferred ideas

- [ ] **MUTE button role** - Deferred by the user on October 21, 2026.
  No distinct assignment chosen; revisit after screen support is understood,
  when layered controls may provide a useful role. This defers the design
  decision, not a change to the current STEMS-duplicate behavior.
- [ ] **Powered platter rotation and other haptics** - Playback-driven
  rotation, cue effects, and other haptic behavior need their own agreed scope.
  The completed tension setting does not implement those behaviors.
- [ ] **Jog-ring position polish** - Hold the current chase position on top
  touch or playback stop, follow physical rotation, and resume smoothly.
  Deferred by the user on October 20, 2026; not implemented. Physical rotation
  scale still needs measurement; the LED's 2880-position encoding is not proof
  of sensor counts per revolution.

## DJ-laptop validation

Broader testing of this session's changes on the actual DJ laptop is in
progress. On October 20, 2026, an initial launch reported that
`default-tension-config.json` could not be opened. The installed resource was
present, signature verification reported no error, and the same app then
launched normally without changes. The cause is unconfirmed; no code fix or
rebuild was made for that report.

On-device screens/pad legends and a controller settings-menu gesture remain
deferred. Automatic Slip during REV is a separate design choice, not part of
the completed momentary-reverse fix. See the mapping audit for these ideas
and the Encdr upgrade candidate.
