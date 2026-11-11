//! What Djay reports back for the four decks, decoded from the feedback
//! addresses of `catalog::outputs`.

use std::time::Duration;

use super::catalog::{FEEDBACK_CHANNEL, ROOT_OUTPUTS, RootFamily, embedded_number};
use crate::full::feedback::Message;

const DECK_COUNT: usize = 4;
const CUE_NOTES: std::ops::Range<usize> = 40..48;
const SAMPLER_NOTES: std::ops::RangeInclusive<u8> = 48..=55;
const NOTE_OFF: u8 = 0x80;
const NOTE_ON: u8 = 0x90;
const CONTROLLER: u8 = 0xb0;

#[derive(Clone, Debug)]
pub struct FeedbackState {
    notes: [[Option<u8>; 128]; DECK_COUNT],
    playing: [Option<bool>; DECK_COUNT],
    loaded: [Option<bool>; DECK_COUNT],
    progress: [Option<u8>; DECK_COUNT],
}

impl Default for FeedbackState {
    fn default() -> Self {
        Self {
            notes: [[None; 128]; DECK_COUNT],
            playing: [None; DECK_COUNT],
            loaded: [None; DECK_COUNT],
            progress: [None; DECK_COUNT],
        }
    }
}

/// The deck (0-3) a root output's data byte belongs to.
fn root_deck(family: RootFamily, kind: u8, number: u8) -> Option<usize> {
    ROOT_OUTPUTS
        .iter()
        .find(|root| root.family == family && root.kind.message_type() == kind)
        .and_then(|root| number.checked_sub(root.base))
        .map(usize::from)
        .filter(|deck| *deck < DECK_COUNT)
}

impl FeedbackState {
    /// Applies one message; true when something an LED shows changed.
    pub fn apply(&mut self, message: Message, _now: Duration) -> bool {
        let channel = message.status & 0x0f;
        let kind = message.status & 0xf0;
        if channel == FEEDBACK_CHANNEL {
            return self.apply_root(kind, message);
        }
        if !matches!(kind, NOTE_OFF | NOTE_ON)
            || usize::from(channel) >= DECK_COUNT
            || !embedded_number(message.number)
        {
            return false;
        }
        let value = if kind == NOTE_OFF { 0 } else { message.value };
        let deck = usize::from(channel);
        let changed = self.notes[deck][usize::from(message.number)] != Some(value);
        self.notes[deck][usize::from(message.number)] = Some(value);
        if SAMPLER_NOTES.contains(&message.number) {
            // Decks A/C share sampler bank 1 and B/D bank 2.
            self.notes[deck ^ 2][usize::from(message.number)] = Some(value);
        }
        changed
    }

    fn apply_root(&mut self, kind: u8, message: Message) -> bool {
        let type_of = match kind {
            CONTROLLER => super::catalog::CONTROLLER_TYPE,
            NOTE_OFF | NOTE_ON => super::catalog::NOTE_TYPE,
            _ => return false,
        };
        if let Some(deck) = root_deck(RootFamily::Playing, type_of, message.number) {
            let playing = Some(message.value != 0);
            let changed = self.playing[deck] != playing;
            self.playing[deck] = playing;
            return changed;
        }
        if let Some(deck) = root_deck(RootFamily::Progress, type_of, message.number) {
            let changed = self.progress[deck] != Some(message.value);
            self.progress[deck] = Some(message.value);
            return changed;
        }
        let Some(deck) = root_deck(RootFamily::Loaded, type_of, message.number) else {
            return false;
        };
        let loaded = kind == NOTE_ON && message.value != 0;
        let changed = self.loaded[deck] != Some(loaded);
        self.loaded[deck] = Some(loaded);
        if !loaded {
            self.notes[deck][CUE_NOTES].fill(None);
            self.playing[deck] = None;
            self.progress[deck] = None;
        }
        changed
    }

    /// The last velocity of `number` on `deck` (0-3); `None` before any feedback.
    pub fn note(&self, deck: usize, number: u8) -> Option<u8> {
        *self.notes.get(deck)?.get(usize::from(number))?
    }

    /// True when Djay last reported `number` on `deck` as on.
    pub fn note_on(&self, deck: usize, number: u8) -> bool {
        self.note(deck, number).is_some_and(|value| value != 0)
    }

    pub fn is_playing(&self, deck: usize) -> bool {
        self.playing.get(deck).copied().flatten() == Some(true)
    }

    #[cfg(test)]
    pub fn is_loaded(&self, deck: usize) -> Option<bool> {
        self.loaded.get(deck).copied().flatten()
    }

    /// Whole-track progress, 0-127.
    pub fn progress(&self, deck: usize) -> Option<u8> {
        self.progress.get(deck).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::full::feedback::Decoder;

    const NOW: Duration = Duration::ZERO;

    fn message(status: u8, number: u8, value: u8) -> Message {
        Message {
            status,
            number,
            value,
        }
    }

    fn root(family: RootFamily) -> Option<&'static crate::s8::catalog::RootOutput> {
        ROOT_OUTPUTS.iter().find(|root| root.family == family)
    }

    #[test]
    fn playing_follows_the_root_controller_per_deck() -> anyhow::Result<()> {
        let playing = root(RootFamily::Playing).ok_or_else(|| anyhow::anyhow!("playing"))?;
        let mut state = FeedbackState::default();
        let (channel, _, number) = playing.address(3);
        assert!(state.apply(message(0xb0 | channel, number, 127), NOW));
        assert!(state.is_playing(2));
        assert!(!state.is_playing(0));
        assert!(!state.apply(message(0xb0 | channel, number, 100), NOW));
        assert!(state.apply(message(0xb0 | channel, number, 0), NOW));
        assert!(!state.is_playing(2));
        Ok(())
    }

    #[test]
    fn progress_is_kept_per_deck() -> anyhow::Result<()> {
        let progress = root(RootFamily::Progress).ok_or_else(|| anyhow::anyhow!("progress"))?;
        let mut state = FeedbackState::default();
        let (channel, _, number) = progress.address(2);
        assert!(state.apply(message(0xb0 | channel, number, 64), NOW));
        assert_eq!(state.progress(1), Some(64));
        assert_eq!(state.progress(0), None);
        assert!(!state.apply(message(0xb0 | channel, number, 64), NOW));
        Ok(())
    }

    #[test]
    fn unloading_a_deck_clears_its_cues_playing_and_progress() -> anyhow::Result<()> {
        let loaded = root(RootFamily::Loaded).ok_or_else(|| anyhow::anyhow!("loaded"))?;
        let playing = root(RootFamily::Playing).ok_or_else(|| anyhow::anyhow!("playing"))?;
        let progress = root(RootFamily::Progress).ok_or_else(|| anyhow::anyhow!("progress"))?;
        let mut state = FeedbackState::default();
        let (channel, _, load) = loaded.address(1);
        assert!(state.apply(message(0x90 | channel, load, 127), NOW));
        assert_eq!(state.is_loaded(0), Some(true));
        state.apply(message(0x90, 41, 3), NOW);
        state.apply(message(0x90, 2, 127), NOW);
        state.apply(message(0xb0 | channel, playing.address(1).2, 127), NOW);
        state.apply(message(0xb0 | channel, progress.address(1).2, 9), NOW);
        assert!(state.apply(message(0x80 | channel, load, 0), NOW));
        assert_eq!(state.is_loaded(0), Some(false));
        assert_eq!(state.note(0, 41), None);
        assert!(!state.is_playing(0));
        assert_eq!(state.progress(0), None);
        assert!(state.note_on(0, 2), "sync is not cleared by an unload");
        Ok(())
    }

    #[test]
    fn deck_notes_decode_by_channel_with_note_off_as_zero() {
        let mut state = FeedbackState::default();
        assert!(state.apply(message(0x92, 1, 127), NOW));
        assert!(state.note_on(2, 1));
        assert!(!state.note_on(0, 1));
        assert!(!state.apply(message(0x92, 1, 127), NOW));
        assert!(state.apply(message(0x82, 1, 64), NOW));
        assert_eq!(state.note(2, 1), Some(0));
        assert!(state.apply(message(0x90, 40, 5), NOW));
        assert_eq!(state.note(0, 40), Some(5));
    }

    #[test]
    fn fx_notes_of_both_units_are_kept() {
        let mut state = FeedbackState::default();
        assert!(state.apply(message(0x90, 35, 127), NOW));
        assert!(state.apply(message(0x90, 39, 127), NOW));
        assert!(state.note_on(0, 35));
        assert!(state.note_on(0, 39));
    }

    #[test]
    fn sampler_notes_are_shared_between_decks_a_and_c() {
        let mut state = FeedbackState::default();
        assert!(state.apply(message(0x90, 48, 127), NOW));
        assert!(state.note_on(0, 48));
        assert!(state.note_on(2, 48));
        assert!(!state.note_on(1, 48));
        assert!(!state.note_on(3, 48));
    }

    #[test]
    fn unknown_messages_are_ignored() {
        let mut state = FeedbackState::default();
        for ignored in [
            message(0x90, 9, 127),
            message(0x90, 64, 127),
            message(0x94, 0, 127),
            message(0xb0, 1, 127),
            message(0xb6, 0, 127),
            message(0xb6, 12, 127),
            message(0x96, 4, 127),
            message(0xe0, 0, 1),
            message(0x99, 0, 127),
        ] {
            assert!(!state.apply(ignored, NOW), "{ignored:?}");
        }
        assert!(!state.is_playing(0));
        assert_eq!(state.note(0, 9), None);
        assert_eq!(state.note(9, 0), None);
    }

    #[test]
    fn the_shared_decoder_feeds_running_status_and_drops_realtime() {
        let mut state = FeedbackState::default();
        let mut decoder = Decoder::default();
        let mut changed = 0;
        decoder.feed(&[0x90, 1, 127, 0xf8, 2, 127, 0xfe, 4], |message| {
            changed += usize::from(state.apply(message, NOW));
        });
        decoder.feed(&[127], |message| {
            changed += usize::from(state.apply(message, NOW));
        });
        assert_eq!(changed, 3);
        assert!(state.note_on(0, 1));
        assert!(state.note_on(0, 2));
        assert!(state.note_on(0, 4));
    }
}
