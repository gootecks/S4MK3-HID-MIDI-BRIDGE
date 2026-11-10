//! Drives a job's stages through a transport and summarises each one.

use std::time::{Duration, Instant};

use s4mk3_hid_midi_bridge::device::s8::screen::Panel;
use s4mk3_hid_midi_bridge::device::s8::screen::stats::summarize;
use serde_json::{Value, json};

use crate::transport::Transport;

/// At most this many transfers in one invocation, all stages together.
pub const MAX_FRAMES: usize = 3600;
/// An invocation stops sending after this long, idle gaps included.
pub const MAX_DURATION: Duration = Duration::from_secs(60);
/// Pause between stages with the interface still claimed.
pub const STAGE_GAP: Duration = Duration::from_secs(2);

pub type Build = Box<dyn FnMut(usize, &mut Vec<u8>) -> Panel>;

pub struct Plan {
    pub total: usize,
    /// Time between the starts of transfers; `None` sends back to back.
    pub interval: Option<Duration>,
}

pub struct Stage {
    pub label: String,
    pub params: Value,
    pub plan: Plan,
    pub build: Build,
}

pub struct Job {
    pub step: &'static str,
    /// Set when the job is exactly one frame; `--dump` writes these bytes.
    pub raw: Option<Vec<u8>>,
    pub stages: Vec<Stage>,
}

#[derive(Default)]
pub struct Outcome {
    pub times: [Vec<Duration>; 2],
    pub bytes: u64,
    pub elapsed: Duration,
    /// Transfers that started a whole interval after their slot.
    pub late: u32,
    pub stopped: Option<&'static str>,
    pub error: Option<String>,
}

/// Runs every stage in order and prints a start line and one JSON summary
/// line per stage. Stops at the first transfer error and at the time cap.
/// Returns the error, if any.
pub fn run_job(job: &mut Job, transport: &mut dyn Transport, dry_run: bool) -> Option<String> {
    let started = Instant::now();
    let count = job.stages.len();
    for (index, stage) in job.stages.iter_mut().enumerate() {
        if index > 0 {
            transport.wait_until(Instant::now() + STAGE_GAP);
        }
        println!(
            "{:.1}s stage {}/{count}: {}",
            started.elapsed().as_secs_f64(),
            index + 1,
            stage.label
        );
        transport.stage(index);
        let outcome = drive(transport, &stage.plan, &mut stage.build, started);
        let line = summary(job.step, index, &stage.params, &outcome, dry_run);
        println!("{line}");
        if outcome.stopped.is_some() {
            return outcome.error;
        }
    }
    None
}

/// Sends `plan.total` frames, each slot timed from one start instant so
/// delays do not accumulate. `cap_start` is when the invocation began.
fn drive(
    transport: &mut dyn Transport,
    plan: &Plan,
    build: &mut Build,
    cap_start: Instant,
) -> Outcome {
    let mut outcome = Outcome::default();
    let mut frame = Vec::new();
    let start = Instant::now();
    for seq in 0..plan.total {
        if let Some(interval) = plan.interval {
            let slot = u32::try_from(seq).unwrap_or(u32::MAX);
            let deadline = start + interval.mul_f64(f64::from(slot));
            transport.wait_until(deadline);
            if Instant::now() > deadline + interval {
                outcome.late += 1;
            }
        }
        if cap_start.elapsed() >= MAX_DURATION {
            outcome.stopped = Some("time cap");
            break;
        }
        let panel = build(seq, &mut frame);
        match transport.send(seq, panel, &frame) {
            Ok(took) => {
                outcome.times[usize::from(panel.index())].push(took);
                outcome.bytes += u64::try_from(frame.len()).unwrap_or_default();
            }
            Err(error) => {
                outcome.error = Some(format!("{error:#}"));
                outcome.stopped = Some("transfer error");
                break;
            }
        }
    }
    outcome.elapsed = start.elapsed();
    outcome
}

/// One JSON object: what was asked, what was sent, rates and transfer times
/// per panel in milliseconds.
fn summary(step: &str, stage: usize, params: &Value, outcome: &Outcome, dry_run: bool) -> Value {
    let seconds = outcome.elapsed.as_secs_f64();
    let per_second = |count: f64| if seconds > 0.0 { count / seconds } else { 0.0 };
    let mut panels = serde_json::Map::new();
    for (name, times) in ["left", "right"].into_iter().zip(&outcome.times) {
        if times.is_empty() {
            continue;
        }
        let frames = u32::try_from(times.len()).unwrap_or(u32::MAX);
        panels.insert(
            name.to_owned(),
            json!({
                "frames": frames,
                "rate": per_second(f64::from(frames)),
                "transfer_ms": summarize(times),
            }),
        );
    }
    let frames: usize = outcome.times.iter().map(Vec::len).sum();
    json!({
        "step": step,
        "stage": stage,
        "params": params,
        "dry_run": dry_run,
        "frames_sent": frames,
        "bytes": outcome.bytes,
        "elapsed_s": seconds,
        "late": outcome.late,
        "stopped": outcome.stopped,
        "error": outcome.error,
        "panels": panels,
    })
}
