---
summary: "Attended probe of the S8 mixer CUE LEDs through feature report 0xF4 0x26: plan C0-C5, recovery and the 2026-11-20 run."
type: procedure
issues: [134, 123]
---

# S8 mixer CUE LED check

Framing, risks and the other `0xF4` commands are in
[led-output-mixer.md](led-output-mixer.md#0xf4-cue-filter-and-thru).

## Plan

This is the hardware test plan for
[issue #134](https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE/issues/134).
Run it only with explicit approval to stop NIHardwareAgent and send the listed reports.
Connect exactly one S8. Quit any bridge and S8Host, and open Controller Editor with
the panel labels visible before stopping NIHardwareAgent. Leave Controller Editor open
and confirm the agent stays stopped during the test. Djay need not run.

Prebuild with `mise exec -- cargo build --locked --example s8-cue-probe` from the repo root.

`--mask` sends one 33-byte HID feature report on interface 5 in shared mode.
Each step lists the first three bytes; the remaining 30 bytes are zero.
The probe never sends `0x24` (THRU) or `0x25` (FILTER).

`--read-latch` alone sends nothing. It prints changes to the low nibble of wire byte 39
in report `01` (A=1, B=2, C=4, D=8). Record LEDs and latch separately.
No observed report means the latch is unknown, not zero.

1. **C0 — clear baseline.** Send `F4 26 00` plus 30 zero bytes:
   `mise exec -- cargo run --locked --example s8-cue-probe -- --mask 0x00 --read-latch 10`.
   Expect all four CUE LEDs dark. Are all four dark (yes/no)? Note the latch mask
   independently; it need not be zero.
2. **C1 — CUE A (Q30a).** Send `F4 26 01` plus 30 zero bytes:
   `mise exec -- cargo run --locked --example s8-cue-probe -- --mask 0x01 --read-latch 10`.
   Expect A lit and B–D dark. Is only A lit (yes/no)?
3. **C2 — all four (Q30b).** Send `F4 26 0F` plus 30 zero bytes:
   `mise exec -- cargo run --locked --example s8-cue-probe -- --mask 0x0F --read-latch 10`.
   Expect A–D lit. Are all four lit (yes/no)?
4. **C3 — physical CUE A after C2.** Send no bytes:
   `mise exec -- cargo run --locked --example s8-cue-probe -- --read-latch 20`.
   Wait for the initial latch mask, then press and release CUE A. Expect latch bit `01`
   to toggle. Does A's LED follow that latch rather than stay on (yes/no)? If the press
   toggled A's latch on while its LED was already lit, press and release A once more to
   observe the off transition. Also note whether any of the other CUE LEDs changed.
5. **C4 — clear, then physical CUE B.** Send `F4 26 00` plus 30 zero bytes:
   `mise exec -- cargo run --locked --example s8-cue-probe -- --mask 0x00 --read-latch 20`.
   Expect all four LEDs dark after the write. Wait for the initial latch mask, then
   press and release CUE B. If B's latch was already on, press and release it again
   to observe an off-to-on transition. When latch bit `02` turns on, does B light up
   again (yes/no)?
6. **C5 — retention after exit (Q32).** Send `F4 26 0F` plus 30 zero bytes:
   `mise exec -- cargo run --locked --example s8-cue-probe -- --mask 0x0F --read-latch 20`.
   Expect all four LEDs on before exit. Do not touch any controls. After the command
   returns naturally (no exit write), check immediately and again after 10 seconds.
   Are all four LEDs still lit at both checks (yes/no)? If they were not all lit
   before exit, mark retention inconclusive.

**Recovery:** After C5, or if aborting after a write, clear with `F4 26 00` plus 30
zero bytes using
`mise exec -- cargo run --locked --example s8-cue-probe -- --mask 0x00`.
Restore the agent even if the clear fails:
`open -g "/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app"`.
Confirm that Controller Editor's panel labels and normal CUE behavior return. If they do not,
power-cycle the S8 and check again.

## Run 2026-11-20

Run with the agent stopped and no bridge running. Probe output is in
`docs/s8/captures/cue-leds-20261120/`. User answers, per step:

- C0: all four dark. The latch read timed out: the S8 sends no button report
  until a control changes.
- C1: only A lit. C2: all four lit.
- C3: the first read timed out before the press; that late press turned A off.
  Rerun as C3b with `--mask 0x0F --read-latch 30`: the latch read `0x0F` right
  after the write, then `0x0E` after a press of A (A off) and `0x0F` after a
  second press (A on). The user: "it turned it off, then back on again. now all
  4 are on".
- C4: the latch read `0x00` after the write and `0x02` after a press of B; the
  user saw only B lit.
- C5 was not run: C1 and C2 already showed the state kept after the tool exits.
- Recovery: `F4 26 00` sent, NIHardwareAgent restored.

Finding: `F4 26` writes the S8's own CUE latch, not only the LEDs. The LEDs show
the latch, and a press toggles the written state. A host can therefore set the
latch to djay's cue state, and each press then keeps both in step.
