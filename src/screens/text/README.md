# S4 Screen Sans

This embedded, pre-rendered glyph atlas follows the user-approved smooth bold
sans-serif preview. It is derived from **Liberation Sans Bold 2.1.5**, licensed
under the SIL Open Font License 1.1. `LICENSE` contains the original notice.
S4 Screen Sans is the name of this converted subset, not an upstream font name.

Upstream release:
`https://github.com/liberationfonts/liberation-fonts/releases/tag/2.1.5`

Source archive:
`https://github.com/liberationfonts/liberation-fonts/files/7261482/liberation-fonts-ttf-2.1.5.tar.gz`

- Archive SHA-256: `7191c669bf38899f73a2094ed00f7b800553364f90e2637010a69c0e268f25d0`
- `LiberationSans-Bold.ttf` SHA-256: `788abee4c806d660e8aee46689dd8540cd4bb98da03dcc9d171ce3efd99a9173`
- Generator: `tools/generate-screen-font.py`, Pillow 12.3.0.

To regenerate, unpack the pinned archive and run:

```sh
uv run tools/generate-screen-font.py /path/to/LiberationSans-Bold.ttf
```

The command prints Rust source to stdout. Apply that output to `atlas.rs`;
normal Cargo builds never download fonts or run Python. Source and portable
distribution must retain the font's license.

The subset contains the screens' uppercase alphabet, digits, space, plus/minus,
percent, slash, question mark, period and minus. Each glyph has its measured
integer advance, cap-relative offset, width and four-bit coverage mask.
The six sizes are native 8, 16, 24, 32, 40 and 48 pixels, not magnified bitmaps.
ASCII A-P stores coverage 0-15 without escaping. The renderer blends each mask
against the actual RGB565 background, including inverted tiles and colored
quadrants. No runtime glyph allocation, rasterizer or installed font is needed.
