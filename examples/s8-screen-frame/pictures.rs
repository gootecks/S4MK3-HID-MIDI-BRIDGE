//! Pictures of what a dry run would put on the panels, drawn from the bytes
//! the tool builds by applying them to a decoded framebuffer.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context as _, bail};
use s4mk3_hid_midi_bridge::device::s8::screen::decode::Panels;
use s4mk3_hid_midi_bridge::device::s8::screen::stats::sample_positions;
use s4mk3_hid_midi_bridge::device::s8::screen::{self, Panel};

use crate::picture_file::{compare, write};
use crate::transport::StageInfo;

/// Frames per panel to draw when a stage is the only one of the run.
const SPREAD: usize = 5;

/// A frame number on one panel, counted from 0 within a stage.
#[derive(Clone, Copy, Debug)]
pub enum Pick {
    Last,
    At(usize),
}

impl std::str::FromStr for Pick {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        if text == "last" {
            return Ok(Self::Last);
        }
        text.parse()
            .map(Self::At)
            .map_err(|_| format!("'{text}' is neither 'last' nor a frame number"))
    }
}

pub struct Options {
    pub dir: PathBuf,
    pub name: String,
    /// Draw exactly these frames of every stage instead of the default rule.
    pub picks: Option<Vec<Pick>>,
    /// Compare with the files in `dir` instead of writing them.
    pub check: bool,
}

pub struct Pictures {
    options: Options,
    panels: Panels,
    tag: String,
    wanted: Vec<usize>,
    seen: [usize; 2],
    done: usize,
}

impl Pictures {
    /// `under` is a frame applied to both panels first, for steps that draw
    /// over what an earlier step left.
    pub fn new(options: Options, under: Option<[u8; 4]>) -> anyhow::Result<Self> {
        if !options.check {
            fs::create_dir_all(&options.dir)
                .with_context(|| format!("create {}", options.dir.display()))?;
        }
        let mut panels = Panels::default();
        if let Some(pair) = under {
            let mut frame = Vec::new();
            for panel in [Panel::Left, Panel::Right] {
                screen::solid(&mut frame, panel, pair);
                panels.apply(&frame)?;
            }
        }
        Ok(Self {
            options,
            panels,
            tag: String::new(),
            wanted: Vec::new(),
            seen: [0; 2],
            done: 0,
        })
    }

    /// Picks the frames to draw in this stage: with `--png-frames` those;
    /// otherwise, per panel, the last frame of every stage plus the first
    /// frame of the first stage, and for a single-stage run five frames
    /// spread from first to last.
    pub fn stage(&mut self, info: &StageInfo) {
        let per_panel = info.total / info.panels.max(1);
        let last = per_panel.saturating_sub(1);
        self.wanted = match &self.options.picks {
            Some(picks) => picks
                .iter()
                .map(|pick| match *pick {
                    Pick::Last => last,
                    Pick::At(number) => number,
                })
                .collect(),
            None if info.count == 1 => sample_positions(per_panel, SPREAD),
            None if info.index == 0 => sample_positions(per_panel, 2),
            None => vec![last],
        };
        info.tag.clone_into(&mut self.tag);
        self.seen = [0; 2];
    }

    /// Applies a frame the tool sends and draws it when it was picked.
    pub fn observe(&mut self, panel: Panel, frame: &[u8]) -> anyhow::Result<()> {
        let applied = self.panels.apply(frame)?;
        if applied != panel {
            bail!("frame addressed to another panel than the tool sent it to");
        }
        let index = usize::from(panel.index());
        let number = self.seen[index];
        self.seen[index] += 1;
        if !self.wanted.contains(&number) {
            return Ok(());
        }
        let side = if panel == Panel::Left {
            "left"
        } else {
            "right"
        };
        let tag = if self.tag.is_empty() {
            String::new()
        } else {
            format!("-{}", self.tag)
        };
        let path = self.options.dir.join(format!(
            "{}{tag}-{side}-f{number:04}.png",
            self.options.name
        ));
        let rgb = self.panels.rgb888(panel);
        if self.options.check {
            compare(&path, &rgb)?;
        } else {
            write(&path, &rgb)?;
        }
        self.done += 1;
        Ok(())
    }

    pub fn report(&self) -> String {
        let verb = if self.options.check {
            "checked"
        } else {
            "wrote"
        };
        format!(
            "{verb} {} pictures in {}",
            self.done,
            self.options.dir.display()
        )
    }
}
