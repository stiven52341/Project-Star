//! Texts about scenes and the values of their nodes, in each language.

use star_engine::prelude::{BodyKind, TextureFilter};
use star_script::ShowFps;

use super::{Language, Texts};
use crate::scene_editor::{LightKind, Tool};

impl Texts {
    pub(crate) const fn tool_name(&self, tool: Tool) -> &'static str {
        match tool {
            Tool::View => self.pick("View", "Ver"),
            Tool::Move => self.pick("Move", "Mover"),
            Tool::Rotate => self.pick("Rotate", "Rotar"),
            Tool::Scale => self.pick("Scale", "Escalar"),
        }
    }

    pub(crate) const fn tool_hint(&self, tool: Tool) -> &'static str {
        match tool {
            Tool::View => self.pick(
                "Dragging only moves the camera (Q)",
                "Arrastrar solo mueve la cámara (Q)",
            ),
            Tool::Move => self.pick(
                "Drag the arrows or the square to move the node selected (W)",
                "Arrastra las flechas o el cuadro para mover el nodo seleccionado (W)",
            ),
            Tool::Rotate => self.pick(
                "Drag a ring to turn the node selected (E)",
                "Arrastra un anillo para rotar el nodo seleccionado (E)",
            ),
            Tool::Scale => self.pick(
                "Drag a box to scale along an axis, or the square to scale evenly (R)",
                "Arrastra una caja para escalar en un eje, o el cuadro para escalar por igual (R)",
            ),
        }
    }

    /// The title of the section of the `export` fields of `class`.
    pub(crate) fn exported_section(&self, class: &str) -> String {
        match self.language {
            Language::English => format!("{class} (export fields)"),
            Language::Spanish => format!("{class} (campos export)"),
        }
    }

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

    /// The Project menu's entry of when the game shows its frames per
    /// second.
    pub(crate) const fn show_fps(&self) -> &'static str {
        self.pick("Show FPS", "Mostrar FPS")
    }

    pub(crate) const fn show_fps_choice(&self, choice: ShowFps) -> &'static str {
        match choice {
            ShowFps::Editor => self.pick("When played from the editor", "Al jugar desde el editor"),
            ShowFps::Always => {
                self.pick("Always, exported games too", "Siempre, también exportado")
            }
            ShowFps::Never => self.pick("Never (F3 shows them)", "Nunca (F3 los muestra)"),
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
    pub(crate) const fn light_kind_name(&self, kind: LightKind) -> &'static str {
        match kind {
            LightKind::Directional => self.pick("Directional", "Direccional"),
            LightKind::Point => self.pick("Point", "Puntual"),
            LightKind::Spot => self.pick("Spot", "Foco"),
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
