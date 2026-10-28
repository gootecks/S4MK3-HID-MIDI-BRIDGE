use super::*;
use crate::screens::rgb;

#[test]
fn embedded_masks_have_complete_rows_and_native_coverage() {
    // Given the complete compiled font subset at all supported sizes.
    for size in &atlas::SIZES {
        assert_eq!(size.len(), CHARACTERS.chars().count());
        for (character, glyph) in CHARACTERS.chars().zip(size) {
            // When each mask's machine-consumed metadata is interpreted.
            let width = glyph.width.max(1);
            // Then rows are complete, alpha is valid, and no printable glyph is missing.
            assert_eq!(glyph.mask.len() % width, 0);
            assert!(glyph.mask.iter().all(|alpha| (b'A'..=b'P').contains(alpha)));
            assert!(glyph.advance > 0);
            if character != ' ' {
                assert!(glyph.mask.iter().any(|&alpha| alpha > b'A'));
                assert!(glyph.mask.iter().any(|&alpha| alpha > b'A' && alpha < b'P'));
            }
        }
    }
}

#[test]
fn coverage_blends_each_rgb565_channel_without_bleeding() {
    // Given distinct foreground and background colors.
    for background in [0, 0xffff, rgb(0, 220, 80), rgb(252, 176, 36)] {
        for foreground in [0, 0xffff, rgb(10, 12, 18)] {
            // When zero and full coverage are composited.
            // Then both endpoints are preserved exactly.
            assert_eq!(blend(background, foreground, 0), background);
            assert_eq!(blend(background, foreground, 15), foreground);
        }
    }
    assert_eq!(blend(0, 0xffff, 7), (0xe << 11) | (0x1d << 5) | 0xe);
}

#[test]
fn real_text_smooths_edges_on_dark_green_amber_and_inverted_fills() {
    // Given the actual UI background/ink pairs.
    for (background, foreground) in [
        (rgb(10, 12, 18), rgb(244, 248, 255)),
        (rgb(0, 220, 80), 0),
        (rgb(252, 176, 36), 0),
        (rgb(244, 248, 255), rgb(10, 12, 18)),
    ] {
        let mut frame = Frame::default();
        frame.clear(background);
        // When a real value is drawn through the shared renderer.
        frame.text(12, 12, "8", 6, foreground);
        // Then solid ink and intermediate blended edges coexist with untouched fill.
        assert!(frame.pixels.contains(&foreground));
        assert!(frame.pixels.contains(&background));
        assert!(
            frame
                .pixels
                .iter()
                .any(|&pixel| pixel != foreground && pixel != background)
        );
    }
}

#[test]
fn proportional_measurement_centers_values_and_preserves_spacing() {
    // Given strings with different proportional advances.
    assert!(Frame::text_width("III", 6) < Frame::text_width("WWW", 6));
    assert!(Frame::text_width("8 8", 6) > Frame::text_width("88", 6));
    let mut frame = Frame::default();
    // When a value is centered in a quadrant.
    frame.centered_text(80, 44, "88", 6, 0xffff);
    // Then its visible bounding box stays centered within one pixel.
    let xs: Vec<_> = frame
        .pixels
        .iter()
        .enumerate()
        .filter_map(|(index, &pixel)| (pixel != 0).then_some(index % WIDTH))
        .collect();
    assert!(
        xs.iter()
            .min()
            .zip(xs.iter().max())
            .is_some_and(|(left, right)| { (left + right).abs_diff(160) <= 2 })
    );
}

#[test]
fn text_clips_at_all_edges_without_wrapping_or_losing_partial_glyphs() {
    // Given each glyph at a safe origin and the top-left screen edge.
    for scale in 1..=6 {
        for character in CHARACTERS.chars() {
            let text = character.to_string();
            let mut inset = Frame::default();
            let mut clipped = Frame::default();
            inset.text(8, 8, &text, scale, 0xffff);
            // When negative bearings and the right/bottom edge require clipping.
            clipped.text(0, 0, &text, scale, 0xffff);
            // Then the visible top-left portion matches the unclipped glyph.
            for y in 0..56 {
                for x in 0..56 {
                    assert_eq!(
                        clipped.pixels[y * WIDTH + x],
                        inset.pixels[(y + 8) * WIDTH + x + 8],
                        "{character} at scale {scale}"
                    );
                }
            }
        }
    }
    let mut edge = Frame::default();
    edge.text(WIDTH - 6, HEIGHT - 4, "8±", 6, 0xffff);
    assert!(edge.pixels.iter().enumerate().all(|(index, &pixel)| {
        pixel == 0 || (index % WIDTH >= WIDTH - 6 && index / WIDTH >= HEIGHT - 4)
    }));
    assert!(edge.pixels.contains(&0xffff));
}
