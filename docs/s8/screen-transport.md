# S8 screen transport channel

How the macOS app and the bridge exchange S8 screen frames and control
events over the bridge's standard streams (#44, #51). The Swift side codes
against this file.

Status: the code and its fake-transport tests are written; **nothing here
has been run against a device**. The last section lists what only hardware
can confirm.

## Flags

Both flags are for the Kontrol S8 only and need `--midi full`. The S4 MK3
refuses them with a clear message, and the S8 still refuses the S4 screen
flags `--screens` and `--screen-control-stdin`.

| Flag                 | Direction            | Content              |
| -------------------- | -------------------- | -------------------- |
| `--app-events`       | bridge → app, stdout | `S8_CONTROL` lines   |
| `--s8-screens-stdin` | app → bridge, stdin  | `S8FR` frame records |

`--s8-screens-stdin` also writes `S8_SCREEN_STATUS` lines on stdout.
Every line starts at column 0 and ends with `\n`. Bridge log output may
share stdout; a reader MUST ignore lines with other prefixes.

## Control events (`--app-events`)

Added by #51; this section is its specification.

```text
S8_CONTROL display_button <left|right> <2|3|5|6|7|8> <down|up>
S8_CONTROL screen_encoder <left|right> <1-4> <signed delta in counts>
```

- Encdr names `{left|right}_screen_left_{1-4}` are display buttons 1–4 and
  `{left|right}_screen_right_{1-4}` are buttons 5–8.
- Buttons 1 and 4 belong to Native Instruments (colour settings, Controller
  Editor template switch) and are never emitted.
- Encoder deltas are in counts, 1000 per turn, clockwise positive.
- The first report of each report ID after start records resting state only
  and emits nothing.

## Frame records (`--s8-screens-stdin`)

The app writes records to the bridge's stdin. Records are back to back with
no padding. A record is a 12-byte header followed by the payload.

| Offset | Size | Field                                              |
| -----: | ---: | -------------------------------------------------- |
|      0 |    4 | ASCII `S8FR` (`53 38 46 52`)                       |
|      4 |    1 | panel: `0` left, `1` right                         |
|      5 |    3 | zero                                               |
|      8 |    4 | payload length, `u32` little-endian, `261120`      |
|     12 |261120| 480 × 272 pixels, RGB565 big-endian, row-major from the top left |

Pixels are two bytes each (`PIXEL_BYTES` = 480 × 272 × 2 = 261 120). The
bridge adds the framing itself (header, the full-panel literal
command `00 00 FF 00`, footer); the app never sends those bytes. Each frame
is one complete panel image: the channel has no partial updates.

### What the bridge sends to the device

For each accepted record the bridge sends one USB bulk transfer of
`FRAME_BYTES` = 261 148 bytes to interface 6, endpoint `0x04`, built by
`screen::full`: frame header, command `00 00 FF 00`, the pixels, footer.
Full-panel literal frames are hardware-validated
([screen-frame-validation.md](screen-frame-validation.md)).

### Stream rules

- EOF between records is a clean stop. The bridge stops the same way it does
  on Ctrl-C or the time limit: exit frames, release, `inactive`, exit.
- The app MUST keep stdin open for as long as it wants the bridge to run.
  Closing stdin stops the whole bridge, MIDI included, as EOF does on the
  S4 `--screen-control-stdin` channel.
- Reads may arrive in any size; records may be split across writes.
- Any of these is a protocol error: wrong magic, panel other than 0 or 1,
  non-zero bytes 5–7, a length other than `261120`, EOF inside a record
  (header or payload), or a read error. The stream cannot be resynchronised.
  The bridge then reports `S8_SCREEN_STATUS failed <reason>`, stops the
  screen path (exit frames to the panels that got a frame, release) and
  keeps MIDI running. It reads and discards the rest of stdin so the writer
  never blocks, and EOF still stops the bridge.

## Status lines

```text
S8_SCREEN_STATUS pending
S8_SCREEN_STATUS active
S8_SCREEN_STATUS inactive
S8_SCREEN_STATUS failed <reason>
```

`<reason>` is one line (control characters are replaced by spaces).

| State      | Meaning                                                        |
| ---------- | -------------------------------------------------------------- |
| `pending`  | Flag set, nothing claimed. Printed once at start.              |
| `active`   | The device accepted the first frame. Printed once.             |
| `inactive` | Exit frames sent and the interface released cleanly.           |
| `failed`   | A claim or transfer failed, or a protocol error. Final state.  |

Order: `pending`, then `active` once the first transfer succeeds, then
exactly one of `inactive` or `failed`. A failure before the first frame goes
`pending` → `failed`; a clean stop before any frame goes `pending` →
`inactive` (nothing was claimed, so nothing is sent). `failed` and
`inactive` are final: there is no retry and no re-arming in the same bridge
process, and frames arriving afterwards are discarded. To retry, restart the
bridge.

## Worker behaviour

Placement: `src/device/s8/screen/{wire,worker,usb}.rs` in the library next
to the frame encoder, so the stdin parser and the worker are testable
without the binary; `src/s8/screens.rs` in the binary holds the stdout
status lines, the stdin thread and the flag check.

- **Claim once.** Interface 6 is claimed when the first frame is due and
  held until stop. Nothing is claimed while the app sends nothing.
- **Latest wins.** Each panel has one slot. A newer record for a panel
  replaces an unsent older one. The reader thread only copies into the slot,
  so a slow transfer never blocks stdin reading, the HID loop or MIDI.
- **One panel does not resend the other.** A transfer happens only for a
  panel that has a new, unsent frame. An unchanged panel is never repeated.
- **Pacing.** The start of two transfers to the same panel are at least
  `FRAME_INTERVAL` apart: 30 frames per second per panel
  (`MAX_FRAMES_PER_SECOND`). **Provisional**: no S8 refresh rate has been
  measured; phase 5 (#43) replaces it. Frames dropped by pacing are the
  older ones of the slot. When both panels are ready they alternate.
- **One transfer per frame**, 500 ms timeout, no retry. A failed or short
  transfer, or a failed claim, reports `failed <reason>` and releases the
  interface. A failed transfer sends no exit frames; the claim is released.
- **Exit frames.** On a clean stop and after a protocol error, the bridge
  sends `screen::solid` black (`00 00 00 00` pixel pairs) to each panel that
  received a frame, then releases the interface. **Provisional**: #27
  decides what the panels show when the bridge exits. The frames equal the
  black frames in the validation record.
- **NIHardwareAgent.** The bridge neither looks for nor stops
  NIHardwareAgent. If the agent holds interface 6 the claim fails and the
  bridge reports `failed <reason>` while MIDI keeps running. Stopping the
  agent is the caller's job, as for the rest of the full S8 bridge (#26,
  [agent-stop-validation.md](agent-stop-validation.md)).

## Validation state

Hardware-validated (from the bridge's example tools, not from this channel):

- Claiming interface 6 with the agent stopped.
- Full-panel literal and repeat frames on both panels, RGB565 big-endian,
  no init report needed.

Covered by tests with a fake transport only:

- Record parsing, split reads, every protocol error, stream → worker →
  transport bytes for both panels, latest-wins under a blocked transfer,
  pacing, status order, claim and transfer failure, exit frames.

Not validated on hardware (only a device can confirm):

- Holding interface 6 across many frames, and releasing it afterwards.
- Any refresh rate, in particular that 30 frames per second per panel is
  sustainable and visually smooth (#43).
- Both panels updated back to back within one bridge process.
- The claim failure when NIHardwareAgent holds interface 6, and that the
  reason text is useful.
- What the panels show after the exit frames, and after a `failed` stop (#27).
- The 500 ms transfer timeout under a stalled device.
- stdin throughput: about 15.7 MB per second for two panels at the cap.
