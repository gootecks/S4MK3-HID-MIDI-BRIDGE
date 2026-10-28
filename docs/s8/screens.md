# S8 screens

Research and first-frame test plan for #26. Nothing in this file has been
sent to an S8 from this bridge yet.

## What we know

Sources: [kontrol-s8-protocol](https://github.com/gusgustavodj/kontrol-s8-protocol)
(`docs/overview.md` §10, `data/display-payloads.json`,
`data/DISPLAY_INITIALIZATION.csv`; CC BY 4.0), Encdr's S8 descriptor and
hardware docs, and the S4 MK3 screen code in `src/screens.rs`. Evidence for
S8 displays is Windows (NI driver) and Android (experimental) only. No source
shows a working macOS path.

| Item | Value | Confidence |
| ---- | ----- | ---------- |
| Transport | Interface 6 (`FF/BD/00`), bulk OUT `0x04`, max packet 512 | Both sources agree |
| Geometry | Two 480×272 panels; screen 0 left, screen 1 right | Both agree |
| Header | `84 00 SS 60 00 00 00 00 00 00 00 00 01 E0 01 10` (16 bytes, `SS` = screen) | Observed payloads |
| Body | Commands `OP 00 CNT_HI CNT_LO`, counts in pixel pairs, total `0xFF00` | Observed payloads |
| Opcodes | `00` LITERAL (+4×CNT bytes), `01` REPEAT (+4 bytes), `02` SKIP | Observed payloads |
| Footer | `03 00 00 00 40 00 SS 00` | Both agree |
| Pixels | 16-bit big-endian, nominally RGB565 | Red/blue order unresolved |
| Init | Optional feature SET_REPORT `0xF8` on interface 5, `E0 01 10 01 05 60 00 64 32 00`; byte 8 seen as `0x14`, `0x50`, `0x64` | Windows only; meaning and necessity unknown |
| Shutdown, recovery, max frame size, refresh rate | Unknown | Not demonstrated |

### Reconciling Encdr and the protocol repo

Encdr describes a "raw" 20-byte header ending `00 00 FF 00`, while the
protocol repo describes run-length commands after a 16-byte header. They
agree: `00 00 FF 00` is a single LITERAL command covering all `0xFF00` pairs.
A full uncompressed frame is therefore 16 + 4 + 261,120 + 8 = 261,148 bytes.
This is a derivation from both documents, not a tested fact.

Encdr is wrong or stale in three places:

- Its screen reference doc puts the panels on interfaces 1 and 2, which are
  audio.
- Its `0x66`/`0x67` screen IDs at byte 3 are an old hypothesis; byte 2 holds
  the screen and byte 3 is `0x60`.
- It states BGR565 as fact; the protocol repo leaves the order open.

### Differences from the S4 MK3 path

`src/screens.rs` sends 320×240 raw pixels on interface 4, endpoint `0x03`,
header byte 3 `0x21`, a 4-byte footer, and no commands. The S8 needs its own
packet builder. The raster, text and layout code (`Frame`, `rect`, `text`)
can be reused once it takes the panel size as a parameter.

### macOS ownership

With the S8 in MIDI mode, `NIHardwareAgent` holds a user client on interface 6
(see [input-capture.md](input-capture.md)). It draws Controller Editor labels
on the panels. A direct claim of interface 6 needs that client gone. The S4
app does this by force-quitting the agent (SIGKILL, because a clean NI shutdown
blanks S4 panels) and relaunching it afterwards.

Phase 0 settled whether the S8 keeps sending Evan's MIDI when the agent is
gone ([agent-stop-validation.md](agent-stop-validation.md)): it does not. The
agent turns HID input into MIDI on its virtual `Traktor Kontrol S8 Input`
port; the controller's own MIDI port stays silent. With the agent stopped,
HID input keeps flowing, interface 6 is free and the panels keep the last
labels, but Djay gets no MIDI. `open -g` restores everything.

So the hybrid setup is out. Two routes remain:

- **Full bridge control:** stop the agent, claim interface 6 and translate
  every S8 control from HID with the bridge's own Djay mapping (#13, #14), as
  the S4 app does.
- **NI-service screens:** keep the agent and Evan's mapping, and send frames
  through NI's own service. Encdr v0.8.4 has such a route (CFMessagePort to
  `NIHostIntegrationAgent`), used when interface 6 cannot be claimed. There
  is no macOS evidence yet that it works for the S8.

**Decision (#26): full bridge control.** The bridge stops the agent, claims
interface 6 and translates every S8 control from HID with its own Djay
mapping, as the S4 app does. Evan's layout and ideas carry over into the S8
catalog (#13, #14); his Controller Editor template and mapping file do not.
This removes the dependency on Controller Editor and NI's agent, which no
longer receive S8 support. The NI-service route is not pursued.

## First-frame test plan

Each phase needs separate approval. Stop at the first unexpected result and
record it here before going further.

### Common rules

- Interfaces touched: 5 (read; optional `0xF8` in phase 3 only) and 6. Never
  interface 4 (DFU), never a configuration change, reset or firmware request.
- One frame per approved step, each a single bulk transfer with a 500 ms
  timeout. No retries, no loops, no partial frames, no SKIP commands.
- The passive recorder runs throughout so HID and MIDI side effects are
  logged.
- Djay stays open but no track is playing.
- Stop conditions: any transfer error, timeout or short write; the S8
  disconnecting or its controls freezing; unexpected LED or audio changes.
  On a stop, release the interface and go to recovery.

### Recovery

1. Release interface 6 and quit the test tool.
2. Relaunch `NIHardwareAgent` (`open` on its app bundle under
   `/Library/Application Support/Native Instruments/Hardware/`), as the S4
   app does.
3. Check that the displays show Controller Editor labels again and that left
   PLAY sends ch 1 CC 10.
4. If not, power-cycle the S8 and check again.

### Phase 0: agent stop (no S8 output)

1. Force-quit `NIHardwareAgent` with SIGKILL.
2. Observe: panel contents, whether interface 6 has no client in `ioreg`,
   and whether left PLAY, a CUE button and a pad still send MIDI.
3. Wait 30 seconds to check whether launchd or Controller Editor relaunches
   the agent.
4. Run recovery.

Result: MIDI does not survive. Full bridge control or the NI-service route,
chosen in #26, decides whether phase 1 runs.

### Phase 1: claim only

Claim interface 6 with `nusb` (the same way `UsbScreens::open` claims the S4
interface), hold for 5 seconds, release. No transfers. Observe that HID
still flows; MIDI is already gone while the agent is stopped.

`examples/s8-screen-claim.rs` does exactly this and nothing else: it refuses
to run while NIHardwareAgent is running, opens the one connected S8, claims
interface 6, holds it for 5 seconds and releases it. `mise run
s8:screen-claim` runs it during an attended agent stop with the passive
recorder.

Result: the claim succeeds and disturbs nothing; HID keeps flowing and the
agent takes interface 6 back on relaunch
([screen-claim-validation.md](screen-claim-validation.md)).

### Phase 2: first frames

Left panel only, one frame each:

1. The observed `BLACK_0` payload, byte for byte (32 bytes):
   `84 00 00 60 00 00 00 00 00 00 00 00 01 E0 01 10 01 00 FF 00 00 00 00 00 03 00 00 00 40 00 00 00`.
   Expected: the left panel goes black.
2. A solid colour built from the same frame, REPEAT pair `F8 00 F8 00`
   (32 bytes). The panel turns red if pixels are RGB565, blue if BGR565.
   This settles the colour order.
3. The same two frames on the right panel (screen byte and footer byte
   `01`).

### Phase 3: only if phase 2 shows nothing

Send the `0xF8` feature report once, then repeat phase 2 step 1. This is
the only interface 5 output in the plan.

### Phase 4: full frame

One full LITERAL frame (261,148 bytes) with a test pattern: corner markers
and labelled colour bars, like `screens::test_pattern` for the S4. This tests
whether the S8 accepts uncompressed frames, which would let the bridge reuse
the S4 rendering with a simpler encoder.

### What to record

For each step: the exact bytes or a hash for full frames, transfer status,
panel photo, whether MIDI and HID still flowed, and what happened on recovery.
Record in a validation file under `docs/s8/`, not in this plan.

## Still open after this plan

- What the S8 shows when the bridge stops, and whether the agent redraws
  its labels after relaunch.
- Refresh rate and latency for partial updates.
- What Djay can actually feed the panels: only MIDI feedback, as on the S4
  (mode legends and indicators, no waveforms or track titles).
