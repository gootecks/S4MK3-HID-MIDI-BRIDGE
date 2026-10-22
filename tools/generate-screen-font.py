# /// script
# requires-python = ">=3.12"
# dependencies = ["Pillow==12.3.0"]
# ///
"""Print the embedded S4 Screen Sans atlas; no files are changed.

Pass the unmodified LiberationSans-Bold.ttf from liberation-fonts-ttf-2.1.5.
Run with uv, then apply the generated source to src/screens/text/atlas.rs.
The font's OFL notice is kept beside the generated atlas.
"""

import hashlib
import io
import sys
from pathlib import Path
from typing import Final

from PIL import Image, ImageDraw, ImageFont

CHARACTERS: Final = " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789±%/?.-"
SIZES: Final = (8, 16, 24, 32, 40, 48)


def generate(font_data: bytes) -> str:
    """Rasterize fixed sizes to four-bit coverage, encoded as ASCII A-P."""
    lines = [
        "//! Generated S4 Screen Sans coverage masks; do not hand-edit.",
        "//! Source: Liberation Sans Bold 2.1.5; see LICENSE and README.md.",
        f"//! Font SHA-256: {hashlib.sha256(font_data).hexdigest()}",
        "",
        "use super::Glyph;",
        "",
        "#[rustfmt::skip]",
        f"pub(super) static SIZES: [[Glyph; {len(CHARACTERS)}]; 6] = [",
    ]
    for size in SIZES:
        font = ImageFont.truetype(io.BytesIO(font_data), size)
        cap_top = font.getbbox("M", anchor="ls")[1]
        lines.append(f"    [ // {size} pixels")
        for character in CHARACTERS:
            left, top, right, bottom = font.getbbox(character, anchor="ls")
            width, height = right - left, bottom - top
            mask = Image.new("L", (width, height))
            ImageDraw.Draw(mask).text((-left, -top), character, font=font, fill=255, anchor="ls")
            coverage = "".join(chr(65 + (value * 15 + 127) // 255) for value in mask.tobytes())
            advance = round(font.getlength(character))
            lines.append(
                f"        Glyph {{ advance: {advance}, left: {left}, "
                f"top: {top - cap_top}, width: {width}, mask: b\"{coverage}\" }},"
            )
        lines.append("    ],")
    lines.extend(["];", ""])
    return "\n".join(lines)


if __name__ == "__main__":
    sys.stdout.write(generate(Path(sys.argv[1]).read_bytes()))
