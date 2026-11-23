//! Texts about scenes and the values of their nodes, in each language.

use star_engine::prelude::{BodyKind, TextureFilter};

use super::{Language, Texts};

impl Texts {
    pub(crate) fn scene_unreadable(&self, reason: &str) -> String {
        match self.language {
            Language::English => format!("Could not open the scene: {reason}"),
            Language::Spanish => format!("No se pudo abrir la escena: {reason}"),
        }
    }

    pub(crate) fn scene_not_saved(&self, reason: &str) -> String {
        match self.language {
            Language::English => format!("Could not save the scene: {reason}"),
            Language::Spanish => format!("No se pudo guardar la escena: {reason}"),
        }
    }

    pub(crate) const fn body_kind_name(&self, kind: BodyKind) -> &'static str {
        match kind {
            BodyKind::Dynamic => self.pick("Dynamic", "Dinámico"),
            BodyKind::Static => self.pick("Static", "Estático"),
            BodyKind::Kinematic => self.pick("Kinematic", "Cinemático"),
        }
    }

    pub(crate) const fn filter_name(&self, filter: TextureFilter) -> &'static str {
        match filter {
            TextureFilter::Smooth => self.pick("Smooth", "Suave"),
            TextureFilter::Pixelated => self.pick("Pixelated", "Pixelado"),
        }
    }

    /// The name of a kind of light: `directional` is true for the sun's
    /// kind, false for a lamp's.
    pub(crate) const fn light_kind_name(&self, directional: bool) -> &'static str {
        if directional {
            self.pick("Directional", "Direccional")
        } else {
            self.pick("Point", "Puntual")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_names_are_in_the_language_shown() {
        assert_eq!(
            Language::Spanish.texts().body_kind_name(BodyKind::Static),
            "Estático"
        );
        assert_eq!(
            Language::English
                .texts()
                .filter_name(TextureFilter::Pixelated),
            "Pixelated"
        );
    }
}
