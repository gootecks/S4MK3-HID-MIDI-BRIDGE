//! Adapt held MIDI owners and confirmed Djay state to the hardware screens.

use s4mk3_hid_midi_bridge::screens::page::{Info, PadState, Page};

use super::{Address, Translator, catalog::PadMode, feedback::FeedbackState};

mod key;

pub struct Output {
    enabled: bool,
    managed: bool,
    worker: Option<s4mk3_hid_midi_bridge::screens::Worker>,
}

impl Output {
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            managed: false,
            worker: None,
        }
    }

    pub fn managed(enabled: bool) -> anyhow::Result<Self> {
        Ok(Self {
            enabled,
            managed: true,
            worker: Some(s4mk3_hid_midi_bridge::screens::Worker::start_managed(
                enabled,
            )?),
        })
    }

    pub fn control(&self) -> Option<s4mk3_hid_midi_bridge::screens::Control> {
        self.worker
            .as_ref()
            .filter(|_| self.managed)
            .map(s4mk3_hid_midi_bridge::screens::Worker::control)
    }

    /// Publication follows initialized input and a completed native output tick.
    pub fn publish(&mut self, pages: [Page; 2]) {
        if self.managed {
            if let Some(worker) = &mut self.worker {
                worker.publish(pages);
            }
            return;
        }
        if !self.enabled {
            return;
        }
        if self.worker.is_none() {
            match s4mk3_hid_midi_bridge::screens::Worker::start() {
                Ok(worker) => self.worker = Some(worker),
                Err(error) => {
                    tracing::warn!(%error, "SCREENS_DISABLED: could not start screen worker");
                    self.enabled = false;
                    return;
                }
            }
        }
        if let Some(worker) = &mut self.worker {
            worker.publish(pages);
        }
    }

    /// Feedback can refresh an existing worker, but cannot start USB early.
    pub fn refresh(&mut self, pages: [Page; 2]) {
        if let Some(worker) = &mut self.worker {
            worker.refresh(pages);
        }
    }
}

pub(super) fn pages(translator: &Translator, feedback: &FeedbackState) -> [Page; 2] {
    std::array::from_fn(|side| match translator.pads[side] {
        PadMode::Hotcue | PadMode::Samples => {
            let deck = usize::from(translator.decks[side].channel());
            Page::Info(Info {
                jump: feedback.info.jump_size(deck),
                loop_active: feedback.notes[deck][6].map(|value| value != 0),
                loop_size: feedback.info.active_loop_size(deck),
                key: feedback.info.key_raw[deck].and_then(key::label),
                tempo_range: feedback.info.tempo_range,
                selecting: translator.move_selecting[deck],
            })
        }
        PadMode::Stems => {
            let deck = translator.decks[side];
            let notes = &feedback.notes[usize::from(deck.channel())];
            let shifted = translator.shifted[side];
            let solo: [Option<bool>; 4] =
                std::array::from_fn(|index| notes[60 + index].map(|value| value != 0));
            Page::Stems {
                shifted,
                pads: std::array::from_fn(|index| {
                    let number = [56_u8, 57, 58, 59, 60, 61, 62, 63][index];
                    let exclusive = shifted && index < 4;
                    let owner = Address {
                        channel: deck.channel() + if exclusive { 8 } else { 0 },
                        number,
                    };
                    PadState {
                        active: if exclusive {
                            solo.iter()
                                .enumerate()
                                .try_fold(true, |only, (stem, state)| {
                                    state.map(|active| only && active == (stem == index))
                                })
                        } else {
                            notes[usize::from(number)].map(|value| value != 0)
                        },
                        held: translator
                            .held
                            .values()
                            .flatten()
                            .any(|address| *address == owner),
                    }
                }),
            }
        }
    })
}

pub fn write_previews(path: &std::path::Path) -> anyhow::Result<()> {
    use s4mk3_hid_midi_bridge::screens::{Frame, page};
    std::fs::create_dir_all(path)?;
    let mut frame = Frame::default();
    for (deck_index, deck) in [
        super::Deck::A,
        super::Deck::B,
        super::Deck::C,
        super::Deck::D,
    ]
    .into_iter()
    .enumerate()
    {
        for (mode, name) in [
            (PadMode::Hotcue, "hotcue"),
            (PadMode::Samples, "samples"),
            (PadMode::Stems, "stems"),
        ] {
            for shifted in [false, true] {
                let translator = Translator {
                    decks: [deck; 2],
                    pads: [mode; 2],
                    shifted: [shifted; 2],
                    ..Translator::default()
                };
                page::render(
                    &pages(&translator, &FeedbackState::default())[0],
                    &mut frame,
                );
                frame.write_ppm(
                    &path.join(format!("deck-{deck_index}-{name}-shift-{shifted}.ppm")),
                )?;
            }
        }
    }
    for (name, muted, solo, shifted, held, known) in [
        ("stems-off", 0, 0, false, false, true),
        ("stems-mixed", 1, 2, false, false, true),
        ("stems-only", 1, 2, true, false, true),
        ("stems-multiple-solos", 0, 3, true, false, true),
        ("stems-held-unknown", 0, 0, false, true, false),
        ("stems-held-on", 4, 0, false, true, true),
    ] {
        let mut translator = Translator {
            pads: [PadMode::Stems; 2],
            shifted: [shifted; 2],
            ..Translator::default()
        };
        let mut feedback = FeedbackState::default();
        if known {
            for pad in 0_u8..8 {
                feedback.apply(
                    super::feedback::Message {
                        status: 0x90,
                        number: 56 + pad,
                        value: if (if pad < 4 { muted } else { solo }) & (1 << (pad % 4)) != 0 {
                            127
                        } else {
                            0
                        },
                    },
                    std::time::Duration::ZERO,
                );
            }
        }
        if held {
            translator.translate(
                &[super::Control::Button("left_pad_3", true)],
                None,
                &mut Vec::new(),
            )?;
        }
        page::render(&pages(&translator, &feedback)[0], &mut frame);
        frame.write_ppm(&path.join(format!("{name}.ppm")))?;
    }
    write_info_previews(path, &mut frame)?;
    page::stopped(&mut frame);
    frame.write_ppm(&path.join("stopped.ppm"))?;
    Ok(())
}

fn write_info_previews(
    path: &std::path::Path,
    frame: &mut s4mk3_hid_midi_bridge::screens::Frame,
) -> anyhow::Result<()> {
    use s4mk3_hid_midi_bridge::screens::page;
    for (name, selecting, bright, active, long) in [
        ("info-inactive", false, true, false, false),
        ("info-active", false, true, true, false),
        ("info-select-bright", true, true, false, false),
        ("info-select-dim", true, false, false, false),
        ("info-long", false, true, true, true),
    ] {
        let mut translator = Translator::default();
        translator.move_selecting[0] = selecting;
        let mut feedback = FeedbackState::default();
        for (status, number, value) in [
            (0xb0, if long { 80 } else { 86 }, 127),
            (0x90, 6, if active { 127 } else { 0 }),
            (0xb0, if long { 90 } else { 98 }, 127),
            (0xb0, 112, if long { 46 } else { 71 }),
            (0xb6, 8, if long { 95 } else { 5 }),
        ] {
            feedback.apply(
                super::feedback::Message {
                    status,
                    number,
                    value,
                },
                std::time::Duration::ZERO,
            );
        }
        page::render_phase(&pages(&translator, &feedback)[0], frame, bright);
        frame.write_ppm(&path.join(format!("{name}.ppm")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
