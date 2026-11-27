//! The waveform of a sound: the loudest it gets, up and down, in each of a
//! few thousand slices of its length, worked out once, then drawn at any
//! width.

use eframe::egui;
use star_engine::star_core::Sound;

/// How many slices the waveform keeps: more than the pixels of a wide
/// screen.
const SLICES: usize = 4096;
/// The height of the waveform, in points.
pub(super) const WAVEFORM_HEIGHT: f32 = 180.0;
const WAVE_COLOR: egui::Color32 = egui::Color32::from_rgb(110, 190, 140);
const PLAYHEAD_COLOR: egui::Color32 = egui::Color32::from_rgb(240, 200, 80);
const CENTER_LINE_WIDTH: f32 = 1.0;

/// The lowest and highest sample of each slice of a sound, its channels
/// mixed, from -1 to 1.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Waveform {
    slices: Vec<[f32; 2]>,
}

impl Waveform {
    /// The waveform of `sound`.
    pub(super) fn of(sound: &Sound) -> Self {
        let channels = usize::from(sound.channels()).max(1);
        let frames = sound.frames();
        let count = SLICES.min(frames).max(1);
        let mut slices = vec![[0.0_f32; 2]; count];
        for (frame, samples) in sound.samples().chunks_exact(channels).enumerate() {
            // Frames are counted in `usize`, far below where `f32` errs.
            #[allow(clippy::cast_precision_loss)]
            let mixed = samples.iter().sum::<f32>() / channels as f32;
            let slice = frame * count / frames.max(1);
            if let Some([low, high]) = slices.get_mut(slice) {
                *low = low.min(mixed);
                *high = high.max(mixed);
            }
        }
        Self { slices }
    }

    /// Draws it across `rect`, with a line where `played` (0 to 1) of the
    /// sound is, if it plays.
    pub(super) fn draw(&self, painter: &egui::Painter, rect: egui::Rect, played: Option<f32>) {
        let middle = rect.center().y;
        let half = rect.height() / 2.0;
        painter.hline(
            rect.x_range(),
            middle,
            egui::Stroke::new(CENTER_LINE_WIDTH, egui::Color32::DARK_GRAY),
        );
        let columns = rect.width().max(1.0);
        // A waveform is a few thousand slices.
        #[allow(clippy::cast_precision_loss)]
        let count = self.slices.len() as f32;
        let mut column = 0.0;
        while column < columns {
            let [low, high] =
                self.peak_between(column / columns * count, (column + 1.0) / columns * count);
            let x = rect.left() + column;
            painter.vline(
                x,
                (middle - high * half)..=(middle - low * half).max(middle - high * half + 1.0),
                egui::Stroke::new(1.0, WAVE_COLOR),
            );
            column += 1.0;
        }
        if let Some(played) = played {
            let x = rect.left() + played.clamp(0.0, 1.0) * rect.width();
            painter.vline(x, rect.y_range(), egui::Stroke::new(2.0, PLAYHEAD_COLOR));
        }
    }

    /// The lowest and highest of the slices from `start` to `end`.
    fn peak_between(&self, start: f32, end: f32) -> [f32; 2] {
        // Both are within the slices, which are a few thousand.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (first, last) = (
            start as usize,
            (end.ceil() as usize).max(start as usize + 1),
        );
        self.slices
            .get(first..last.min(self.slices.len()))
            .unwrap_or_default()
            .iter()
            .fold([0.0, 0.0], |[low, high], [slice_low, slice_high]| {
                [low.min(*slice_low), high.max(*slice_high)]
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_keep_the_loudest_samples_of_mixed_channels() {
        // Two frames of two channels: loud up, then down.
        let sound = Sound::from_interleaved(2, 44_100, vec![1.0, 0.5, -1.0, -0.5]).unwrap();

        let waveform = Waveform::of(&sound);

        assert_eq!(waveform.slices, [[0.0, 0.75], [-0.75, 0.0]]);
        assert_eq!(waveform.peak_between(0.0, 2.0), [-0.75, 0.75]);
    }

    #[test]
    fn long_sounds_are_cut_into_a_few_thousand_slices() {
        let sound = Sound::from_interleaved(1, 44_100, vec![0.25; SLICES * 3]).unwrap();

        let waveform = Waveform::of(&sound);

        assert_eq!(waveform.slices.len(), SLICES);
        assert!(
            waveform
                .slices
                .iter()
                .all(|&[_, high]| (high - 0.25).abs() < 1e-6)
        );
    }
}
