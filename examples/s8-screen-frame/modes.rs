//! The frame plans behind each command.

use std::time::Duration;

use anyhow::ensure;
use s4mk3_hid_midi_bridge::device::s8::screen::pattern::{self, canvas, pixel_buffer, stamp, wire};
use s4mk3_hid_midi_bridge::device::s8::screen::{self, PIXEL_BYTES, Panel};
use serde_json::json;

use crate::run::{Job, MAX_DURATION, MAX_FRAMES, Plan, STAGE_GAP, Stage};

pub const MAX_RATE: u32 = 60;
const MAX_STAGES: usize = 8;

pub fn single(step: &'static str, panel: Panel, frame: Vec<u8>) -> Job {
    let copy = frame.clone();
    Job {
        step,
        raw: Some(frame),
        stages: vec![Stage {
            label: format!("one frame to the {} panel", name(panel)),
            params: json!({ "panel": name(panel) }),
            plan: Plan {
                total: 1,
                panels: 1,
                tag: String::new(),
                interval: None,
            },
            build: Box::new(move |_, out| {
                out.clone_from(&copy);
                panel
            }),
        }],
    }
}

pub fn pattern_frame(panel: Panel, swap_rb: bool) -> (Vec<u8>, Vec<u16>) {
    let canvas = canvas(panel);
    let mut bytes = pixel_buffer();
    wire(&canvas, swap_rb, &mut bytes);
    let mut frame = Vec::new();
    screen::full(&mut frame, panel, &bytes);
    (frame, canvas)
}

pub fn solid(panel: Panel, pair: [u8; 4]) -> Vec<u8> {
    let mut frame = Vec::new();
    screen::solid(&mut frame, panel, pair);
    frame
}

pub fn partial_skip(panel: Panel) -> Vec<u8> {
    let mut frame = Vec::new();
    pattern::partial_skip(&mut frame, panel);
    frame
}

pub fn partial_rect(panel: Panel) -> Vec<u8> {
    let mut frame = Vec::new();
    pattern::partial_rect(&mut frame, panel);
    frame
}

/// The test pattern with a frame counter and a moving bar, per panel.
struct Animator {
    bases: [Vec<u16>; 2],
    scratch: Vec<u16>,
    bytes: Box<[u8; PIXEL_BYTES]>,
    swap_rb: bool,
}

impl Animator {
    fn new(swap_rb: bool) -> Self {
        Self {
            bases: [canvas(Panel::Left), canvas(Panel::Right)],
            scratch: Vec::new(),
            bytes: pixel_buffer(),
            swap_rb,
        }
    }

    fn frame(&mut self, panel: Panel, number: usize, out: &mut Vec<u8>) {
        self.scratch
            .clone_from(&self.bases[usize::from(panel.index())]);
        stamp(&mut self.scratch, u32::try_from(number).unwrap_or(u32::MAX));
        wire(&self.scratch, self.swap_rb, &mut self.bytes);
        screen::full(out, panel, &self.bytes);
    }
}

/// One stage per entry of `rates`: that many frames per second on each of
/// `panels` for `seconds`, with the interface held across stages. With two
/// panels the transfers alternate, so the transfer rate is twice the rate.
pub fn stream(panels: &[Panel], rates: &[u32], seconds: u32, swap_rb: bool) -> anyhow::Result<Job> {
    ensure!(
        (1..=MAX_STAGES).contains(&rates.len()),
        "give 1 to {MAX_STAGES} rates"
    );
    ensure!(
        rates.iter().all(|rate| (1..=MAX_RATE).contains(rate)),
        "rates must be 1 to {MAX_RATE} frames per second"
    );
    let stages = u32::try_from(rates.len())?;
    let time = Duration::from_secs(u64::from(seconds)) * stages + STAGE_GAP * (stages - 1);
    ensure!(
        seconds >= 1 && time <= MAX_DURATION,
        "{} s of streaming and gaps exceed the cap of {} s",
        time.as_secs(),
        MAX_DURATION.as_secs()
    );
    let mut total = 0;
    for rate in rates {
        total += usize::try_from(rate * seconds)? * panels.len();
    }
    ensure!(
        total <= MAX_FRAMES,
        "{total} frames exceed the cap of {MAX_FRAMES}"
    );
    let count = u32::try_from(panels.len())?;
    let chosen: Vec<&str> = panels.iter().copied().map(name).collect();
    let stages = rates
        .iter()
        .map(|&rate| {
            let mut animator = Animator::new(swap_rb);
            let panels = panels.to_vec();
            Stage {
                label: format!("{rate} frames/s per panel, {}", chosen.join("+")),
                params: json!({
                    "rate_per_panel": rate,
                    "seconds": seconds,
                    "panels": chosen,
                    "swap_rb": swap_rb,
                }),
                plan: Plan {
                    total: usize::try_from(rate * seconds).unwrap_or_default() * panels.len(),
                    panels: panels.len(),
                    tag: format!("{rate}fps"),
                    interval: Some(Duration::from_secs(1).div_f64(f64::from(rate * count))),
                },
                build: Box::new(move |seq, out| {
                    let panel = panels[seq % panels.len()];
                    animator.frame(panel, seq / panels.len(), out);
                    panel
                }),
            }
        })
        .collect();
    Ok(Job {
        step: "stream",
        raw: None,
        stages,
    })
}

/// `frames` full frames to `panel`, back to back.
pub fn burst(panel: Panel, frames: u32, swap_rb: bool) -> anyhow::Result<Job> {
    let total = usize::try_from(frames)?;
    ensure!(
        (1..=MAX_FRAMES).contains(&total),
        "frames must be 1 to {MAX_FRAMES}"
    );
    let mut animator = Animator::new(swap_rb);
    Ok(Job {
        step: "burst",
        raw: None,
        stages: vec![Stage {
            label: format!("{frames} frames back to back to the {} panel", name(panel)),
            params: json!({ "frames": frames, "panel": name(panel), "swap_rb": swap_rb }),
            plan: Plan {
                total,
                panels: 1,
                tag: String::new(),
                interval: None,
            },
            build: Box::new(move |seq, out| {
                animator.frame(panel, seq, out);
                panel
            }),
        }],
    })
}

const fn name(panel: Panel) -> &'static str {
    match panel {
        Panel::Left => "left",
        Panel::Right => "right",
    }
}
