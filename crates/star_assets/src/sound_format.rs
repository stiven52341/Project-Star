//! The kinds of sound files the engine reads, and their decoding.

use std::io::Cursor;

use star_core::{EngineError, EngineResult, Sound};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;

use crate::AssetPath;

/// A sound file format, known by the extension of its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundFormat {
    /// Uncompressed WAV (`.wav`): short effects that must start at once.
    Wav,
    /// OGG Vorbis (`.ogg`, `.oga`): compressed and free, good for music.
    Ogg,
    /// MP3 (`.mp3`).
    Mp3,
    /// FLAC (`.flac`): compressed without losing anything.
    Flac,
}

impl SoundFormat {
    /// The format of the file at `path`, from its extension (in any case).
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if the extension is not that of a
    /// supported sound format.
    pub fn of(path: &AssetPath) -> EngineResult<Self> {
        let extension = path.extension().unwrap_or_default();
        let is = |name: &str| extension.eq_ignore_ascii_case(name);
        if is("wav") {
            Ok(Self::Wav)
        } else if is("ogg") || is("oga") {
            Ok(Self::Ogg)
        } else if is("mp3") {
            Ok(Self::Mp3)
        } else if is("flac") {
            Ok(Self::Flac)
        } else {
            Err(EngineError::Asset(format!(
                "\"{path}\" is not a supported sound: use .wav, .ogg, .mp3 or .flac"
            )))
        }
    }

    /// Decodes a whole sound of this format from its bytes.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if `bytes` is not a valid sound of
    /// this format, or the sound changes its channels or sample rate midway.
    pub fn decode(self, bytes: &[u8]) -> EngineResult<Sound> {
        decode_sound(bytes, self.extension())
    }

    /// The extension that hints symphonia which reader to try first.
    const fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Ogg => "ogg",
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
        }
    }
}

/// The channels and sample rate a sound is decoded with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Layout {
    channels: u16,
    sample_rate: u32,
}

/// The first sound track of a file, being read packet by packet.
struct Track<'bytes> {
    reader: Box<dyn FormatReader + 'bytes>,
    decoder: Box<dyn AudioDecoder>,
    id: u32,
    /// The format's extension, to name it in errors.
    extension: &'static str,
}

fn decode_sound(bytes: &[u8], extension: &'static str) -> EngineResult<Sound> {
    let mut track = Track::open(bytes, extension)?;
    let mut layout = None;
    let mut samples = Vec::new();
    let mut packet_samples = Vec::new();
    while let Some(packet_layout) = track.decode_next(&mut packet_samples)? {
        if *layout.get_or_insert(packet_layout) != packet_layout {
            return Err(EngineError::Asset(format!(
                "the {extension} sound changes its channels or sample rate midway"
            )));
        }
        samples.extend_from_slice(&packet_samples);
    }
    let layout = layout
        .ok_or_else(|| EngineError::Asset(format!("the {extension} sound has no samples")))?;
    Sound::from_interleaved(layout.channels, layout.sample_rate, samples)
}

impl<'bytes> Track<'bytes> {
    /// Finds the first sound track in `bytes` and its decoder.
    fn open(bytes: &'bytes [u8], extension: &'static str) -> EngineResult<Self> {
        let source = MediaSourceStream::new(
            Box::new(Cursor::new(bytes)),
            MediaSourceStreamOptions::default(),
        );
        let mut hint = Hint::new();
        hint.with_extension(extension);
        let reader = symphonia::default::get_probe()
            .probe(
                &hint,
                source,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|error| invalid(extension, &error))?;
        let no_sound = || EngineError::Asset(format!("the {extension} file has no sound"));
        let track = reader
            .default_track(TrackType::Audio)
            .ok_or_else(no_sound)?;
        let id = track.id;
        let parameters = track
            .codec_params
            .as_ref()
            .and_then(|parameters| parameters.audio())
            .ok_or_else(no_sound)?;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(parameters, &AudioDecoderOptions::default())
            .map_err(|error| invalid(extension, &error))?;
        Ok(Self {
            reader,
            decoder,
            id,
            extension,
        })
    }

    /// Decodes the next packet of the track into `samples`, interleaved,
    /// and gives its layout; `None` when the track has ended.
    fn decode_next(&mut self, samples: &mut Vec<f32>) -> EngineResult<Option<Layout>> {
        while let Some(packet) = self
            .reader
            .next_packet()
            .map_err(|error| self.invalid(&error))?
        {
            if packet.track_id != self.id {
                continue;
            }
            let buffer = match self.decoder.decode(&packet) {
                Ok(buffer) => buffer,
                // A damaged packet is skipped: the rest of the sound still plays.
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(error) => return Err(invalid(self.extension, &error)),
            };
            let spec = buffer.spec();
            let channels = u16::try_from(spec.channels().count()).map_err(|_| {
                EngineError::Asset(format!(
                    "the {} sound has too many channels",
                    self.extension
                ))
            })?;
            buffer.copy_to_vec_interleaved(samples);
            return Ok(Some(Layout {
                channels,
                sample_rate: spec.rate(),
            }));
        }
        Ok(None)
    }

    fn invalid(&self, error: &SymphoniaError) -> EngineError {
        invalid(self.extension, error)
    }
}

fn invalid(extension: &str, error: &SymphoniaError) -> EngineError {
    EngineError::Asset(format!("invalid {extension} sound: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sample rate of the test tones.
    const TONE_RATE: u32 = 22_050;
    /// Frames of the test tones: a tenth of a second.
    const TONE_FRAMES: usize = 2205;
    /// Lossy formats may add silence at the ends: MP3 encoders pad the
    /// sound to whole blocks and add their own delay.
    const FRAME_TOLERANCE: usize = 1500;
    /// Loudest sample of the test tones (they are at half volume), with
    /// room for what lossy formats change.
    const TONE_PEAK: f32 = 0.6;

    fn format_of(path: &str) -> EngineResult<SoundFormat> {
        SoundFormat::of(&AssetPath::new(path).unwrap())
    }

    fn assert_is_the_tone(sound: &Sound, channels: u16) {
        assert_eq!(sound.channels(), channels);
        assert_eq!(sound.sample_rate(), TONE_RATE);
        assert!(
            sound.frames().abs_diff(TONE_FRAMES) <= FRAME_TOLERANCE,
            "{} frames",
            sound.frames()
        );
        let peak = sound
            .samples()
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        assert!(peak > 0.1 && peak < TONE_PEAK, "peak {peak}");
    }

    #[test]
    fn the_format_comes_from_the_extension_in_any_case() {
        assert_eq!(format_of("jump.wav").unwrap(), SoundFormat::Wav);
        assert_eq!(format_of("music/THEME.OGG").unwrap(), SoundFormat::Ogg);
        assert_eq!(format_of("bell.oga").unwrap(), SoundFormat::Ogg);
        assert_eq!(format_of("theme.mp3").unwrap(), SoundFormat::Mp3);
        assert_eq!(format_of("theme.Flac").unwrap(), SoundFormat::Flac);
    }

    #[test]
    fn unknown_extensions_are_asset_errors() {
        for path in ["theme.midi", "theme"] {
            let error = format_of(path).unwrap_err();

            assert!(error.to_string().contains(path), "{error}");
        }
    }

    #[test]
    fn wav_sounds_are_decoded() {
        let stereo = SoundFormat::Wav
            .decode(include_bytes!("../test_data/tone_stereo.wav"))
            .unwrap();
        let mono = SoundFormat::Wav
            .decode(include_bytes!("../test_data/tone_mono.wav"))
            .unwrap();

        assert_is_the_tone(&stereo, 2);
        assert_eq!(stereo.frames(), TONE_FRAMES);
        assert_is_the_tone(&mono, 1);
    }

    #[test]
    fn ogg_sounds_are_decoded() {
        let sound = SoundFormat::Ogg
            .decode(include_bytes!("../test_data/tone_stereo.ogg"))
            .unwrap();

        assert_is_the_tone(&sound, 2);
    }

    #[test]
    fn mp3_sounds_are_decoded() {
        let sound = SoundFormat::Mp3
            .decode(include_bytes!("../test_data/tone_stereo.mp3"))
            .unwrap();

        assert_is_the_tone(&sound, 2);
    }

    #[test]
    fn flac_sounds_are_decoded() {
        let sound = SoundFormat::Flac
            .decode(include_bytes!("../test_data/tone_stereo.flac"))
            .unwrap();

        assert_is_the_tone(&sound, 2);
        assert_eq!(sound.frames(), TONE_FRAMES);
    }

    #[test]
    fn corrupt_data_is_an_asset_error() {
        let error = SoundFormat::Ogg.decode(b"not a sound").unwrap_err();

        assert!(matches!(error, EngineError::Asset(_)), "{error}");
    }
}
