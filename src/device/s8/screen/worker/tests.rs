use std::io::Cursor;
use std::sync::mpsc::{Receiver, Sender, channel};

use anyhow::{anyhow, ensure};

use super::*;
use crate::device::s8::screen::wire;

const WAIT: Duration = Duration::from_secs(5);

/// `BLACK_0` and its right-panel twin from `docs/s8/screen-frame-validation.md`.
const BLACK_LEFT: &str = "84000060000000000000000001e001100100ff00000000000300000040000000";
const BLACK_RIGHT: &str = "84000160000000000000000001e001100100ff00000000000300000040000100";

#[derive(Debug)]
enum Event {
    Claim,
    Send(Vec<u8>, Instant),
    Release,
}

#[derive(Default)]
struct Fake {
    events: Option<Sender<Event>>,
    fail_claim: bool,
    fail_send_from: Option<usize>,
    gate: Option<Receiver<()>>,
    sent: usize,
}

impl Transport for Fake {
    fn claim(&mut self) -> anyhow::Result<()> {
        self.emit(Event::Claim);
        ensure!(!self.fail_claim, "interface 6 is held by NIHardwareAgent");
        Ok(())
    }

    fn send(&mut self, frame: &[u8]) -> anyhow::Result<()> {
        self.emit(Event::Send(frame.to_vec(), Instant::now()));
        if let Some(gate) = &self.gate {
            gate.recv_timeout(WAIT).context("gate never opened")?;
        }
        self.sent += 1;
        ensure!(
            self.fail_send_from.is_none_or(|from| self.sent < from),
            "transfer timed out"
        );
        Ok(())
    }

    fn release(&mut self) {
        self.emit(Event::Release);
    }
}

impl Fake {
    fn emit(&self, event: Event) {
        if let Some(events) = &self.events {
            events.send(event).ok();
        }
    }
}

struct Rig {
    worker: Worker,
    events: Receiver<Event>,
    statuses: Receiver<Status>,
}

fn rig(interval: Duration, configure: impl FnOnce(&mut Fake)) -> anyhow::Result<Rig> {
    let (events_tx, events) = channel();
    let (status_tx, statuses) = channel();
    let mut fake = Fake {
        events: Some(events_tx),
        ..Fake::default()
    };
    configure(&mut fake);
    let worker = Worker::start(fake, Config { interval }, move |status| {
        status_tx.send(status.clone()).ok();
    })?;
    Ok(Rig {
        worker,
        events,
        statuses,
    })
}

impl Rig {
    fn event(&self) -> anyhow::Result<Event> {
        Ok(self.events.recv_timeout(WAIT)?)
    }

    fn status(&self) -> anyhow::Result<Status> {
        Ok(self.statuses.recv_timeout(WAIT)?)
    }

    fn sent(&self) -> anyhow::Result<(Vec<u8>, Instant)> {
        loop {
            match self.event()? {
                Event::Send(frame, at) => return Ok((frame, at)),
                Event::Claim => {}
                Event::Release => return Err(anyhow!("released before a frame")),
            }
        }
    }

    /// Stops the worker and returns what the transport and status saw after it.
    fn finish(self) -> (Vec<Event>, Vec<Status>) {
        let Self {
            worker,
            events,
            statuses,
        } = self;
        drop(worker);
        (events.try_iter().collect(), statuses.try_iter().collect())
    }
}

fn put(handle: &Handle, panel: Panel, fill: u8) -> anyhow::Result<()> {
    let pixels = Box::<[u8; PIXEL_BYTES]>::try_from(vec![fill; PIXEL_BYTES])
        .map_err(|_| anyhow!("pixel buffer size"))?;
    handle.submit(panel, &pixels);
    Ok(())
}

/// A full frame spelled out byte by byte, independent of the encoder.
fn expected(panel: u8, fill: u8) -> Vec<u8> {
    let mut frame = vec![0x84, 0x00, panel, 0x60, 0, 0, 0, 0];
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x01, 0xe0, 0x01, 0x10]);
    frame.extend_from_slice(&[0x00, 0x00, 0xff, 0x00]);
    frame.resize(frame.len() + PIXEL_BYTES, fill);
    frame.extend_from_slice(&[0x03, 0x00, 0x00, 0x00, 0x40, 0x00, panel, 0x00]);
    frame
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02x}").ok();
        text
    })
}

/// Panel and first pixel byte of a full frame, or the whole black frame.
fn describe(event: &Event) -> String {
    match event {
        Event::Claim => "claim".to_owned(),
        Event::Release => "release".to_owned(),
        Event::Send(frame, _) if frame.len() == FRAME_BYTES => {
            format!("full panel {} fill {:02x}", frame[2], frame[20])
        }
        Event::Send(frame, _) => format!("short {}", hex(frame)),
    }
}

fn described(events: &[Event]) -> Vec<String> {
    events.iter().map(describe).collect()
}

#[test]
fn pacing_constant_is_thirty_frames_per_second() {
    assert_eq!(MAX_FRAMES_PER_SECOND, 30);
    let total = FRAME_INTERVAL * 30;
    assert!(total <= Duration::from_secs(1));
    assert!(total > Duration::from_millis(999));
}

#[test]
fn full_frame_is_header_command_pixels_footer_for_each_panel() -> anyhow::Result<()> {
    let rig = rig(Duration::ZERO, |_| {})?;
    let handle = rig.worker.handle();
    put(&handle, Panel::Left, 0x11)?;
    assert_eq!(rig.sent()?.0, expected(0, 0x11));
    put(&handle, Panel::Right, 0x22)?;
    assert_eq!(rig.sent()?.0, expected(1, 0x22));
    Ok(())
}

#[test]
fn claims_once_and_walks_pending_active_inactive() -> anyhow::Result<()> {
    let rig = rig(Duration::ZERO, |_| {})?;
    assert_eq!(rig.status()?, Status::Pending);
    assert!(rig.events.try_recv().is_err(), "nothing is claimed early");
    let handle = rig.worker.handle();
    for round in 0..3 {
        put(&handle, Panel::Left, round)?;
        rig.sent()?;
    }
    assert_eq!(rig.status()?, Status::Active);
    let (events, statuses) = rig.finish();
    assert_eq!(
        described(&events),
        ["short ".to_owned() + BLACK_LEFT, "release".to_owned()]
    );
    assert_eq!(statuses, [Status::Inactive]);
    Ok(())
}

#[test]
fn stop_without_frames_releases_and_reports_inactive() -> anyhow::Result<()> {
    let rig = rig(Duration::ZERO, |_| {})?;
    let (events, statuses) = rig.finish();
    assert_eq!(described(&events), ["release"]);
    assert_eq!(statuses, [Status::Pending, Status::Inactive]);
    Ok(())
}

#[test]
fn newer_frame_replaces_an_unsent_one_while_the_transfer_blocks() -> anyhow::Result<()> {
    let (open, gate) = channel();
    let rig = rig(Duration::ZERO, |fake| fake.gate = Some(gate))?;
    let handle = rig.worker.handle();
    put(&handle, Panel::Left, 0xa1)?;
    assert_eq!(rig.sent()?.0, expected(0, 0xa1));
    put(&handle, Panel::Left, 0xb2)?;
    put(&handle, Panel::Left, 0xc3)?;
    for _ in 0..4 {
        open.send(())?;
    }
    assert_eq!(rig.sent()?.0, expected(0, 0xc3));
    let (events, _) = rig.finish();
    assert_eq!(
        described(&events),
        ["short ".to_owned() + BLACK_LEFT, "release".to_owned()]
    );
    Ok(())
}

#[test]
fn a_blocked_transfer_does_not_block_the_reader() -> anyhow::Result<()> {
    let (open, gate) = channel();
    let rig = rig(Duration::ZERO, |fake| fake.gate = Some(gate))?;
    let handle = rig.worker.handle();
    put(&handle, Panel::Left, 1)?;
    rig.sent()?;
    for round in 0..50 {
        put(&handle, Panel::Right, round)?;
    }
    open.send(())?;
    open.send(())?;
    open.send(())?;
    open.send(())?;
    drop(rig.finish());
    Ok(())
}

#[test]
fn one_panel_does_not_resend_the_other() -> anyhow::Result<()> {
    let rig = rig(Duration::ZERO, |_| {})?;
    let handle = rig.worker.handle();
    put(&handle, Panel::Left, 0x01)?;
    rig.sent()?;
    put(&handle, Panel::Right, 0x02)?;
    rig.sent()?;
    let (events, _) = rig.finish();
    assert_eq!(
        described(&events),
        [
            "short ".to_owned() + BLACK_LEFT,
            "short ".to_owned() + BLACK_RIGHT,
            "release".to_owned()
        ]
    );
    Ok(())
}

#[test]
fn a_panel_never_starts_transfers_faster_than_the_interval() -> anyhow::Result<()> {
    let interval = Duration::from_millis(60);
    let rig = rig(interval, |_| {})?;
    let handle = rig.worker.handle();
    put(&handle, Panel::Left, 1)?;
    let (_, first) = rig.sent()?;
    put(&handle, Panel::Left, 2)?;
    put(&handle, Panel::Right, 3)?;
    let mut left = None;
    let mut right = None;
    while left.is_none() || right.is_none() {
        let (frame, at) = rig.sent()?;
        if frame[2] == 0 {
            left = Some(at);
        } else {
            right = Some(at);
        }
    }
    let left = left.context("left frame")?;
    assert!(left.duration_since(first) >= interval);
    let right = right.context("right frame")?;
    assert!(
        right.duration_since(first) < interval * 20,
        "the other panel keeps its own clock"
    );
    Ok(())
}

#[test]
fn claim_failure_reports_failed_and_releases() -> anyhow::Result<()> {
    let rig = rig(Duration::ZERO, |fake| fake.fail_claim = true)?;
    assert_eq!(rig.status()?, Status::Pending);
    let handle = rig.worker.handle();
    put(&handle, Panel::Left, 1)?;
    assert_eq!(
        rig.status()?,
        Status::Failed("interface 6 is held by NIHardwareAgent".to_owned())
    );
    put(&handle, Panel::Left, 2)?;
    let (events, statuses) = rig.finish();
    assert_eq!(described(&events), ["claim", "release"]);
    assert!(statuses.is_empty());
    Ok(())
}

#[test]
fn transfer_failure_reports_failed_releases_and_sends_no_exit_frames() -> anyhow::Result<()> {
    let rig = rig(Duration::ZERO, |fake| fake.fail_send_from = Some(2))?;
    let handle = rig.worker.handle();
    put(&handle, Panel::Left, 1)?;
    assert_eq!(rig.sent()?.0, expected(0, 1));
    assert_eq!(rig.status()?, Status::Pending);
    assert_eq!(rig.status()?, Status::Active);
    put(&handle, Panel::Left, 2)?;
    assert_eq!(
        rig.status()?,
        Status::Failed("transfer timed out".to_owned())
    );
    let (events, _) = rig.finish();
    assert_eq!(described(&events), ["full panel 0 fill 02", "release"]);
    Ok(())
}

#[test]
fn stop_sends_black_to_each_panel_that_received_a_frame_then_inactive() -> anyhow::Result<()> {
    let rig = rig(Duration::ZERO, |_| {})?;
    let handle = rig.worker.handle();
    put(&handle, Panel::Left, 1)?;
    rig.sent()?;
    put(&handle, Panel::Right, 2)?;
    rig.sent()?;
    let (events, statuses) = rig.finish();
    let [Event::Send(left, _), Event::Send(right, _), Event::Release] = events.as_slice() else {
        return Err(anyhow!("unexpected events {events:?}"));
    };
    assert_eq!(left.len(), 32);
    assert_eq!(hex(left), BLACK_LEFT);
    assert_eq!(hex(right), BLACK_RIGHT);
    assert_eq!(
        statuses,
        [Status::Pending, Status::Active, Status::Inactive]
    );
    Ok(())
}

#[test]
fn abort_parks_the_panels_and_reports_the_reason() -> anyhow::Result<()> {
    let rig = rig(Duration::ZERO, |_| {})?;
    let handle = rig.worker.handle();
    put(&handle, Panel::Right, 1)?;
    rig.sent()?;
    handle.abort("bad panel 2");
    let (events, statuses) = rig.finish();
    assert_eq!(
        described(&events),
        ["short ".to_owned() + BLACK_RIGHT, "release".to_owned()]
    );
    assert_eq!(
        statuses,
        [
            Status::Pending,
            Status::Active,
            Status::Failed("bad panel 2".to_owned())
        ]
    );
    Ok(())
}

#[test]
fn a_panicking_status_callback_still_parks_releases_and_joins() -> anyhow::Result<()> {
    let (events_tx, events) = channel();
    let fake = Fake {
        events: Some(events_tx),
        ..Fake::default()
    };
    let worker = Worker::start(fake, Config::default(), |_| {
        std::panic::resume_unwind(Box::new("stdout closed"));
    })?;
    let handle = worker.handle();
    put(&handle, Panel::Left, 1)?;
    loop {
        if matches!(events.recv_timeout(WAIT)?, Event::Send(..)) {
            break;
        }
    }
    drop(worker);
    let events: Vec<Event> = events.try_iter().collect();
    let [Event::Send(black, _), Event::Release] = events.as_slice() else {
        return Err(anyhow!("unexpected events {events:?}"));
    };
    assert_eq!(hex(black), BLACK_LEFT);
    Ok(())
}

fn record(panel: u8, fill: u8) -> Vec<u8> {
    let mut bytes = wire::MAGIC.to_vec();
    bytes.extend_from_slice(&[panel, 0, 0, 0]);
    bytes.extend_from_slice(&261_120_u32.to_le_bytes());
    bytes.resize(wire::HEADER_BYTES + PIXEL_BYTES, fill);
    bytes
}

#[test]
fn records_from_a_stream_reach_the_transport_as_full_frames() -> anyhow::Result<()> {
    let rig = rig(Duration::ZERO, |_| {})?;
    let mut stream = record(0, 0x31);
    stream.extend(record(1, 0x42));
    wire::serve(Cursor::new(stream), &rig.worker.handle());
    assert_eq!(rig.sent()?.0, expected(0, 0x31));
    assert_eq!(rig.sent()?.0, expected(1, 0x42));
    let (_, statuses) = rig.finish();
    assert_eq!(
        statuses,
        [Status::Pending, Status::Active, Status::Inactive]
    );
    Ok(())
}

#[test]
fn a_protocol_error_stops_the_screen_path_and_drains_the_stream() -> anyhow::Result<()> {
    let rig = rig(Duration::ZERO, |_| {})?;
    let mut stream = record(0, 1);
    stream[0] = b'X';
    stream.extend(record(1, 2));
    let mut cursor = Cursor::new(stream);
    wire::serve(&mut cursor, &rig.worker.handle());
    assert_eq!(cursor.position(), u64::try_from(cursor.get_ref().len())?);
    assert_eq!(rig.status()?, Status::Pending);
    assert_eq!(
        rig.status()?,
        Status::Failed("bad record magic [58, 38, 46, 52]".to_owned())
    );
    let (events, _) = rig.finish();
    assert_eq!(described(&events), ["release"]);
    Ok(())
}

#[test]
fn failure_reasons_stay_on_one_line() {
    assert_eq!(Status::failed("a\nb\r\tc").to_string(), "failed a b  c");
    assert_eq!(Status::Pending.to_string(), "pending");
    assert_eq!(Status::Active.to_string(), "active");
    assert_eq!(Status::Inactive.to_string(), "inactive");
}
