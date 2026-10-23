# Screen support and the Native Instruments driver

October 23, 2026. Local development build; not a new public release.
No Terminal commands or extra development tools are required on the DJ laptop.

## Use the new app

1. Stop any standalone bridge. Quit the older menu-bar app before replacing
   its Applications copy. The new packages are in
   `dist/Screens NI Handover Fix-20261023/`; the older app and packages
   remain untouched.
2. Copy that folder's `S4 MK3 Bridge.app` to Applications, or install it from
   the included DMG. Open the new app and select the full-controller profile.
   Keep your existing Info Screen mapping selected.
3. Open **Settings > Screens**. **Enable S4 screens** is the same preference
   as the checked item in the status-bar menu. It is on by default to preserve
   the previous app's behavior. Automatic NI driver stopping is off by default;
   restoring a driver this app stopped is on by default.
4. Start the bridge. If the NI driver is running, the app asks whether it may
   temporarily stop it. Choose **Enable screens**, **Continue without screens**,
   or **Cancel**. **Do this automatically in future** opts into automatic
   stopping only if you choose Enable screens. Close Traktor, Maschine and
   Komplete Kontrol before stopping their shared service.
5. If you continue without screens, MIDI, LEDs and saved jog tension still run.
   The screen setting remains checked, the status icon is red, and the menu
   says screens are blocked. Use **Enable screens - stop NI driver** to recover
   without restarting the bridge. The Screens tab exposes the same action.
6. Turning screens off while running clears/releases only the screens; MIDI,
   LEDs and jog service continue. Any owned NI restoration responsibility is
   retained until the whole bridge stops or the app quits; turning off screens
   does not restart NI's driver alongside an ongoing bridge session.
   With screen support off, ordinary startup and menu opening do not check or
   stop NI's driver.
7. Use **Stop bridge** or quit the app when finished. Automatic restoration
   applies only if this app stopped the NI driver and restoration is enabled.
   Save and restart keeps the driver stopped between bridge children.

The confirmation reads:

> The Native Instruments screen driver software is running in the background
> and will prevent the screens from functioning in this app. We can temporarily
> stop it and restore it when you quit.

Automatic restoration alone is not permission to stop the driver. Enabling
automatic stopping suppresses this confirmation on subsequent screen starts.

## Return to Traktor or recover after a crash

1. Open **Settings > Screens > Manual recovery**, or use
   **Restart Native Instruments screen driver** in the status-bar menu.
2. If the bridge is running, accept **Stop bridge & restore NI driver**.
   The app waits for that child to exit and release its controller interfaces.
3. The app starts the installed NIHardwareAgent in the background. If it is
   already running, it does not create another instance. This manual action
   works with screen support off and regardless of who previously stopped it.
4. Open Traktor after the driver is restored. If Traktor was already open,
   restart Traktor yourself; the bridge never closes it automatically.

A restoration record is written before NI is stopped. It survives a bridge-app
crash and appears as a recovery hint on the next launch. Failed restoration
keeps that responsibility recorded. Even without a record, the manual restart
button remains available. Force Quit cannot run this app's normal cleanup;
there is no separate always-running or privileged recovery helper.

## Status and controls

| State | Icon while the bridge runs | Recovery |
| --- | --- | --- |
| Screen support off | Green | Manual NI restart remains available |
| Screens active | Green | Toggle can release screens live |
| Screens requested but blocked/failed | Red | Explicit enable/stop-NI action |
| Bridge stopped | Normal template icon | Settings and manual recovery remain available |

Turning off screen support removes screen-related red attention. A blocked
screen service does not mean MIDI has stopped. A missing NI installation does
not block bridge startup; manual restoration reports that installation is
missing instead of trying to install it.

## Process and permission boundary

Only the installed
`/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app`
is managed. The backend verifies bundle ID `com.native-instruments.NIHardwareService`,
the executable path, and Native Instruments' signed identity.
It sends SIGKILL only to the freshly revalidated driver process, subscribes before
requesting termination, and waits for the actual exit with a bounded deadline.
Attended testing isolated SIGTERM shutdown as the screen-blanking trigger:
stopping NI alone darkened both panels, and a later bridge Start did not recover
them. A verified forced stop preserved the logos, and the unchanged bridge then
displayed both pages without a controller reset. The user selected this guarded
handover policy. The existing active-NI-app checks and explicit stopping consent
still apply. The app does not use sudo,
disable login items, change SIP, send AppleScript or install a privileged helper.
Native permission failures are reported rather than escalated.

Restoration starts the verified installed executable directly with disconnected
standard input/output, then verifies its actual process executable. It does not
wait for a background agent to complete an AppKit launch handshake. Native process
discovery also recognizes the agent before LaunchServices registers it; exit
observation uses process events. Shared restoration requests use one native launch
and keep app actions busy until all cleanup transactions finish.
No hardware ownership is handed to NI before the screen worker acknowledges
release or the entire bridge child has exited.

## Preferences and runtime contract

`screen-settings.json` is stored beside the existing app preferences. It contains
`enabled`, `automaticallyStopNI`, `restoreNIOnStop`, and internal `restorePending`
booleans. Missing fields use safe compiled defaults; invalid files are retained
until explicit Reset preferences. Screen controls save immediately. Existing
LED/jog/tension/run files and custom Djay mappings are not migrated or renamed.

The full-profile app holds an stdin pipe open using `--screen-control-stdin`.
Commands are `S4_SCREEN_CONTROL <requestID> ON|OFF|RETRY`; responses are
`S4_SCREEN_STATUS <requestID> active|inactive|failed|pending`, independently of
the selected log level. Generation 0 represents startup. Only the current
generation can satisfy a request; stale acknowledgements are ignored.

`active` follows successful first writes to both panels. `inactive` and
`failed` follow transport release. The worker retains latest state across off
and retry, waits for initialized input and a successful native LED-output tick
before its first publication, and never waits for USB from
the MIDI/HID/motor service. OFF clears the screens; whole-bridge shutdown shows
BRIDGE STOPPED. Managed stdin EOF requests the ordinary safe bridge stop path.
The blocking stdin reader lasts until EOF or process exit.

Minimal MIDI, HID probe, controller checks and mapping export do not enable this
control protocol or manage NI. The standalone CLI still opts in with `--screens`.

## Validation boundary

Tests cover exact native agent identity, absence of an installed agent, refusal
while NI apps are active, denied termination, observed exit and timeout, native
restore identity, manual unowned recovery, durable restoration responsibility,
no automatic checks when off, shared menu preferences, red icon pixels, stale
control generations, release acknowledgement and restore-after-child-exit, and live toggles through a real
owned subprocess. The quit-during-handover and child-exit-during-OFF paths use
explicit event gates rather than sleeps.

NI policy tests use injected seams; native regressions launch owned AppKit and
headless fixture apps and observe actual process exit. UI tests use a fake
agent and an owned subprocess. An authorized real-driver check restored
NIHardwareAgent in 0.202 seconds. The final rebuilt app passed attended Start,
sustained visible screens, Stop/NI restoration and a normal Quit in 0.011 seconds.
The user also verified working Traktor screens afterward and reported that the
corrected app works well on the actual DJ laptop. Long-session endurance remains
untested. See the [validation record](screen-driver-handover-validation.md).
