//! Texts of the sound tab, in each language.

use super::{Language, Texts};

/// Seconds in a minute, to write lengths as minutes and seconds.
const SECONDS_PER_MINUTE: f64 = 60.0;

impl Texts {
    pub(crate) const fn play_sound(&self) -> &'static str {
        self.pick("Play", "Reproducir")
    }

    pub(crate) const fn stop_sound(&self) -> &'static str {
        self.pick("Stop", "Detener")
    }

    pub(crate) const fn mixer(&self) -> &'static str {
        self.pick("Mixer", "Mezclador")
    }

    pub(crate) const fn open_mixer(&self) -> &'static str {
        self.pick("Audio mixer", "Mezclador de audio")
    }

    pub(crate) const fn mixer_hint(&self) -> &'static str {
        self.pick(
            "The groups of sounds and the volume each starts at. Sound players and sounds play in a group; scripts change a group's volume with Audio.set_group_volume. Saved in project.toml.",
            "Los grupos de sonidos y el volumen con el que empieza cada uno. Los reproductores y los sonidos suenan en un grupo; los scripts cambian el volumen de un grupo con Audio.set_group_volume. Se guarda en project.toml.",
        )
    }

    pub(crate) const fn add_group(&self) -> &'static str {
        self.pick("Add group", "Añadir grupo")
    }

    pub(crate) const fn remove_group(&self) -> &'static str {
        self.pick("Remove", "Quitar")
    }

    pub(crate) const fn no_group(&self) -> &'static str {
        self.pick("None", "Ninguno")
    }

    pub(crate) const fn sound_group(&self) -> &'static str {
        self.pick("Group", "Grupo")
    }

    pub(crate) const fn sound_player_section(&self) -> &'static str {
        self.pick("Sound player", "Reproductor de sonido")
    }

    pub(crate) const fn sound_file(&self) -> &'static str {
        self.pick("Sound", "Sonido")
    }

    pub(crate) const fn volume(&self) -> &'static str {
        self.pick("Volume", "Volumen")
    }

    pub(crate) const fn looping(&self) -> &'static str {
        self.pick("Loop", "Repetir")
    }

    pub(crate) const fn play_on_start(&self) -> &'static str {
        self.pick("Play on start", "Sonar al empezar")
    }

    pub(crate) const fn no_sounds_hint(&self) -> &'static str {
        self.pick(
            "Copy a sound (.wav, .ogg, .mp3 or .flac) into the assets folder first.",
            "Copia antes un sonido (.wav, .ogg, .mp3 o .flac) en la carpeta assets.",
        )
    }

    pub(crate) const fn sound_inspector_hint(&self) -> &'static str {
        self.pick(
            "A sound of the project. Play it to hear it; scripts load it with Sound.load.",
            "Un sonido del proyecto. Reprodúcelo para escucharlo; los scripts lo cargan con Sound.load.",
        )
    }

    /// How long a sound is, how many channels it has and at what rate.
    pub(crate) fn sound_details(&self, seconds: f64, channels: u16, rate: u32) -> String {
        let minutes = (seconds / SECONDS_PER_MINUTE).floor();
        let rest = seconds - minutes * SECONDS_PER_MINUTE;
        let length = format!("{minutes}:{rest:06.3}");
        match self.language {
            Language::English => {
                let channels = match channels {
                    1 => "mono".to_owned(),
                    2 => "stereo".to_owned(),
                    other => format!("{other} channels"),
                };
                format!("{length} · {channels} · {rate} Hz")
            }
            Language::Spanish => {
                let channels = match channels {
                    1 => "mono".to_owned(),
                    2 => "estéreo".to_owned(),
                    other => format!("{other} canales"),
                };
                format!("{length} · {channels} · {rate} Hz")
            }
        }
    }

    pub(crate) fn sound_unreadable(&self, error: &str) -> String {
        match self.language {
            Language::English => format!("The sound cannot be read: {error}"),
            Language::Spanish => format!("No se puede leer el sonido: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_are_minutes_and_seconds() {
        let texts = Language::English.texts();

        assert_eq!(
            texts.sound_details(75.5, 2, 44_100),
            "1:15.500 · stereo · 44100 Hz"
        );
        assert_eq!(
            texts.sound_details(0.25, 1, 22_050),
            "0:00.250 · mono · 22050 Hz"
        );
    }
}
