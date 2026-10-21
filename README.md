# S4 MK3 HID-MIDI Bridge

Use the Native Instruments Traktor Kontrol S4 MK3 with Djay Pro on macOS
through a menu-bar app. The bridge translates controls and drives LED feedback;
Djay sends audio directly to the S4. No development tools are needed.

**Version 0.1.1-beta.3** is a beta with a custom Djay mapping, not a recreation
of every Traktor feature. Read the control differences and known limitations
below before using it for a set.

## Requirements

- An Apple Silicon Mac running macOS 12 or later, plus a macOS version supported
  by your installed Djay Pro. (Windows Support is planned!!)
- Djay Pro with **PRO access**, which is required for custom MIDI mappings.
- A Traktor Kontrol S4 MK3, its power adapter, and a USB connection to the Mac.

## Download and install

1. Open [GitHub Releases](../../releases) and find **0.1.1-beta.3**.
2. Download **`S4 MK3 Bridge-0.1.1-beta.3-arm64.dmg`**.
3. Open the disk image, drag **S4 MK3 Bridge** to **Applications**, then eject
   the disk image.
4. Open **S4 MK3 Bridge** from Applications.

The current portable build is **0.1.1-beta.3 (build 4)**. Its app, ZIP, and DMG
have been rebuilt and verified. See the
[release notes and validation status](docs/release-0.1.1-beta.3.md).

The app is ad-hoc signed, not Developer ID signed or notarized. macOS may block
the first launch because it can't verify the developer. After attempting to
open it, go to **System Settings > Privacy & Security** and choose **Open Anyway**,
then confirm **Open**. On macOS 12, use **System Preferences > Security & Privacy >
General** instead. Approve it only if you trust the download; there's no need
to disable Gatekeeper.

## First start, mapping, and audio

1. Connect and power on the S4. Run only one bridge instance.
2. Open the app's Settings from its menu-bar icon. Settings opens automatically
   on first launch; **Check controller** can help confirm the connection.
3. Select the **full-controller profile**, choose continuous operation, and
   click **Start bridge**.
4. Choose **Install Djay mapping…**, save the mapping, and accept Djay's import.
   If necessary, open the saved `.djayMidiMapping` file in Djay.
5. In Djay, enable **S4 MK3 MIDI Full** and select **S4 MK3 Bridge** as its
   mapping. Repeated imports may add a number to the name. Use the four-deck
   layout to access decks A, B, C, and D.
6. In Djay's audio settings, select the **S4's native audio device** and configure
   Main and Pre-cue outputs for the speakers and headphones. Audio goes directly
   between Djay and the S4, not through the bridge. Master, booth, headphone
   level, and headphone mix controls remain hardware-local.

**Expected THRU warning:** when a crossfader assignment switch is set to
**THRU**, Djay may show a warning triangle. Clicking it opens **"Crossfader
assignment changed"** and asks whether to restore the default assignment for
all decks. This is expected: THRU intentionally lets that channel bypass the
crossfader. This specific warning does not mean the bridge or mapping is
broken. Choose **Keep** to retain your switch assignments; **Restore** resets
them to Djay's defaults.

![Djay's warning triangle after changing crossfader assignments to THRU](docs/images/djay-crossfader-thru-indicator.png)

![Crossfader assignment changed dialog with Keep and Restore buttons](docs/images/djay-crossfader-thru-warning.png)

If Djay shows only a blank S4 configuration, restart Djay with the bridge
already running. After reconnecting Djay or reselecting its mapping, use
**Stop bridge**, then **Start bridge** to refresh crossfader assignment and curve
initialization.

When updating, install and select the mapping from the new app. Save it under
a new filename and keep any custom mappings under their existing names.
Restarting the bridge alone doesn't update Djay's mapping.

Closing Settings leaves the app running in the menu bar. Use **Stop bridge**
to stop control translation, or **Quit S4 MK3 Bridge** to quit.

![Bridge settings with profile, Start/Stop, and mapping installation controls](docs/images/bridge-settings.png)

_Bridge controls and mapping installation._

## Controls and differences from Traktor

The mapping covers four-deck transport, touch scratching and pitch bending,
mixing, EQ, headphone cue, loops, browsing, hotcues, samples, Neural Mix, and
effects. A/C and B/D select the deck controlled by each side. Software feedback
drives button and pad LEDs, channel meters, and jog-ring decoration.

These are intentional custom mappings. Djay's loop, Neural Mix, and effect
models differ from Traktor's, so the same labels don't always mean the same
thing.

| Control                            | Behavior in this bridge                                                                                                                                                                                                               |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **LOOP turn / press**              | Select loop size / toggle the loop. Loop size is independent of MOVE's jump length, unlike Traktor's shared length.                                                                                                                   |
| **MOVE turn**                      | Jump by the selected jump length, or move the active loop by that length. Each deck keeps its own loop and jump sizes.                                                                                                                |
| **MOVE press**                     | Toggle jump-size selection. Turn MOVE to change the size immediately; press again to exit. The selected deck's selector pulses while selecting. The mode survives deck switches, has no timeout, and clears when the bridge restarts. |
| **Shift+MOVE turn**                | Jump one beat per step in either direction, even during size selection, without changing the chosen jump length.                                                                                                                      |
| **Shift+LOOP turn / press**        | Move the loop region / reloop the stored loop range. Manual loop in/out isn't mapped. These replace Traktor's shifted key/quantize functions.                                                                                         |
| **REV**                            | Reverse only while held. Release ends reverse. It doesn't automatically enable Slip as Traktor enables Flux; use FLUX to control Slip independently.                                                                                  |
| **Sync / Shift+Sync**              | Normal Sync is deck-specific. Shift+Sync on either side cycles Djay's tempo-fader range **globally for all decks**, rather than locking one deck's tempo fader as in Traktor. Djay determines the ranges and their order.             |
| **Channel FX1 / FX2**              | Choose the deck controlled by the left / right FX strip. These are per-deck control targets, not Traktor's shared FX buses or audio-routing assignments.                                                                              |
| **FX strips**                      | Control three manual effects on the target deck; Shift with an effect button selects the next effect. See the bank-enable limitation below.                                                                                           |
| **Channel ON and mixer FX**        | ON selects the Instant FX target. ON is white for normal EQ, cyan for Neural Mix EQ, and brighter on the target deck. An active Instant FX overrides that deck's ON with its effect-button color. |
| **Filter, then channel ON**        | Filter flashes cyan awaiting a deck; the next ON toggles only that deck's Neural Mix EQ without changing the Instant FX target. Filter again cancels. Idle Filter is dim cyan; it no longer resets a filter. |
| **STEMS / MUTE**                   | Both select the same Neural Mix layer: four mute pads and four solo pads; Shift with pads 1-4 provides exclusive solo. MUTE isn't a held Traktor-style modifier.                                                                      |
| **REC**                            | Assigned to Djay's Record Sample action, but its behavior remains unresolved. Don't rely on it as Traktor's Pattern Recorder.                                                                                                         |
| **Shift+CUE**                      | Jump to the beginning of the song, not the saved start cue. Normal CUE is unchanged. |
| **Playlist / Star buttons**        | Playlist marks/unmarks selected songs; Star focuses the Queue. These swap Note 10 (A#-1) and Note 11 (B-1) actions on deck channels 1-4. |
| **Preview**                        | Hold to preview; turn BROWSER while holding to scrub, then release to stop. BROWSER otherwise browses, without requiring Shift. If preview gets out of sync with the hold, close it with the mouse. |

HOTCUE pads trigger eight hotcues; Shift clears them. SAMPLES pads play
samples; Shift stops them. A/C share sampler bank 1, and B/D share bank 2.
Neural Mix availability depends on the track and Djay support.

**Screens, powered playback rotation, and haptic cues aren't supported.**
JOG/TT select software wheel behavior, not powered platter rotation. The app
does provide motor-assisted wheel tension in the full-controller profile,
as described below. There are no on-device pad legends. Jog-ring animation
remains decorative, not track-position or physical-platter-position tracking.

## App settings

### LEDs and palette

Settings includes deck, stem, and hotcue colors; lamp brightness; loop and
tempo-center indicators; decorative jog-ring animation; and channel-meter
appearance. LED settings can be imported or exported.
Choose **Save and restart** to apply LED changes. A mapping export isn't needed.
Saved LED preferences are preserved across app updates.

All eight selectable Djay hotcue colors have been physically confirmed:
red, orange, blue, yellow, green, azalea, cyan, and purple. White isn't offered
in Djay's hotcue picker; its separate fallback hasn't been verified.

### Jog wheels

The **Jog > Hardware** section has a shared **Jog Wheel Tension** slider:

- **Assist (-100 to -1):** assists each wheel's own movement while its metal
  top is touched.
- **Neutral (0):** the first-install default.
- **Brake (1 to 100):** resists each wheel's movement, including after release.

Both wheels share the saved setting but respond independently. Channel 1's
fader remains a normal mixer control. This setting applies only to the
full-controller profile: use **Save preferences** before starting, or
**Save and restart** while running. It does **not** require a new Djay mapping.
Stop and Quit clear force; reopening restores the saved value without starting
the bridge. After a USB disconnect, reconnect and explicitly start the bridge.

Brief controller-input gaps no longer close the bridge. Tension clears while
readings are missing and returns automatically after fresh readings and new
wheel movement; Stop/Start is not required. The 50 ms cutoff and touch rules
remain in place. Physical timeout-recovery testing is still pending; see
[timeout recovery](docs/tension-timeout-recovery.md).

#### Djay scratch and pitch-bend response

The separate scratch and pitch-bend sections set Speed and Reaction for all
four decks. The calibrated defaults are:

| Action                   | Speed | Reaction |
| ------------------------ | ----: | -------: |
| Scratch while touching   |  2.7% |     150% |
| Pitch bend without touch |  2.7% |      17% |

Save your preferences, then use **Install Djay mapping…** to export a new
mapping and select it in Djay. Export uses the current editor values and can
run while the bridge is running. **Saving or restarting alone doesn't apply
jog mapping changes.** Keep existing custom mappings under their original names.

Speed and Reaction adjust Djay's response, independently of hardware tension.
Neither setting resolves released-backspin behavior.

![Jog settings showing Hardware Jog Wheel Tension and separate scratch and pitch-bend controls](docs/images/jog-settings.png)

_Jog settings at Neutral (0), with the calibrated scratch and pitch-bend defaults._

The menu-bar icon turns green while the bridge is running. **Diagnostics**
shows the app release and build number alongside the bridge log.

## Known issues and limitations

The [current issue checklist](docs/issues.md) is the source of truth. Older
test records describe earlier builds and may still list completed work as
pending. LOOP/MOVE, Reloop, momentary REV, Browse direction, crossfader startup,
Shift+Sync, and the eight selectable hotcue colors have been confirmed in
controller testing. Focused hardware-tension checks passed for both wheels,
Djay/LED coexistence, Stop/Quit, saved settings, and explicit restart after
reconnection. The user also accepted the Mixer ON/Neural Mix EQ behavior;
the exhaustive edge-case checklist and broader DJ-laptop testing remain open.

Open bugs and unresolved behavior:

- **Jog feel and backspin:** backspin still ends on touch release. Slow/fast
  response and final stop behavior need further work; the new tension setting
  isn't full Traktor jog parity.
- **FX bank enable:** the leftmost FX button duplicates slot 1. Whole-bank
  bypass that preserves individual slot states remains unresolved.
- **REC:** recording source, destination/sample slot, and press/hold behavior
  aren't established.
- **Preview scrubbing:** native forward/backward skips are verified, and
  Shift+BROWSER seeking is implemented. Physical encoder direction and
  multi-detent response still need a live check with the new app and mapping.
- **MUTE/STEMS:** the current alias works as described above, but a possible
  held modifier and final pad behavior are undecided.
- **EXT, microphone, and line inputs:** S4 input channels, hardware source
  switching, and supported Djay routing remain unresolved.
- **Mapping reconnect/reselection:** automatic detection isn't implemented.
  Stop and start the bridge after either event.
- **Jog-ring position:** the decorative chase continues while a platter is held
  and returns to 12 o'clock when playback stops. Touch/physical-rotation tracking
  and holding the last position are deferred.

Independent loop/jump sizes, global Shift+Sync, per-deck FX targeting, and REV
without automatic Slip are design choices, not outstanding fixes.

## Roadmap

- Refine jog feel and released backspin, and resolve the open FX, recording,
  preview, pad, and external-input questions.
- Consider jog-ring touch/rotation polish separately. Playback-driven motor
  rotation, other haptics, screens, pad legends, and an on-controller settings
  gesture remain deferred.
- Explore a standalone hardware bridge longer term, along with other DJ
  applications and simpler guest setup. These aren't features of this beta.

## Beta feedback

Report problems through the repository's Issues tab. Include the app version,
macOS and Djay versions, selected mapping name, deck and modifier/pad mode,
steps to reproduce, and expected versus actual behavior. For jog or LED
problems, a short video helps. Include relevant app **Diagnostics** output,
checking it for private information before sharing.

## Thanks

Thanks to RufusIbiza, maintainer of [Encdr](https://github.com/RufusIbiza/Encdr),
for the S4 MK3 protocol documentation and controller support this bridge builds on.

## Developer and reference documentation

- [macOS app, packaging, and developer checks](macos/README.md)
- [Full mapping layout and test records](docs/full-midi-test.md)
- [Settings and control tuning](docs/configurable-controls.md)
- [Current issue checklist](docs/issues.md)
- [Traktor comparison and historical evidence](docs/traktor-djay-comparison.md)
- [Djay action reference](docs/djay-action-reference.md)
- [Bridge function reference](docs/bridge-function-reference.md)
- [LED feedback](docs/led-feedback.md)
- [Standalone hardware research and board shortlist](docs/hardware-bridge-research.md)
- [Design history](docs/design.md)
