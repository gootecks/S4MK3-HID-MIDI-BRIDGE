# Startup positions, FX pickup and library navigation

These controls require 0.2.0-beta.1 and its matching generated Djay mapping.
They are not included in 0.1.1-beta.3 or older downloads.
For the complete daily workflow, start with the [user guide](user-guide.md).

## Setup

1. Stop any separate bridge process.
2. Use **Install Djay mapping...** in the updated app. Save under a new name;
   keep existing custom mappings.
3. Select that new configuration for **S4 MK3 MIDI Full** in Djay.
4. Start the app's full-controller bridge. If it was already running when you
   changed mappings, use **Stop bridge**, then **Start bridge**.
5. Enable S4 screens if you want the additional FX warning text.

## Startup channel positions

The bridge takes an initial physical analog snapshot. Once Djay returns mapping
feedback, all four channel volumes and their high/mid/low EQ values are set to
the physical positions, including faders at zero. Movement before Djay becomes
ready updates the cached positions. Startup synchronization happens once; later
unchanged snapshots do not overwrite mouse edits.

Gain, filter, crossfader position and tempo are not included. The existing
startup synchronization of crossfader assignment switches and curve is separate.
After mapping reconnection or reselection, restart the bridge to initialize a
new session; automatic mapping-reconnect detection is not available.

## Shared FX controls

Selecting a new FX target does not apply the old deck's knob positions. The
bridge waits for that deck's actual parameter feedback and blocks movement
until each control reaches or crosses its software position.

- FX buttons 1-3 blink for mismatched parameter knobs.
- The strip's main ON button blinks for dry/wet mismatch.
- Blink-on uses the target deck's color; blink-off is dark.
- After pickup, normal effect-enable feedback returns.
- Dry/wet controls three software slots. Each slot picks up independently;
  the warning clears when all three agree with the physical knob.
- Unknown software values are not treated as synchronized.

The optional screen header follows the affected logical deck, even when an FX
strip targets a deck on the other side. `C FX SYNC L13W` means deck C needs
pickup on left-strip knobs 1, 3 and dry/wet. `R` identifies the right strip;
a question mark after a control means its feedback is missing. The header
disappears after synchronization, restoring the ordinary Info or STEMS layout.

## Jump and library gestures

Shift+MOVE temporarily selects one beat, uses native relative jumping with the
encoder's full detent count, then restores the last confirmed jump size. No size
is guessed if feedback is absent or conflicting: the one-beat jump still runs,
but the previous size cannot be restored. Establish a confirmed JUMP value
before relying on restoration. The screen continues to show
actual Djay feedback. Install the matching mapping for the new size-selection
notes.

Djay can coalesce several separate zero-delay jump commands, including normal
unshifted MOVE. That existing native limitation is distinct from preserving the
selected jump size.

Star (numeric Note 11 / B-1 on channels 1-4) uses **Switch Library Table**.
Shift+BROWSER clockwise selects the next library source; counterclockwise
selects the previous source. Normal BROWSER still browses tracks.

Preview scrubbing requires holding **Preview**, not Shift. While Preview is
held, BROWSER scrubs even if Shift is also held; release Preview to stop.
Shift+BROWSER press retains Library Back.

## Musical key when loading or ejecting

KEY follows Djay's current key feedback. Replacing a track no longer lets a
loading notification erase a key that Djay already sent. Ejecting a track
shows `--` when Djay reports no key; `?` means feedback is unavailable or
unrecognized, not a guessed key.

This runtime fix does not change MIDI addresses. A mapping already providing
the current numeric screen feedback can stay selected. Older mappings still
need replacing for the screen/navigation/FX additions above. If Djay shows a
key but the latest bridge shows `?`, retain the track and report its deck,
title, Djay key and app Diagnostics. See the
[captured fix and physical checks](key-feedback-order-validation.md).
