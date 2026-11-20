Kontrol S8 kit, built from commit @COMMIT@
https://github.com/gootecks/S4MK3-HID-MIDI-BRIDGE

Runs the Kontrol S8 with djay Pro through the bridge: MIDI, deck LEDs and both
panels. For an Apple Silicon Mac with djay Pro, the Native Instruments
software for the S8 and nothing else.

While it runs, NIHardwareAgent is stopped. The usual S8 setup (Controller
Editor template and its djay mapping) is off until you stop the kit with
Ctrl-C, which brings it back.

FIRST TIME

1. Unzip, then open Terminal and run these two lines. The first moves the
   kit out of Downloads; the second removes the quarantine mark that AirDrop
   and downloads add, which would block the unsigned programs.

       mv ~/Downloads/"@KIT@" ~/
       xattr -dr com.apple.quarantine ~/"@KIT@"

2. Open djay Pro and Controller Editor. Connect the S8.

3. Start the kit from Terminal:

       ~/"@KIT@"/"Start S8.command"

   - Choose Default (Return) or Evan (e). The first start with that layout
     copies its mapping into djay's mapping folder and asks you to restart
     djay Pro. It never overwrites a mapping.
   - macOS asks for Accessibility access for Terminal. Allow it in System
     Settings > Privacy & Security > Accessibility, then start again.
   - If macOS asks whether Terminal may access data from other apps, allow
     it: the deck page reads djay's track analysis.

4. Answer the questions about deck LEDs and the deck page. When the bridge
   runs, choose Settings > MIDI in djay Pro, then device "Kontrol S8 MIDI
   Full" and the matching mapping: "@MAPPING@" for Default or
   "Kontrol S8 Bridge Evan" for Evan. Match the launcher choice every time.

EVERY TIME

Open djay Pro and Controller Editor, then start "Start S8.command".
Ctrl-C stops it: the panels go black, then show the Controller Editor
labels again after a few seconds.

IF SOMETHING IS WRONG

- Ctrl-C. Wait five seconds. The usual setup works again.
- S8 dark and asking for MIDI mode: Ctrl-C, open Controller Editor, start
  again.
- Stop with Ctrl-C, not by closing the Terminal window.
- Each run writes a log into the kit's "logs" folder.

KNOWN LIMITS

- Both layouts use the bridge, not Controller Editor's MIDI addresses.
- Evan was checked on the S8 as mapping Evan r4. Default is the checked
  mapping above.
- No VU meters and no mixer LEDs yet.
- Display buttons 1 and 4 do nothing while the kit runs.
- Deck LEDs and panels in one run were not yet checked on the S8 when this
  kit was built; each was checked alone.
