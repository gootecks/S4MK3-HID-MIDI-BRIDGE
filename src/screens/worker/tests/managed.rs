use super::*;
use crossbeam_channel::{bounded, unbounded};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Open,
    Write(Display, Vec<u8>),
    Released,
    Status(u64, State),
}

struct Release(crossbeam_channel::Sender<Event>);

impl Drop for Release {
    fn drop(&mut self) {
        assert!(
            self.0.send(Event::Released).is_ok(),
            "screen test release event undelivered"
        );
    }
}

fn managed(
    enabled: bool,
    gate: Option<crossbeam_channel::Receiver<()>>,
    fail_first: bool,
) -> anyhow::Result<(Worker, crossbeam_channel::Receiver<Event>)> {
    let (events, received) = unbounded();
    let writer_events = events.clone();
    let mut attempts = 0;
    let worker = Worker::start_with_config(
        move || {
            attempts += 1;
            writer_events.send(Event::Open)?;
            let release = Release(writer_events.clone());
            let writer_events = writer_events.clone();
            let gate = if attempts == 1 { gate.clone() } else { None };
            let mut first = true;
            let fails = fail_first && attempts == 1;
            Ok(move |screen, frame: &Frame| {
                let _keep_owned = &release;
                let mut packet = Vec::new();
                frame.packet(screen, &mut packet);
                writer_events.send(Event::Write(screen, packet))?;
                if first {
                    first = false;
                    if let Some(gate) = &gate {
                        gate.recv_timeout(Duration::from_secs(2))?;
                    }
                    if fails {
                        anyhow::bail!("test transfer failure");
                    }
                }
                Ok(())
            })
        },
        enabled,
        Some(move |id, state| {
            assert!(
                events.send(Event::Status(id, state)).is_ok(),
                "screen test status undelivered"
            );
        }),
    )?;
    Ok((worker, received))
}

fn next(events: &crossbeam_channel::Receiver<Event>) -> anyhow::Result<Event> {
    Ok(events.recv_timeout(Duration::from_secs(2))?)
}

#[test]
fn on_before_initialized_input_reports_pending_without_claiming_usb() -> anyhow::Result<()> {
    // Given a worker that has not received its first initialized HID page.
    let (mut worker, events) = managed(false, None, false)?;
    // When the app asks for screens before the first input snapshot.
    worker.control().request(7, Action::On);
    // Then the request is pending, and publication unlocks both writes.
    assert_eq!(next(&events)?, Event::Status(7, State::Pending));
    worker.publish(pages(None));
    assert_eq!(next(&events)?, Event::Open);
    assert!(matches!(next(&events)?, Event::Write(Display::Left, _)));
    assert!(matches!(next(&events)?, Event::Write(Display::Right, _)));
    assert_eq!(next(&events)?, Event::Status(7, State::Active));
    drop(worker);
    Ok(())
}

#[test]
fn initial_generation_reports_active_only_after_both_panels_and_failure_after_release()
-> anyhow::Result<()> {
    // Given generation zero enabled before initialized input.
    for fails in [false, true] {
        let (mut worker, events) = managed(true, None, fails)?;
        // When the first HID page reaches the worker.
        worker.publish(pages(None));
        assert_eq!(next(&events)?, Event::Open);
        assert!(matches!(next(&events)?, Event::Write(Display::Left, _)));
        if fails {
            assert_eq!(next(&events)?, Event::Released);
            assert_eq!(next(&events)?, Event::Status(0, State::Failed));
        } else {
            assert!(matches!(next(&events)?, Event::Write(Display::Right, _)));
            assert_eq!(next(&events)?, Event::Status(0, State::Active));
        }
        drop(worker);
    }
    Ok(())
}

#[test]
fn live_off_releases_transport_before_inactive_and_on_restores_latest_pages() -> anyhow::Result<()>
{
    // Given both panels initialized and an acknowledged active generation.
    let (mut worker, events) = managed(false, None, false)?;
    let control = worker.control();
    worker.publish(pages(None));
    control.request(1, Action::On);
    assert_eq!(next(&events)?, Event::Status(1, State::Pending));
    assert_eq!(next(&events)?, Event::Open);
    assert!(matches!(next(&events)?, Event::Write(Display::Left, _)));
    assert!(matches!(next(&events)?, Event::Write(Display::Right, _)));
    assert_eq!(next(&events)?, Event::Status(1, State::Active));
    // When OFF closes the transport and input continues publishing new state.
    control.request(2, Action::Off);
    let mut blank = Frame::default();
    page::render(&page::Page::Empty, &mut blank);
    for screen in [Display::Left, Display::Right] {
        let mut expected = Vec::new();
        blank.packet(screen, &mut expected);
        assert_eq!(next(&events)?, Event::Write(screen, expected));
    }
    assert_eq!(next(&events)?, Event::Released);
    assert_eq!(next(&events)?, Event::Status(2, State::Inactive));
    worker.publish(pages(Some(true)));
    control.request(3, Action::On);
    assert_eq!(next(&events)?, Event::Status(3, State::Pending));
    assert_eq!(next(&events)?, Event::Open);
    let Event::Write(Display::Left, packet) = next(&events)? else {
        anyhow::bail!("latest left page missing")
    };
    let mut frame = Frame::default();
    page::render(&pages(Some(true))[0], &mut frame);
    let mut expected = Vec::new();
    frame.packet(Display::Left, &mut expected);
    assert_eq!(packet, expected);
    assert!(matches!(next(&events)?, Event::Write(Display::Right, _)));
    assert_eq!(next(&events)?, Event::Status(3, State::Active));
    drop(worker);
    Ok(())
}

#[test]
fn failed_screen_transfer_releases_before_failed_and_retry_reacquires() -> anyhow::Result<()> {
    // Given a transport that fails its first raster.
    let (mut worker, events) = managed(false, None, true)?;
    let control = worker.control();
    worker.publish(pages(None));
    control.request(11, Action::On);
    assert_eq!(next(&events)?, Event::Status(11, State::Pending));
    assert_eq!(next(&events)?, Event::Open);
    assert!(matches!(next(&events)?, Event::Write(Display::Left, _)));
    assert_eq!(next(&events)?, Event::Released);
    assert_eq!(next(&events)?, Event::Status(11, State::Failed));
    // When RETRY is received with a fresh page, no full bridge restart is needed.
    worker.publish(pages(Some(false)));
    control.request(12, Action::Retry);
    assert_eq!(next(&events)?, Event::Status(12, State::Pending));
    assert_eq!(next(&events)?, Event::Open);
    assert!(matches!(next(&events)?, Event::Write(Display::Left, _)));
    assert!(matches!(next(&events)?, Event::Write(Display::Right, _)));
    assert_eq!(next(&events)?, Event::Status(12, State::Active));
    drop(worker);
    Ok(())
}

#[test]
fn blocked_usb_does_not_block_control_or_publish_stale_active() -> anyhow::Result<()> {
    // Given an in-flight first transfer stopped at a subscribed gate.
    let (resume, gate) = bounded(1);
    let (mut worker, events) = managed(false, Some(gate), false)?;
    let control = worker.control();
    worker.publish(pages(None));
    control.request(21, Action::On);
    assert_eq!(next(&events)?, Event::Status(21, State::Pending));
    assert_eq!(next(&events)?, Event::Open);
    assert!(matches!(next(&events)?, Event::Write(Display::Left, _)));
    // When OFF supersedes ON, publication and requests complete before USB resumes.
    control.request(22, Action::Off);
    worker.publish(pages(Some(true)));
    resume.send(())?;
    assert!(matches!(next(&events)?, Event::Write(Display::Left, _)));
    let event = next(&events)?;
    assert!(
        matches!(event, Event::Write(Display::Right, _)),
        "{event:?}"
    );
    assert_eq!(next(&events)?, Event::Released);
    assert_eq!(next(&events)?, Event::Status(22, State::Inactive));
    // Then no active acknowledgement for the stale generation was sent.
    control.request(23, Action::On);
    assert_eq!(next(&events)?, Event::Status(23, State::Pending));
    assert_eq!(next(&events)?, Event::Open);
    assert!(matches!(next(&events)?, Event::Write(Display::Left, _)));
    assert!(matches!(next(&events)?, Event::Write(Display::Right, _)));
    assert_eq!(next(&events)?, Event::Status(23, State::Active));
    drop(worker);
    Ok(())
}
