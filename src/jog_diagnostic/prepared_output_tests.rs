use super::*;

#[test]
fn readiness_enables_real_force_without_center_resetting_the_clock() -> anyhow::Result<()> {
    for mode in [Mode::FaderTension, Mode::FaderContinuity] {
        let mut writes = Vec::new();
        let mut log = Vec::new();
        {
            let mut session = Session::new(
                fader_control(mode),
                |report| {
                    writes.push(report.to_vec());
                    Ok(report.len())
                },
                &mut log,
            )?;
            session.prepare(5)?;
            for index in 0_u32..=20 {
                let at = u64::from(index) * 50_000;
                session.handle(
                    Event::Sample(with_fader(frame(at, index + 1, [0, 0]), 2048)),
                    at,
                )?;
            }
            session.handle(
                Event::Sample(with_fader(frame(1_020_000, 22, [10, 0]), 4095)),
                1_020_000,
            )?;
            if mode.is_paced() {
                session.handle(Event::OutputTick, 1_020_000)?;
            }
            session.handle(
                Event::Sample(with_fader(frame(1_040_000, 23, [10, 0]), 2048)),
                1_040_000,
            )?;
            assert_eq!(session.remaining_duration_us(1_040_000, 5), Some(4_960_000));
            assert_eq!(session.remaining_duration_us(6_000_000, 5), Some(0));
        }
        assert!(writes.iter().any(|report| report[4..6] != [0, 0]));
        assert!(writes.iter().all(|report| report[9..11] == [0, 0]));
        assert_eq!(writes.last(), Some(&neutral_report().to_vec()));
        assert_eq!(
            rows(&log)?
                .iter()
                .filter(|row| row["kind"] == "ready")
                .count(),
            1
        );
    }
    Ok(())
}
