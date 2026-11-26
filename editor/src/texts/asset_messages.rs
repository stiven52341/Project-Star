//! Texts about the files of the assets folder, in each language.

use super::{Language, Texts};
use crate::blueprint_editor::BlueprintTemplate;
use crate::workspace::{AssetFileError, NewAsset};

impl Texts {
    /// What the “Create” menu calls a kind of new file.
    pub(crate) const fn new_asset_name(&self, kind: NewAsset) -> &'static str {
        match kind {
            NewAsset::Folder => self.pick("Folder", "Carpeta"),
            NewAsset::Scene => self.pick("Scene", "Escena"),
            NewAsset::Blueprint => self.pick("Blueprint", "Blueprint"),
            NewAsset::Script => self.pick("StarScript script", "Script de StarScript"),
            NewAsset::Rust => self.pick("Rust script", "Script de Rust"),
            NewAsset::Keyframes => self.pick("Keyframe animations", "Animaciones de keyframes"),
            NewAsset::States => self.pick("State machine", "Máquina de estados"),
            NewAsset::Template(template) => template.name(),
        }
    }

    /// What a ready-made blueprint does, for its entry in the menu.
    pub(crate) const fn template_hint(&self, template: BlueprintTemplate) -> &'static str {
        match template {
            BlueprintTemplate::Movement2D => self.pick(
                "Moves the node with the actions left, right, up and down (WASD, arrows or the left stick). Adds the actions the project lacks.",
                "Mueve el nodo con las acciones left, right, up y down (WASD, flechas o el stick izquierdo). Añade al proyecto las acciones que le falten.",
            ),
            BlueprintTemplate::Movement3D => self.pick(
                "Moves the node over the ground with the actions left, right, up (forward) and down (back). Adds the actions the project lacks.",
                "Mueve el nodo por el suelo con las acciones left, right, up (adelante) y down (atrás). Añade al proyecto las acciones que le falten.",
            ),
            BlueprintTemplate::Spinner2D => self.pick(
                "Turns the node all the time, speed radians a second.",
                "Gira el nodo sin parar, speed radianes por segundo.",
            ),
            BlueprintTemplate::Spinner3D => self.pick(
                "Turns the node around its vertical axis, speed radians a second.",
                "Gira el nodo sobre su eje vertical, speed radianes por segundo.",
            ),
            BlueprintTemplate::Mover2D => self.pick(
                "Moves the node in a straight line at velocity_x and velocity_y units a second: bullets, clouds.",
                "Mueve el nodo en línea recta a velocity_x y velocity_y unidades por segundo: balas, nubes.",
            ),
            BlueprintTemplate::Mover3D => self.pick(
                "Moves the node in a straight line at velocity_x, velocity_y and velocity_z units a second.",
                "Mueve el nodo en línea recta a velocity_x, velocity_y y velocity_z unidades por segundo.",
            ),
            BlueprintTemplate::Lifetime => self.pick(
                "Removes the node from the game after seconds seconds: bullets, effects.",
                "Quita el nodo del juego pasados seconds segundos: balas, efectos.",
            ),
        }
    }

    /// The label of the field that names a new file of `kind`.
    pub(crate) const fn name_of_new(&self, kind: NewAsset) -> &'static str {
        match kind {
            NewAsset::Folder => self.pick("New folder:", "Carpeta nueva:"),
            NewAsset::Scene => self.pick("New scene:", "Escena nueva:"),
            NewAsset::Blueprint | NewAsset::Template(_) => {
                self.pick("New blueprint:", "Blueprint nuevo:")
            }
            NewAsset::Script => self.pick("New StarScript script:", "Script de StarScript nuevo:"),
            NewAsset::Rust => self.pick("New Rust script:", "Script de Rust nuevo:"),
            NewAsset::Keyframes => self.pick(
                "New keyframe animations:",
                "Animaciones de keyframes nuevas:",
            ),
            NewAsset::States => self.pick("New state machine:", "Máquina de estados nueva:"),
        }
    }

    /// Asks before deleting the file or folder called `name`.
    pub(crate) fn delete_question(&self, name: &str, is_folder: bool) -> String {
        match (self.language, is_folder) {
            (Language::English, false) => format!("Delete {name}? This cannot be undone."),
            (Language::English, true) => {
                format!("Delete the folder {name} and everything in it? This cannot be undone.")
            }
            (Language::Spanish, false) => format!("¿Eliminar {name}? No se puede deshacer."),
            (Language::Spanish, true) => {
                format!("¿Eliminar la carpeta {name} y todo lo que contiene? No se puede deshacer.")
            }
        }
    }

    pub(crate) fn asset_file_error(&self, error: &AssetFileError) -> String {
        match (error, self.language) {
            (AssetFileError::InvalidName, _) => self
                .pick(
                    "Names may have letters, digits, spaces, - and _, and start with a letter, a digit or _.",
                    "Los nombres pueden tener letras, números, espacios, - y _, y empezar por una letra, un número o _.",
                )
                .to_owned(),
            (AssetFileError::InvalidCodeName, _) => self
                .pick(
                    "Code is named with letters, digits and _, starting with a letter or _.",
                    "El código se nombra con letras, números y _, empezando por una letra o _.",
                )
                .to_owned(),
            (AssetFileError::AlreadyExists(path), Language::English) => {
                format!("{path} already exists.")
            }
            (AssetFileError::AlreadyExists(path), Language::Spanish) => {
                format!("Ya existe {path}.")
            }
            (AssetFileError::ClassExists(class), Language::English) => format!(
                "The class {class} already exists, in the engine or in another script or blueprint: scripts and blueprints are one program, so each class needs its own name."
            ),
            (AssetFileError::ClassExists(class), Language::Spanish) => format!(
                "La clase {class} ya existe, en el motor o en otro script o blueprint: los scripts y blueprints forman un solo programa, así que cada clase necesita su propio nombre."
            ),
            (AssetFileError::Io(error), Language::English) => format!("The disk refused: {error}"),
            (AssetFileError::Io(error), Language::Spanish) => {
                format!("El disco lo rechazó: {error}")
            }
        }
    }
}
