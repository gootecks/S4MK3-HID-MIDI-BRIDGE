# DriverKit evaluation for the Traktor Kontrol S8 + djay Pro bridge

Research record for #55, 2026-11-09. Read-only: nothing was built or run against the S8.
Sources are Apple's JSON-backed docs (base `https://developer.apple.com/tutorials/data/documentation/<path>.json`) and WWDC transcripts.
Anything not confirmed from a source is tagged **[INFERENCE]**.
The Apple Developer Forums were not reachable, so there is no forum evidence on entitlement-grant policy.

Project status: an Apple Developer ID is available, so Developer ID signing and notarization (§5, step 1) can proceed. The app is still ad-hoc signed today.

## 0. Correction to the working assumption: MIDIDriverKit exists

- **MIDIDriverKit is real.** It is "Develop drivers for MIDI devices" and lets you "implement a MIDI driver extension that communicates with Core MIDI. The framework handles all user-client communication between your driver extension and the Core MIDI server so you don't need to implement a MIDI driver plug-in. You can also leverage … USBDriverKit." — https://developer.apple.com/documentation/mididriverkit
- **Version.** DriverKit platform 24.0, macOS 15.0 (the family entitlement is "macOS 15.0", iPadOS 18) — https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.family.midi
- **Classes.** `IOUserMIDIDriver`, `IOUserMIDIDevice`, `IOUserMIDIEntity`, `IOUserMIDIEndpoint`, `IOUserMIDISource`, `IOUserMIDIDestination`, `IOUserMIDIObject`.
- **Entitlement.** `com.apple.developer.driverkit.family.midi`.
- **Sample.** "Creating a MIDI device driver" publishes a device with a virtual source and destination that shows up in Audio MIDI Setup — https://developer.apple.com/documentation/mididriverkit/creating-a-midi-device-driver

## 1. What each framework is, main classes, minimum macOS

| Framework | Purpose | Main classes | Min macOS | Source |
|---|---|---|---|---|
| DriverKit (base) | User-space driver runtime: C++ classes for driver structure, events, memory, registry types. Installed via SystemExtensions. | `IOService`, `IOUserClient`, `IODispatchQueue`, `IOInterruptDispatchSource`, `IOTimerDispatchSource`, `IODataQueueDispatchSource`, `OSAction`, `IOBufferMemoryDescriptor`, `IOMemoryDescriptor` | 10.15 (DriverKit 19.0) | https://developer.apple.com/documentation/driverkit |
| USBDriverKit | "Drivers for custom or non-class-compliant USB devices"; providers expose configurations, interfaces and endpoints. | `IOUSBHostDevice`, `IOUSBHostInterface`, `IOUSBHostPipe` | 10.15 (DriverKit 19.0; the USB transport entitlement says macOS 10.15) | https://developer.apple.com/documentation/usbdriverkit |
| HIDDriverKit | Drivers for HID devices: keyboards, pointers, digitizers. | `IOUserHIDDevice`, `IOUserUSBHostHIDDevice`, `IOUserHIDEventService`, `IOUserHIDEventDriver`, `IOHIDInterface`, `IOHIDDevice`, `IOHIDElement` | 10.15 | https://developer.apple.com/documentation/hiddriverkit |
| AudioDriverKit | Audio dexts that talk to the CoreAudio HAL. "Eliminates the need to implement an audio server plug-in." Pairs with USB/PCIDriverKit. | `IOUserAudioDriver`, `IOUserAudioDevice`, `IOUserAudioClockDevice`, `IOUserAudioStream`, `IOUserAudioBox`, `IOUserAudio*Control` | 12.0 (DriverKit 21.0; entitlement page "macOS 12.0") | https://developer.apple.com/documentation/audiodriverkit ; https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.family.audio |
| MIDIDriverKit (added) | MIDI dexts that talk to the Core MIDI server. | `IOUserMIDIDriver` and friends (see §0) | 15.0 | https://developer.apple.com/documentation/mididriverkit |
| IOUSBHost (contrast) | Host-mode **user-space app** framework for custom and non-class-compliant USB devices. No entitlement, no system extension. | `IOUSBHostDevice`, `IOUSBHostInterface`, `IOUSBHostObject`, `IOUSBHostPipe` | 10.15 | https://developer.apple.com/documentation/iousbhost |

HIDDriverKit notes:
- `IOUserHIDDevice` is the low-level hardware provider.
- Apple's guidance is to subclass `IOUserUSBHostHIDDevice` instead when you want to customise Apple's default USB HID implementation.
- Both use `IOClass = AppleUserHIDDevice` and `IOProviderClass = IOUSBHostInterface` — https://developer.apple.com/documentation/hiddriverkit/iouserusbhosthiddevice
- `IOUserHIDEventService` parses reports into `IOHIDElement`s via `getElements` and dispatches events from `handleReport`. It uses `IOClass = AppleUserHIDEventService` and `IOProviderClass = IOHIDInterface` — https://developer.apple.com/documentation/hiddriverkit/iouserhideventservice
- `IOUserHIDEventDriver` is Apple's default driver for keyboard, digitizer, scroll and pointer events — https://developer.apple.com/documentation/hiddriverkit/iouserhideventdriver
- The keyboard sample forwards parsed element changes to the system "as part of an event, which the system then dispatches to relevant apps" — https://developer.apple.com/documentation/hiddriverkit/handling-keyboard-events-from-a-human-interface-device

Stale doc warning: "Creating a Driver Using the DriverKit SDK" still says DriverKit "doesn't support USB devices that manipulate audio". That is contradicted by AudioDriverKit (macOS 12+) — https://developer.apple.com/documentation/driverkit/creating-a-driver-using-the-driverkit-sdk

## 2. Distribution cost

### Entitlements

| Entitlement | What it is | Needs Apple approval? | Source |
|---|---|---|---|
| `com.apple.developer.driverkit` | Permission to run as a user-space driver. Required on every dext. | **Yes** — "You must request this entitlement from Apple." | https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit |
| `com.apple.developer.driverkit.transport.usb` | Array of dictionaries naming supported USB devices. Keys mirror descriptor fields: `idVendor`, `idProduct`, `idProductArray`, `idProductMask`, `bcdDevice`, `bDeviceClass`/`SubClass`/`Protocol`, `bInterfaceClass`/`Number`/`SubClass`/`Protocol`, `bConfigurationValue`. | Yes (transport entitlement) | https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.transport.usb |
| `com.apple.developer.driverkit.transport.hid` | Lets the driver interact with HID hardware. | Yes (same request group) | https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.transport.hid |
| `com.apple.developer.driverkit.family.hid.device` | Driver provides a HID service. | Yes | https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.family.hid.device |
| `com.apple.developer.driverkit.family.hid.eventservice` | Driver provides a HID event service. | Yes | https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.family.hid.eventservice |
| `com.apple.developer.driverkit.family.audio` | Audio family. | Yes | https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.family.audio |
| `com.apple.developer.driverkit.family.midi` | MIDI family. | Yes | https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.family.midi |
| `com.apple.developer.driverkit.userclient-access` | Goes on the **app**; lists bundle IDs of dexts it may open an `IOUserClient` to. | The docs list it among the request-group entitlements; the sample provisions it with the others. | https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.userclient-access |
| `com.apple.developer.driverkit.allow-any-userclient-access` | Goes on the **dext**; accepts user clients from any app. AudioDriverKit requires it; for audio Apple says it is "available to all developers who have been approved for any DriverKit entitlements". | Gated on being approved for some DriverKit entitlement. | https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.allow-any-userclient-access ; https://developer.apple.com/videos/play/wwdc2021/10190/ |
| `com.apple.developer.system-extension.install` | Host-app capability needed to submit activation requests. | Added through the Xcode System Extension capability. | https://developer.apple.com/documentation/driverkit/requesting-entitlements-for-driverkit-development |

Process and signing facts:
- You request entitlements through a form on the developer site. The form needs an Apple ID sign-in — https://developer.apple.com/contact/request/system-extension/ redirects to sign-in.
- The request asks for the transport, **"your company's hardware vendor ID for the type of devices"**, device descriptions and a description of the installing app.
- Apple ties the grant to the team profile. All entitlements for one product must be in one group, and the provisioning profile can include only **one** group — https://developer.apple.com/documentation/driverkit/requesting-entitlements-for-driverkit-development
- Entitlements in the dext's file must match the code signature at activation, or activation aborts (same page).
- You need explicit App IDs and provisioning profiles, so automatic signing does not work for the HID sample (same page; also the keyboard sample page).

### Paid membership, Developer ID, notarization

- Notarization or the Mac App Store is **required**: "You must either distribute your app and extension through the Mac App Store, or notarize them" — https://developer.apple.com/documentation/systemextensions
- The dext must be signed by the same Team ID as the app (same page), unless the extension has `com.apple.developer.system-extension.redistributable`.
- WWDC19: Developer ID-signed system extensions "must be notarized before it will run" — https://developer.apple.com/videos/play/wwdc2019/702/
- A paid Apple Developer Program membership is not stated outright in these pages. The entitlement form needs an Apple ID sign-in, and the process uses portal identifiers, profiles and Developer ID certificates. **[INFERENCE]: membership is required in practice.**

### Is the USB entitlement tied to a vendor ID? Would Apple grant it for NI's VID 0x17CC?

- The entitlement format is a list of descriptor-field dictionaries including `idVendor`, `idProduct` and `bInterfaceNumber`. It is therefore device-specific and can be as narrow as one interface of one product — https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.driverkit.transport.usb
- The request form explicitly asks for "your company's hardware vendor ID".
- WWDC20 shows entitlements can be masked to a whole vendor (PCI example) — https://developer.apple.com/videos/play/wwdc2020/10210/
- I found **no document** saying whether Apple grants a transport entitlement for a VID owned by another company. **[INFERENCE]: a request for NI's VID by a non-NI developer is unlikely to be approved without NI's involvement.**
  - Reason: Apple's stated concern is that entitlements prevent "another app or malicious software" from accessing the hardware (WWDC20), and Apple already refuses entitlements for non-hardware-associated virtual audio drivers (WWDC21).
  - Treat it as a gamble, not a plan.
- **Virtual-only MIDI dext.** The MIDIDriverKit sample needs only `driverkit` + `family.midi` (+ `allow-any-userclient-access`), with no transport entitlement. But Apple said for audio that a virtual driver "not associated with a hardware device … entitlements will not be granted" — https://developer.apple.com/videos/play/wwdc2021/10190/ **[INFERENCE]: the same hardware-association test probably applies to a virtual-only MIDI dext.**

### Local development without grants

Source: https://developer.apple.com/documentation/driverkit/debugging-and-testing-system-extensions
- `systemextensionsctl developer on` stops the location check, so the app can run outside /Applications.
- Disabling SIP bypasses code-signing, entitlement and notarization checks. "Xcode doesn't notarize apps or system extensions during the normal development cycle."
- The DriverKit sample uses automatic signing + "Sign to Run Locally" + `AD_HOC_CODE_SIGNING_ALLOWED=YES` + SIP off + dext developer mode — https://developer.apple.com/documentation/driverkit/communicating-between-a-driverkit-extension-and-a-client-app
- WWDC20: "disable system integrity protection, sign your dexts to run locally, and include all the entitlements" — https://developer.apple.com/videos/play/wwdc2020/10210/
- Consequence: only the developer's own Macs work, with SIP off. That is not distributable.

### End-user install flow

- The dext sits in `Contents/Library/SystemExtensions` inside the app, and the app must be in an Applications directory. The app submits an `OSSystemExtensionRequest.activationRequest`. The system checks location, signature, entitlement match and identifier — https://developer.apple.com/documentation/systemextensions/installing-system-extensions-and-drivers
- The user must approve. On macOS: "System Settings > Privacy & Security" (https://developer.apple.com/documentation/driverkit/creating-drivers-for-ipados), or per the MIDI sample, "System Extension Blocked" → System Settings → Login Items & Extensions → Allow (https://developer.apple.com/documentation/mididriverkit/creating-a-midi-device-driver).
- Updates re-run the activation request, and the delegate compares versions (installing page). Deleting the app removes the dext (same page; WWDC19).
- The dext runs on demand: it is started when matching hardware appears (WWDC19).

## 3. Matching and priority

What the sources establish:
- "DriverKit drivers appear to Kernel Extensions and can compete in matching with kernel drivers"; they are visible in the IORegistry — https://developer.apple.com/videos/play/wwdc2019/702/
- IOKit matching runs class → passive (personality keys) → active (probe score). Candidates are sorted by `IOProbeScore`, the highest starts, and "the more generic driver typically loses out to the more specific driver" — https://developer.apple.com/library/archive/documentation/DeviceDrivers/Conceptual/IOKitFundamentals/Matching/Matching.html
- A USB-interface personality uses `IOProviderClass = IOUSBHostInterface`, and "you may add other keys … `VendorID`, `ProductID`, `PrimaryUsagePage`, `PrimaryUsage`" — https://developer.apple.com/documentation/hiddriverkit/iouserhiddevice
- The system creates one `IOUSBHostInterface` per USB interface (https://developer.apple.com/documentation/usbdriverkit/iousbhostinterface). The transport-entitlement key `bInterfaceNumber` exists, so interface-granular matching is supported by design.

Answers:
1. **Would a dext on IF5 or IF6 win over Apple's generic HID driver?**
   - **[INFERENCE]** Yes if its personality is more specific (VID/PID/`bInterfaceNumber`) and its `IOProbeScore` is higher.
   - Apple's own USB HID path is itself a dext class (`AppleUserHIDDevice` / `AppleUserUSBHostHIDDevice`), and the docs name `IOUserUSBHostHIDDevice` as the way to customise it.
   - Nothing I fetched gives Apple's default score, so it needs a test.
   - IF6 has no client today (given), so there is nothing to beat.
2. **Would it win over NIHardwareAgent?**
   - NIHardwareAgent is a user-space process opening the device from outside the kernel matching phase. A dext binds at attach time, before the agent.
   - Open is exclusive: "Only one service at a time may open a session to the interface" — https://developer.apple.com/documentation/usbdriverkit/iousbhostinterface/open
   - **[INFERENCE]** If the dext opens the interface, the agent's and `nusb`'s attempts to open it would fail. If the dext matches but never calls `Open`, user-space opens may still succeed (untested).
   - So a dext that opens IF6 would exclude NI's agent from the screens. It would also exclude our own `nusb` unless the dext exposes a user client and the app talks through it.
3. **Does a claiming dext exclude user-space opens?**
   - For the `IOUSBHostInterface` session, yes (same quote).
   - User-space `IOUSBHost` has `deviceCapture`, which "terminates all clients and drivers" but needs `com.apple.vm.device-access` or root — https://developer.apple.com/documentation/iousbhost/iousbhostobjectinitoptions/devicecapture
   - HID shared reads via `IOHIDManager` are a different layer. A dext `IOUserHIDDevice` still publishes an `IOHIDDevice`, so shared reads continue to work **[INFERENCE]**.
4. **Can a dext take only one interface of a composite device and leave audio and USB MIDI to Apple?**
   - **[INFERENCE]** Yes. Matching is per `IOUSBHostInterface` nub and the entitlement can name `bInterfaceNumber`. A personality with `IOProviderClass = IOUSBHostInterface` and `bInterfaceNumber = 5` (or `6`) matches just that nub.
   - Caution: a device-level personality (`IOUSBHostDevice` provider) would take the whole device and its configuration, so don't use it.
   - This is an interface-granularity claim from the entitlement keys and the provider model, not a tested result.

## 4. Fit against the project

### (a) HIDDriverKit
- Consumer is djay, which wants MIDI. HIDDriverKit services dispatch **HID events** (keyboard, pointer, digitizer) — see the sample and `IOUserHIDEventDriver` page above. It has no MIDI output path.
- A HID dext could parse the S8's IF5 reports, but the translation to MIDI would still have to happen in user space. No gain.
- It can't replace hidapi shared-mode reads in a useful way. Shared reads already work (given) and a HID dext would only change who parses the report.
- It is a large cost (HID transport + family entitlements, signing, approval) for no new capability.
- **Verdict: rule out.**

### (b) USBDriverKit
- **What it adds:**
  - Claims IF5/IF6 at plug-in, before NI's agent starts. This is deterministic ownership with no `NIHardwareAgent` stop/restore dance.
  - Survives app quit and hotplug, with the dext started on demand.
  - Crash isolation, with automatic restart (WWDC19).
- **Costs:**
  - Needs `driverkit` + `transport.usb` (`idVendor` = `0x17CC`) approved by Apple, which is doubtful for a non-NI developer (§2).
  - Developer ID, notarization, provisioning profile, user approval, app in /Applications.
  - C++ in the restricted DriverKit runtime (no filesystem or networking). The current Rust `nusb` code can't run in the dext; it would have to be rewritten and an `IOUserClient` bridge added.
  - Bulk-frame throughput through a user client, and screen-frame tuning, would be new engineering risk **[INFERENCE]**.
- Contrast: user-space `IOUSBHost` (and `nusb`, which is the same idea) needs no entitlement. The bridge already claims IF6 and writes frames to OUT 0x04 (given).
- **Verdict: defer; not worth the entitlement risk now.** The only real gap it closes is the race with NIHardwareAgent, which a user-space coexistence policy can handle.

### (c) AudioDriverKit
- The S8's audio is class-compliant and intentionally untouched (given), so there is nothing to drive.
- Apple's position on virtual audio devices: AudioDriverKit samples that are "not associated with a hardware device" will not get entitlements, and "the audio server plug-in driver model should continue to be used"; the AudioServerPlugIn interface "is not deprecated" — https://developer.apple.com/videos/play/wwdc2021/10190/
- **Verdict: rule out.**

### (d) Can any DriverKit family publish a MIDI device to CoreMIDI?
- **Yes: MIDIDriverKit (macOS 15+).** It is the only DriverKit family that does (§0). Not USBDriverKit, HIDDriverKit or AudioDriverKit.
- Architecture if adopted (**[INFERENCE]** for the exact shape): the Rust bridge keeps parsing HID and talks to a MIDI dext through an `IOUserClient` (`IOUserMIDIDriver::NewUserClient`; the app carries `userclient-access`). The dext exposes a real `IOUserMIDIDevice` with entities, sources and destinations. djay then sees a genuine MIDI device in Audio MIDI Setup.
- It doesn't need USBDriverKit to function. The sample is a virtual device with a port that has a source and a destination, plus a user client for control.
- Cost: `driverkit` + `family.midi` entitlements (approval, with a likely hardware-association hurdle), Developer ID + provisioning profile + notarization, user approval, C++ dext, macOS 15+ only. It would hand **no new capability** to djay beyond what virtual endpoints already provide.

Native routes to "appear as a proper MIDI device", compared:

| Route | What you get | Cost / caveats | Source |
|---|---|---|---|
| Virtual endpoints (`MIDISourceCreateWithProtocol`, `MIDIDestinationCreateWithProtocol`) — **current approach** | Endpoints owned by the app/client. No device parent. | No entitlement, no extra install. Assign the same unique ID each launch so clients keep persistent references. They exist only while the app runs. | https://developer.apple.com/documentation/coremidi/midisourcecreatewithprotocol(_:_:_:_:) ; https://developer.apple.com/documentation/coremidi/mididestinationcreatewithprotocol(_:_:_:_:_:) |
| `MIDIExternalDeviceCreate` | Persistent external device description with entities, for hardware connected via cables. | Docs: "Non-drivers may call this function to create external devices." It describes a cable-connected external device and does not make the S8 itself appear. | https://developer.apple.com/documentation/coremidi/midiexternaldevicecreate(_:_:_:_:) |
| CoreMIDI driver plug-in (`MIDIDriverInterface`, CFPlugIn) | Real `MIDIDevice` created by a driver (`MIDIDeviceCreate`, `MIDIGetDriverDeviceList`). Docs still list "Create driver plug-ins". | Legacy path — MIDIDriverKit's overview says it removes the need to write one. Install location `/Library/Audio/MIDI Drivers` is from my knowledge, not fetched **[INFERENCE]**; it needs admin install. It runs inside Apple's MIDIServer **[INFERENCE]**, so an unsigned or notarization-unfriendly plug-in is a stability and trust risk. | https://developer.apple.com/documentation/coremidi/midi-drivers ; https://developer.apple.com/documentation/mididriverkit |
| MIDIDriverKit dext | Real device, no plug-in. | See cost above. | https://developer.apple.com/documentation/mididriverkit/creating-a-midi-device-driver |

Note: the `MIDIDeviceCreate` doc text says "Nondrivers may call this function to create external devices". That looks like a copy of the `MIDIExternalDeviceCreate` wording, so don't treat it as proof that an app may create a driver-owned device.

## 5. Recommendation

| Item | Decision | Why |
|---|---|---|
| HIDDriverKit | **Rule out** | Wrong output domain (HID events, not MIDI). Hidapi shared reads already work. |
| AudioDriverKit | **Rule out** | Audio is class-compliant and untouched. Apple's virtual-audio position says no entitlement for non-hardware drivers. |
| USBDriverKit | **Defer** | Gives deterministic IF6/IF5 ownership vs NIHardwareAgent, but needs an Apple USB grant for NI's VID that a non-NI developer is unlikely to get **[INFERENCE]**. Revisit only if NI cooperates or Apple confirms the grant. |
| MIDIDriverKit | **Defer / optional experiment** | Only DriverKit family that makes a native MIDI device. macOS 15+. No new djay capability over virtual endpoints. Needs `family.midi` (with hardware-association risk) + paid-program signing. Try only after Developer ID + notarization is in place, and only if "device identity in Audio MIDI Setup" becomes a stated requirement. |
| CoreMIDI driver plug-in | **Rule out** | Legacy, admin-installed, runs in MIDIServer **[INFERENCE]**, and MIDIDriverKit supersedes it. |

Cheaper polish steps that need no DriverKit:

1. **Developer ID signing + notarization** — the real prerequisite for any later system-extension path, and it removes Gatekeeper friction now (notarization is mandatory for system extensions: https://developer.apple.com/documentation/systemextensions). It requires a paid membership **[INFERENCE]**.
2. **`SMAppService` login item or launch agent** (macOS 13+): "register and control LoginItems, LaunchAgents, and LaunchDaemons as helper executables for your app" — https://developer.apple.com/documentation/servicemanagement/smappservice. Gives auto-start at login.
3. **IOKit hotplug** with `IOServiceAddMatchingNotification` ("the preferred method of finding IOService objects that may arrive at any time") — https://developer.apple.com/documentation/iokit/1514362-ioserviceaddmatchingnotification. Match on `idVendor`/`idProduct` for 0x17CC/0x1370. The bridge can then claim the device on attach and release it on detach.
4. **Stable unique IDs for the virtual MIDI endpoints**: Apple tells you to reuse the same unique ID every launch so clients (djay mapping, Audio MIDI Setup) keep persistent references — https://developer.apple.com/documentation/coremidi/midisourcecreatewithprotocol(_:_:_:_:)
5. **NIHardwareAgent coexistence** (user-space policy, no driver):
   - Detect the agent, stop it only for display takeover, and restore it on quit or crash.
   - Restore means opening `/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app` ([agent-stop-validation.md](agent-stop-validation.md)).
   - Add a watchdog so a bridge crash can't leave the S8 without MIDI.
   - Gate this behind explicit user consent and a visible state in the menu bar.
6. **Contingency for the dext path**: if you ever want USBDriverKit, first email NI/Apple about a vendor-ID entitlement request. Don't build the dext before the entitlement is granted.

## Unresolved (no source reachable)

- Apple's actual grant policy for third-party VIDs. Forums blocked and the request form needs sign-in.
- Whether a dext that matches but never opens IF5/IF6 blocks user-space `IOUSBHostInterface` opens.
- Apple's default `IOProbeScore` for `AppleUserUSBHostHIDDevice`.
- Whether djay lists a MIDIDriverKit device differently from virtual endpoints, or whether the `.djayMidiMapping` binds by name.
- Whether Apple would grant `family.midi` for a virtual-only dext.
