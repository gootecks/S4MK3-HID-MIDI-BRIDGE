use super::*;
use crossbeam_channel::{bounded, unbounded};
use std::time::Duration;

fn pages(active: Option<bool>) -> [page::Page; 2] {
    [page::Page::Stems {
        shifted: false,
        fx: page::FxWarning::default(),
        backspin: None,
        pads: [page::PadState {
            active,
            held: false,
        }; 8],
    }; 2]
}

#[test]
fn selecting_info_flashes_without_republishing_or_redrawing_the_other_screen() -> anyhow::Result<()>
{
    // Given a worker subscribed before a selecting info page is published.
    let (sent, received) = unbounded();
    let mut worker = Worker::start_with(move || {
        Ok(move |display, frame: &Frame| {
            sent.send((display, frame.pixels.clone()))?;
            Ok(())
        })
    })?;
    let info = page::Page::Info(page::Info {
        jump: Some("8"),
        selecting: true,
        ..page::Info::default()
    });
    // When no further HID or MIDI publication occurs.
    worker.publish([info, page::Page::Empty]);
    let (side, bright) = received.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(side, Display::Left);
    assert_eq!(
        received.recv_timeout(Duration::from_secs(2))?.0,
        Display::Right
    );
    let (side, dim) = received.recv_timeout(Duration::from_secs(2))?;
    // Then the timer itself advances the left page; right stays coalesced.
    assert_eq!(side, Display::Left);
    assert_ne!(bright, dim);
    let mut expected = Frame::default();
    page::render_phase(&info, &mut expected, false);
    assert_eq!(dim, expected.pixels);
    drop(worker);
    Ok(())
}

#[test]
fn slow_transfer_does_not_block_publisher_and_latest_state_wins() -> anyhow::Result<()> {
    // Given a real worker whose first transfer is gated by an event.
    let (sent, received) = unbounded();
    let (resume, gate) = bounded(1);
    let mut worker = Worker::start_with(move || {
        let mut first = true;
        Ok(move |screen, frame: &Frame| {
            let mut packet = Vec::new();
            frame.packet(screen, &mut packet);
            sent.send(packet)?;
            if first {
                first = false;
                gate.recv_timeout(Duration::from_secs(2))?;
            }
            Ok(())
        })
    })?;
    worker.publish(pages(None));
    received.recv_timeout(Duration::from_secs(2))?;
    // When several updates arrive while USB is blocked.
    worker.publish(pages(Some(false)));
    worker.publish(pages(Some(true)));
    resume.send(())?;
    // Then the pending redraw is on, not the obsolete off state.
    received.recv_timeout(Duration::from_secs(2))?; // initial right
    let packet = received.recv_timeout(Duration::from_secs(2))?;
    let mut frame = Frame::default();
    page::render(&pages(Some(true))[0], &mut frame);
    let mut expected = Vec::new();
    frame.packet(Display::Left, &mut expected);
    assert_eq!(packet, expected);
    received.recv_timeout(Duration::from_secs(2))?; // latest right
    drop(worker);
    // Stop writes a stopped page to each screen and joins.
    let left = received.recv_timeout(Duration::from_secs(2))?;
    page::stopped(&mut frame);
    frame.packet(Display::Left, &mut expected);
    assert_eq!(left, expected);
    assert!(received.recv_timeout(Duration::from_secs(2)).is_ok());
    Ok(())
}

#[test]
fn changing_one_side_does_not_retransmit_the_other() -> anyhow::Result<()> {
    // Given both initial frames have reached the real worker's transport seam.
    let (sent, received) = unbounded();
    let mut worker = Worker::start_with(move || {
        Ok(move |screen, frame: &Frame| {
            let mut packet = Vec::new();
            frame.packet(screen, &mut packet);
            sent.send((screen, packet))?;
            Ok(())
        })
    })?;
    let initial = pages(None);
    worker.publish(initial);
    received.recv_timeout(Duration::from_secs(2))?;
    received.recv_timeout(Duration::from_secs(2))?;
    // When only the left page changes.
    worker.publish([pages(Some(false))[0], initial[1]]);
    assert_eq!(
        received.recv_timeout(Duration::from_secs(2))?.0,
        Display::Left
    );
    worker.publish([pages(Some(false))[0], pages(Some(true))[1]]);
    // Then the next frame is the new right page, not a redundant old right.
    let (screen, packet) = received.recv_timeout(Duration::from_secs(2))?;
    assert_eq!(screen, Display::Right);
    let mut frame = Frame::default();
    page::render(&pages(Some(true))[1], &mut frame);
    let mut expected = Vec::new();
    frame.packet(Display::Right, &mut expected);
    assert_eq!(packet, expected);
    drop(worker);
    Ok(())
}

#[test]
fn transport_failure_ends_worker_without_reaching_control_service() -> anyhow::Result<()> {
    // Given a screen transport that reports a real failure.
    let (failed, event) = bounded(1);
    let mut worker = Worker::start_with(move || {
        Ok(move |_, _: &Frame| {
            failed.send(())?;
            anyhow::bail!("screen transport unavailable")
        })
    })?;
    worker.publish(pages(None));
    // When the transfer fails, an exact event proves the path ran.
    event.recv_timeout(Duration::from_secs(2))?;
    drop(worker);
    // Then joining the screen worker does not propagate its failure to MIDI.
    Ok(())
}

mod managed;
