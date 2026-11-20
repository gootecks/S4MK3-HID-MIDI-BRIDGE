---
summary: "Frozen copies of Djay mappings that were checked on the S8, with install steps."
type: reference
issues: []
---

# Working Kontrol S8 mappings

Frozen copies of Djay mappings that were checked on the S8. They keep a
known-working layout available when the catalog and the generator change.
They are not regenerated, and no test compares them with the generator.

| File                                                   | Source                                                                                                                                                                                                     | Checked                                                                                                                                                    |
| ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Kontrol S8 Bridge r6 2026-11-10.djayMidiMapping`      | Byte-identical to the `--device s8 --generate-mapping` output at commit `56b4a0c`. 517 controls, endpoint `Kontrol S8 MIDI Full`. Installed during the checks as `Kontrol S8 Bridge r6`.                   | Attended checks of 2026-11-10 ([translator-validation.md](../translator-validation.md)). Open: the touchstrip's fine motion, CAPTURE and the booth output. |
| `Kontrol S8 Bridge r8 2026-11-11.djayMidiMapping`      | Byte-identical to the `--device s8 --generate-mapping` output at commit `d49a291`. Adds `userInfo` pad colours and `control` type on `songProgress`. Installed for LED run 2 as `Kontrol S8 Bridge r8`.    | LED run 2 of 2026-11-11 ([led-feedback-validation.md](../led-feedback-validation.md#second-run)). Open: L9, L10, L15, LEDs dim after the stop (#125).      |
| `Kontrol S8 Bridge r9 2026-11-20.djayMidiMapping`      | Byte-identical to the `--device s8 --generate-mapping` output at commit `92a035a`. Adds `mixer.monitorActive1-4` outputs on channel 5, notes 0-3. Installed for the CUE LED run as `Kontrol S8 Bridge r9`. | CUE LED bridge run of 2026-11-20 ([cue-led-check.md](../cue-led-check.md#bridge-run-2026-11-20)).                                                          |
| `Kontrol S8 Bridge Evan r4 2026-11-20.djayMidiMapping` | Byte-identical to the `--device s8 --s8-layout evan --generate-mapping` output at commit `0be85e1`. Installed for the third Evan run as `Kontrol S8 Bridge Evan r4`.                                       | Evan attended runs of 2026-11-20 ([evan-layout-validation.md](../evan-layout-validation.md#third-attended-run-2026-11-20)).                                |

A copy matches the bridge built from its source commit. A later bridge can
send MIDI addresses that an older copy does not assign; [mapping.md](../mapping.md)
lists the current addresses.

## Install

1. Copy the file into Djay's mapping folder. Djay must be restarted to see it.

       cp "docs/s8/mappings/Kontrol S8 Bridge r6 2026-11-10.djayMidiMapping" \
           ~/Music/djay/"MIDI Mappings"/

2. Start the bridge with `mise run s8:midi`. This stops NIHardwareAgent, runs
   the bridge until Ctrl-C and restores the agent.
3. In Djay's MIDI settings, choose `Kontrol S8 MIDI Full` and this mapping.

## Adding a revision

- Copy the file that was installed in Djay for the check; do not edit an
  existing copy.
- Name it `Kontrol S8 Bridge rN YYYY-MM-DD.djayMidiMapping`, with the
  revision number used during the check and the date of the check.
- Add a row to the table with the source commit and the validation record.
