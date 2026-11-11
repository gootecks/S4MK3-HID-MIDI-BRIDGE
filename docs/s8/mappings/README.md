---
summary: "Frozen copies of Djay mappings that were checked on the S8, with install steps."
type: reference
---

# Working Kontrol S8 mappings

Frozen copies of Djay mappings that were checked on the S8. They keep a
known-working layout available when the catalog and the generator change.
They are not regenerated, and no test compares them with the generator.

| File                                              | Source                                                                                                                                                                                   | Checked                                                                                                                                                    |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Kontrol S8 Bridge r6 2026-11-10.djayMidiMapping` | Byte-identical to the `--device s8 --generate-mapping` output at commit `56b4a0c`. 517 controls, endpoint `Kontrol S8 MIDI Full`. Installed during the checks as `Kontrol S8 Bridge r6`. | Attended checks of 2026-11-10 ([translator-validation.md](../translator-validation.md)). Open: the touchstrip's fine motion, CAPTURE and the booth output. |

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
