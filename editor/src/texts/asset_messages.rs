//! Texts about the files of the assets folder, in each language.

use super::{Language, Texts};
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
        }
    }

    /// The label of the field that names a new file of `kind`.
    pub(crate) const fn name_of_new(&self, kind: NewAsset) -> &'static str {
        match kind {
            NewAsset::Folder => self.pick("New folder:", "Carpeta nueva:"),
            NewAsset::Scene => self.pick("New scene:", "Escena nueva:"),
            NewAsset::Blueprint => self.pick("New blueprint:", "Blueprint nuevo:"),
            NewAsset::Script => self.pick("New StarScript script:", "Script de StarScript nuevo:"),
            NewAsset::Rust => self.pick("New Rust script:", "Script de Rust nuevo:"),
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
            (AssetFileError::Io(error), Language::English) => format!("The disk refused: {error}"),
            (AssetFileError::Io(error), Language::Spanish) => {
                format!("El disco lo rechazó: {error}")
            }
        }
    }
}
