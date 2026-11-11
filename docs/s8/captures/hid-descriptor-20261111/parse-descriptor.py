"""Print the reports declared by the Kontrol S8 HID report descriptor.

Reads the IORegistry (ioreg) or a hex file; opens no device and sends nothing.
Usage: uv run parse-descriptor.py [report-descriptor.hex]
"""

import plistlib
import subprocess
import sys

KINDS = {0x80: "input", 0x90: "output", 0xB0: "feature"}


def from_ioreg() -> bytes:
    raw = subprocess.run(
        ["ioreg", "-r", "-c", "IOHIDDevice", "-l", "-a", "-w", "0"],
        capture_output=True,
        check=True,
    ).stdout
    stack = [plistlib.loads(raw)]
    while stack:
        node = stack.pop()
        if isinstance(node, list):
            stack.extend(node)
        elif isinstance(node, dict):
            if node.get("VendorID") == 0x17CC and node.get("ProductID") == 0x1370:
                return node["ReportDescriptor"]
            stack.extend(node.get("IORegistryEntryChildren", []))
    sys.exit("no Kontrol S8 HID node in the IORegistry")


def reports(descriptor: bytes) -> dict[tuple[int, str], int]:
    bits: dict[tuple[int, str], int] = {}
    report_id = size = count = 0
    i = 0
    while i < len(descriptor):
        head = descriptor[i]
        length = (0, 1, 2, 4)[head & 3]
        value = int.from_bytes(descriptor[i + 1 : i + 1 + length], "little")
        tag = head & 0xFC
        i += 1 + length
        if tag == 0x84:
            report_id = value
        elif tag == 0x74:
            size = value
        elif tag == 0x94:
            count = value
        elif tag in KINDS:
            key = (report_id, KINDS[tag])
            bits[key] = bits.get(key, 0) + size * count
    return bits


def main() -> None:
    if len(sys.argv) > 1:
        with open(sys.argv[1], encoding="ascii") as handle:
            descriptor = bytes.fromhex("".join(handle.read().split()))
    else:
        descriptor = from_ioreg()
    print(f"{len(descriptor)} descriptor bytes")
    for (report_id, kind), total in reports(descriptor).items():
        payload = total // 8
        print(f"0x{report_id:02X} {kind:7} payload {payload:3}  with ID {payload + 1:3}")


if __name__ == "__main__":
    main()
