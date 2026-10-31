# S8 screens

Research and first-frame test plan for #26, with results. Phases 2 and 4
sent frames to the S8 from this bridge's tools
([screen-frame-validation.md](screen-frame-validation.md)).

## What we know

Sources: [kontrol-s8-protocol](https://github.com/gusgustavodj/kontrol-s8-protocol)
(`docs/overview.md` §10, `data/display-payloads.json`,
`data/DISPLAY_INITIALIZATION.csv`; CC BY 4.0), Encdr's S8 descriptor and
hardware docs, and the S4 MK3 screen code in `src/screens.rs`. Evidence for
S8 displays is Windows (NI driver) and Android (experimental) only. No source
shows a working macOS path.

| Item | Value | Confidence |
| ---- | ----- | ---------- |
| Transport | Interface 6 (`FF/BD/00`), bulk OUT `0x04`, max packet 512 | Both sources agree; works on macOS (phases 2, 4) |
| Geometry | Two 480×272 panels; screen 0 left, screen 1 right | Confirmed on macOS |
| Header | `84 00 SS 60 00 00 00 00 00 00 00 00 01 E0 01 10` (16 bytes, `SS` = screen) | Observed payloads; works on macOS |
| Body | Commands `OP 00 CNT_HI CNT_LO`, counts in pixel pairs, total `0xFF00` | Observed payloads; LITERAL and REPEAT work on macOS |
| Opcodes | `00` LITERAL (+4×CNT bytes), `01` REPEAT (+4 bytes), `02` SKIP | Observed payloads; SKIP untested |
| Footer | `03 00 00 00 40 00 SS 00` | Both agree; works on macOS |
| Pixels | 16-bit big-endian RGB565 | Confirmed on macOS (phases 2, 4) |
| Init | Optional feature SET_REPORT `0xF8` on interface 5, `E0 01 10 01 05 60 00 64 32 00`; byte 8 seen as `0x14`, `0x50`, `0x64` | Windows only; not needed on macOS |
| Shutdown, max frame size, refresh rate | Unknown; a full 261,148-byte frame works | Not demonstrated |

### Reconciling Encdr and the protocol repo

Encdr describes a "raw" 20-byte header ending `00 00 FF 00`, while the
protocol repo describes run-length commands after a 16-byte header. They
agree: `00 00 FF 00` is a single LITERAL command covering all `0xFF00` pairs.
A full uncompressed frame is therefore 16 + 4 + 261,120 + 8 = 261,148 bytes.
Phase 4 confirmed that the S8 displays it.

Encdr is wrong or stale in three places:

- Its screen reference doc puts the panels on interfaces 1 and 2, which are
  audio.
- Its `0x66`/`0x67` screen IDs at byte 3 are an old hypothesis; byte 2 holds
  the screen and byte 3 is `0x60`.
- It states BGR565; phases 2 and 4 showed RGB565.

### Differences from the S4 MK3 path

`src/screens.rs` sends 320×240 raw pixels on interface 4, endpoint `0x03`,
header byte 3 `0x21`, a 4-byte footer, and no commands. The S8 needs its own
packet builder (#44). Its pages are rendered by the app, not by the S4
raster and text code (see [Page content and rendering](#page-content-and-rendering)).

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

## Page content and rendering

The S4 pages show only what Djay sends as MIDI feedback. The S8 panels can
show more, read from Djay's own data. Library views come first: the laptop
already shows waveforms, while some library views are hard to reach in Djay
or do not exist there.

**Decision: the app renders, the bridge transports.** The menu-bar app builds
each 480×272 panel offscreen at 1x, converts it to RGB565 big-endian and hands
frames to the Rust child. The bridge owns interface 6 and paces transfers
(#44). Views on the panels: #27.

### Library views first

Views: playlists, tracks related to the one on a deck, tracks played before
or after it, and Djay's queue. The display buttons and screen encoders
navigate them (#51); a 960×272 window prototype comes first (#50).

| Source | Data | Limits |
| ------ | ---- | ------ |
| `MediaLibrary.db` snapshot (db, `-wal`, `-shm` together), read-only (#49) | Tracks, playlists and items, queue, history sessions and plays | YapDatabase with TSAF objects; the decoder exists in djaykit (Python) and needs a Swift port |
| djaykit related index, exported between sets (prime-optimal/djaykit#81) | Related tracks per track: co-play transitions, harmonic, BPM and artist scores; before/after counts | Built from play history; the views must work without it |

Acting on a chosen track is the open problem (#48). Djay's MIDI load and
Add to Queue act only on Djay's selected row, and its MIDI feedback carries no
library state, so the app needs Accessibility to find and select the track.
Writing Djay's own Related links from the same index is djaykit work
(prime-optimal/djaykit#80, #82), with Djay quit.

### Deck and waveform pages later

An earlier prototype, `djay-phrase-tool` (MIT, built on
[kyleawayan/djay-pro-bridge](https://github.com/kyleawayan/djay-pro-bridge)),
reads Djay's decks and lays out two 480×272 surfaces in a window (#45). One
use: decks 3 and 4 while Djay shows its two-deck layout.

| Source | Data | Limits |
| ------ | ---- | ------ |
| Accessibility, Djay's window (about 8 polls/s) | Title, artist, BPM, key, elapsed/remaining, playing | View-dependent: time can vanish with the library expanded; BPM has one decimal |
| `MediaLibrary.db` and the analysis cache, read-only | Overview waveform envelope, 3-band samples, beats, bars, cue points | About 2–11 waveform samples/s: an overview, not a zoomed scrolling waveform; key in the blob undecoded |

Its layout is the starting point: a header with title, artist, BPM, key and
time; an overview waveform; a phrase, next-cue and section row.

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

`examples/s8-screen-frame.rs` builds and sends one frame per run (`black`,
`f800` or `pattern`, `--screen left|right`) and refuses to run while
NIHardwareAgent is running. `mise run s8:screen-frame first` runs the four
steps during an attended agent stop with the passive recorder.

Result: all four frames display as expected, and `F8 00` shows red, so the
pixels are RGB565 ([screen-frame-validation.md](screen-frame-validation.md)).

### Phase 3: only if phase 2 shows nothing

Send the `0xF8` feature report once, then repeat phase 2 step 1. This is
the only interface 5 output in the plan.

Result: not run; phase 2 frames displayed without it.

### Phase 4: full frame

One full LITERAL frame (261,148 bytes) with a test pattern: corner markers
and labelled colour bars, like `screens::test_pattern` for the S4. This tests
whether the S8 accepts uncompressed frames, which would let the bridge reuse
the S4 rendering with a simpler encoder.

`mise run s8:screen-frame pattern rgb` sends it to the left panel, then the
right.

Result: the S8 accepts uncompressed frames. The pattern displays exactly as
intended on both panels, each transfer taking about 16 ms. The last frame
stays after the interface is released, and the relaunched agent redraws its
labels ([screen-frame-validation.md](screen-frame-validation.md)).

### What to record

For each step: the exact bytes or a hash for full frames, transfer status,
panel photo, whether MIDI and HID still flowed, and what happened on recovery.
Record in a validation file under `docs/s8/`, not in this plan.

## Still open after this plan

- Refresh rate and latency for repeated frames, holding interface 6 across
  frames, and partial updates (SKIP, sub-rectangles): phase 5, #43.
- What the panels should show when the bridge quits without relaunching the
  agent: they keep the last frame (#27).
- How Djay's four-deck layouts appear through Accessibility (#45).
