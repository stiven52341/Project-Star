//! The system's audio device, fed by the mixer. Only this module knows cpal.

use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, Stream, StreamConfig};
use star_core::{EngineError, EngineResult};

use crate::audio::lock;
use crate::{Audio, Mixer, OutputFormat};

/// The game's audio output, open while this value lives. It stays on the
/// thread that opened it (some systems require it); hand [`Audio`] copies
/// to the rest of the game instead.
pub struct AudioOutput {
    audio: Audio,
    /// Kept so the device keeps playing; dropping it closes the device.
    _stream: Stream,
}

impl std::fmt::Debug for AudioOutput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AudioOutput")
            .field("audio", &self.audio)
            .finish_non_exhaustive()
    }
}

impl AudioOutput {
    /// Opens the system's default audio device, in its preferred format.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Initialization`] if there is no audio device,
    /// or it cannot be opened in a sample format the engine writes.
    pub fn open_default() -> EngineResult<Self> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or_else(|| failure("there is no audio output device"))?;
        let supported = device
            .default_output_config()
            .map_err(|error| failure(format!("the audio device has no format: {error}")))?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let format = OutputFormat::new(config.channels, config.sample_rate)?;
        let mixer = Arc::new(Mutex::new(Mixer::new(format)));
        let stream = match sample_format {
            SampleFormat::F32 => open_stream::<f32>(&device, config, &mixer),
            SampleFormat::F64 => open_stream::<f64>(&device, config, &mixer),
            SampleFormat::I16 => open_stream::<i16>(&device, config, &mixer),
            SampleFormat::I32 => open_stream::<i32>(&device, config, &mixer),
            SampleFormat::U16 => open_stream::<u16>(&device, config, &mixer),
            SampleFormat::U8 => open_stream::<u8>(&device, config, &mixer),
            other => Err(failure(format!(
                "the audio device wants {other} samples, which the engine does not write"
            ))),
        }?;
        stream
            .play()
            .map_err(|error| failure(format!("the audio device does not start: {error}")))?;
        log::info!(
            "Playing audio at {} Hz, {} channels",
            format.sample_rate(),
            format.channels()
        );
        Ok(Self {
            audio: Audio::with_output(mixer),
            _stream: stream,
        })
    }

    /// A handle to play sounds on this output; copies share it.
    #[must_use]
    pub fn audio(&self) -> Audio {
        self.audio.clone()
    }
}

/// Opens a stream that asks the mixer for `T` samples whenever the device
/// needs more.
fn open_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    mixer: &Arc<Mutex<Mixer>>,
) -> EngineResult<Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let mixer = Arc::clone(mixer);
    // Reused between calls: it only grows if the device asks for more.
    let mut samples: Vec<f32> = Vec::new();
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _| {
                samples.resize(output.len(), 0.0);
                lock(&mixer).mix(&mut samples);
                for (sample, &value) in output.iter_mut().zip(&samples) {
                    *sample = T::from_sample(value);
                }
            },
            |error| log::warn!("Audio output: {error}"),
            None,
        )
        .map_err(|error| failure(format!("the audio device cannot be opened: {error}")))
}

fn failure(reason: impl Into<String>) -> EngineError {
    EngineError::Initialization {
        subsystem: "audio",
        reason: reason.into(),
    }
}
