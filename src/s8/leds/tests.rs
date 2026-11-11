use super::{
    DECK_REPORT_LEN, Dual, Level, MAX_VALUE, Rgb, STRIP_LEDS, Side, SideLeds, Strip, bar, clear,
    dot, map, render,
};

type Setter = fn(&mut SideLeds);

fn expected(side: Side, bytes: &[(usize, u8)]) -> [u8; DECK_REPORT_LEN] {
    let mut report = clear(side);
    for (offset, value) in bytes {
        report[offset + 1] = *value;
    }
    report
}

fn lit(side: Side, set: Setter) -> [u8; DECK_REPORT_LEN] {
    let mut leds = SideLeds::default();
    set(&mut leds);
    render(side, &leds)
}

fn single_fields() -> [(&'static str, Setter, usize); 9] {
    [
        ("fx select", |l| l.fx_select = Level::On, 24),
        ("back", |l| l.back = Level::On, 37),
        ("capture", |l| l.capture = Level::On, 38),
        ("edit", |l| l.edit = Level::On, 39),
        ("flux", |l| l.flux = Level::On, 52),
        ("shift", |l| l.shift = Level::On, 55),
        ("sync", |l| l.sync = Level::On, map::SYNC),
        ("cue", |l| l.cue = Level::On, 58),
        ("play", |l| l.play = Level::On, 59),
    ]
}

type DualField = fn(&mut SideLeds) -> &mut Dual;

fn dual_fields() -> [(&'static str, DualField, usize); 5] {
    [
        ("hotcue", |l| &mut l.hotcue, 44),
        ("loop", |l| &mut l.loop_mode, 46),
        ("freeze", |l| &mut l.freeze, 48),
        ("remix", |l| &mut l.remix, 50),
        ("deck", |l| &mut l.deck, 53),
    ]
}

#[test]
fn default_model_renders_id_and_zeroes_for_each_side() {
    for side in Side::ALL {
        let report = render(side, &SideLeds::default());
        assert_eq!(report.len(), 119);
        assert_eq!(report[0], side.report_id());
        assert!(report[1..].iter().all(|byte| *byte == 0));
        assert_eq!(report, clear(side));
    }
}

#[test]
fn each_single_led_lights_exactly_its_byte() {
    for side in Side::ALL {
        for (name, set, offset) in single_fields() {
            assert_eq!(lit(side, set), expected(side, &[(offset, 0x7f)]), "{name}");
        }
    }
}

#[test]
fn each_half_of_every_dual_lights_exactly_its_byte() {
    for side in Side::ALL {
        for (name, field, offset) in dual_fields() {
            let mut leds = SideLeds::default();
            field(&mut leds).white = Level::On;
            assert_eq!(
                render(side, &leds),
                expected(side, &[(offset, 0x7f)]),
                "{name} white"
            );
            let mut leds = SideLeds::default();
            field(&mut leds).blue = Level::On;
            assert_eq!(
                render(side, &leds),
                expected(side, &[(offset + 1, 0x7f)]),
                "{name} blue"
            );
        }
    }
}

#[test]
fn array_buttons_light_exactly_their_bytes() {
    for side in Side::ALL {
        for index in 0..4 {
            let mut leds = SideLeds::default();
            leds.fx[index] = Level::On;
            assert_eq!(render(side, &leds), expected(side, &[(25 + index, 0x7f)]));
            let mut leds = SideLeds::default();
            leds.slots[index] = Level::On;
            assert_eq!(render(side, &leds), expected(side, &[(40 + index, 0x7f)]));
            let mut leds = SideLeds::default();
            leds.loop_ring[index].white = Level::On;
            assert_eq!(render(side, &leds), expected(side, &[(60 + index, 0x7f)]));
            let mut leds = SideLeds::default();
            leds.loop_ring[index].blue = Level::On;
            assert_eq!(render(side, &leds), expected(side, &[(64 + index, 0x7f)]));
        }
        for index in 0..8 {
            let mut leds = SideLeds::default();
            leds.display[index] = Level::On;
            assert_eq!(
                render(side, &leds),
                expected(side, &[(map::DISPLAY[index], 0x7f)])
            );
        }
    }
}

#[test]
fn second_sync_byte_stays_zero() {
    let leds = SideLeds {
        sync: Level::On,
        ..SideLeds::default()
    };
    let report = render(Side::Left, &leds);
    assert_eq!(report[map::SYNC_SECOND + 1], 0);
}

#[test]
fn pad_colour_lands_in_its_three_bytes_in_pad_order() {
    for side in Side::ALL {
        for (pad, base) in [(0, 0), (2, 6), (7, 21)] {
            let mut leds = SideLeds::default();
            leds.pads[pad] = Rgb::new(1, 2, 3);
            let want = expected(side, &[(base, 1), (base + 1, 2), (base + 2, 3)]);
            assert_eq!(render(side, &leds), want, "pad {pad}");
        }
    }
}

#[test]
fn pad_components_above_the_maximum_are_clamped() {
    let mut leds = SideLeds::default();
    leds.pads[0] = Rgb::new(0xff, 0x80, 0x7f);
    let report = render(Side::Left, &leds);
    assert_eq!(report[1..4], [MAX_VALUE; 3]);
}

#[test]
fn dim_level_writes_0x14() {
    let leds = SideLeds {
        play: Level::Dim,
        ..SideLeds::default()
    };
    assert_eq!(
        render(Side::Left, &leds),
        expected(Side::Left, &[(59, 0x14)])
    );
}

#[test]
fn left_and_right_differ_only_in_the_report_id() {
    let mut leds = SideLeds::default();
    leds.pads[3] = Rgb::new(9, 8, 7);
    leds.hotcue.blue = Level::Dim;
    leds.strip.orange = dot(30, 100);
    let leds = leds;
    let left = render(Side::Left, &leds);
    let right = render(Side::Right, &leds);
    assert_ne!(left[0], right[0]);
    assert_eq!(left[1..], right[1..]);
}

#[test]
fn strip_ends_land_at_the_bank_edges() {
    let mut blue = [0; STRIP_LEDS];
    blue[0] = 5;
    blue[24] = 6;
    let mut orange = [0; STRIP_LEDS];
    orange[0] = 7;
    orange[24] = 8;
    let leds = SideLeds {
        strip: Strip { blue, orange },
        ..SideLeds::default()
    };
    let want = expected(Side::Left, &[(68, 5), (92, 6), (93, 7), (117, 8)]);
    assert_eq!(render(Side::Left, &leds), want);
}

#[test]
fn blue_and_orange_banks_do_not_overlap() {
    let mut leds = SideLeds::default();
    leds.strip.blue = [MAX_VALUE; STRIP_LEDS];
    let report = render(Side::Right, &leds);
    let lit: Vec<usize> = (1..DECK_REPORT_LEN).filter(|i| report[*i] != 0).collect();
    assert_eq!(lit, (69..=93).collect::<Vec<_>>());

    let mut leds = SideLeds::default();
    leds.strip.orange = [MAX_VALUE; STRIP_LEDS];
    let report = render(Side::Right, &leds);
    let lit: Vec<usize> = (1..DECK_REPORT_LEN).filter(|i| report[*i] != 0).collect();
    assert_eq!(lit, (94..=118).collect::<Vec<_>>());
}

#[test]
fn strip_intensity_above_the_maximum_is_clamped() {
    let mut leds = SideLeds::default();
    leds.strip.blue = [0xff; STRIP_LEDS];
    leds.strip.orange = [0x80; STRIP_LEDS];
    let report = render(Side::Left, &leds);
    assert!(report[69..=118].iter().all(|byte| *byte == MAX_VALUE));
}

#[test]
fn no_rendered_byte_exceeds_the_maximum_for_extreme_models() {
    let extremes = [
        SideLeds {
            pads: [Rgb::new(0xff, 0xff, 0xff); 8],
            fx: [Level::On; 4],
            display: [Level::On; 8],
            slots: [Level::On; 4],
            hotcue: Dual {
                white: Level::On,
                blue: Level::On,
            },
            loop_ring: [Dual {
                white: Level::On,
                blue: Level::On,
            }; 4],
            strip: Strip {
                blue: [0xff; STRIP_LEDS],
                orange: [0xff; STRIP_LEDS],
            },
            ..SideLeds::default()
        },
        SideLeds {
            pads: [Rgb::new(0x80, 0, 0xfe); 8],
            strip: Strip {
                blue: [0x80; STRIP_LEDS],
                orange: [0xfe; STRIP_LEDS],
            },
            ..SideLeds::default()
        },
    ];
    for leds in &extremes {
        for side in Side::ALL {
            assert!(
                render(side, leds)[1..]
                    .iter()
                    .all(|byte| *byte <= MAX_VALUE)
            );
        }
    }
}

fn lit_indices(leds: &[u8; STRIP_LEDS]) -> Vec<usize> {
    leds.iter()
        .enumerate()
        .filter(|(_, value)| **value != 0)
        .map(|(index, _)| index)
        .collect()
}

#[test]
fn dot_lights_exactly_one_led_at_the_scaled_position() {
    assert_eq!(lit_indices(&dot(0, 50)), vec![0]);
    assert_eq!(lit_indices(&dot(63, 50)), vec![12]);
    assert_eq!(lit_indices(&dot(64, 50)), vec![12]);
    assert_eq!(lit_indices(&dot(127, 50)), vec![24]);
    assert_eq!(lit_indices(&dot(255, 50)), vec![24]);
    assert_eq!(dot(0, 50)[0], 50);
    assert_eq!(dot(0, 0xff)[0], MAX_VALUE);
}

#[test]
fn bar_lights_leds_from_the_left_up_to_the_fraction() {
    assert!(lit_indices(&bar(0, 50)).is_empty());
    assert_eq!(lit_indices(&bar(1, 50)), vec![0]);
    assert_eq!(lit_indices(&bar(63, 50)).len(), 12);
    assert_eq!(lit_indices(&bar(64, 50)).len(), 13);
    assert_eq!(lit_indices(&bar(127, 50)), (0..25).collect::<Vec<_>>());
    assert_eq!(lit_indices(&bar(255, 50)).len(), 25);
    assert_eq!(bar(127, 0xff)[24], MAX_VALUE);
}
