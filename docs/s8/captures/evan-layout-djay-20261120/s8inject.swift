import CoreMIDI
import Foundation

var client = MIDIClientRef()
MIDIClientCreate("S8Inject" as CFString, nil, nil, &client)
var source = MIDIEndpointRef()
MIDISourceCreate(client, "Kontrol S8 MIDI Full" as CFString, &source)
print("READY")
fflush(stdout)

func send(_ bytes: [UInt8]) {
    var list = MIDIPacketList()
    let packet = MIDIPacketListInit(&list)
    _ = MIDIPacketListAdd(&list, MemoryLayout<MIDIPacketList>.size, packet, 0, bytes.count, bytes)
    MIDIReceived(source, &list)
}

while let line = readLine() {
    let p = line.split(separator: " ")
    guard p.count == 4, let ch = UInt8(p[1]), let a = UInt8(p[2]), let b = UInt8(p[3]) else {
        print("? \(line)"); fflush(stdout); continue
    }
    let status: UInt8 = p[0] == "n" ? 0x90 : (p[0] == "o" ? 0x80 : 0xB0)
    send([status | (ch - 1), a, b])
    print("sent \(line)")
    fflush(stdout)
}
