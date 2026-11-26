//! Texts of the frames editor and of a sprite's animations, in each
//! language.

use super::{Language, Texts};

impl Texts {
    /// The clips of a `.frames` file, in its inspector.
    pub(crate) const fn clips(&self) -> &'static str {
        self.pick("Clips", "Clips")
    }

    pub(crate) const fn new_clip(&self) -> &'static str {
        self.pick("New clip", "Nuevo clip")
    }

    pub(crate) const fn delete_clip(&self) -> &'static str {
        self.pick("Delete clip", "Borrar clip")
    }

    pub(crate) const fn clip_speed(&self) -> &'static str {
        self.pick("Speed (frames/s)", "Velocidad (fotogramas/s)")
    }

    pub(crate) const fn clip_looping(&self) -> &'static str {
        self.pick("Loop", "Repetir")
    }

    pub(crate) const fn clip_frames(&self) -> &'static str {
        self.pick("Frames", "Fotogramas")
    }

    pub(crate) const fn clear_frames(&self) -> &'static str {
        self.pick("Clear frames", "Vaciar fotogramas")
    }

    pub(crate) const fn remove_frame(&self) -> &'static str {
        self.pick("Remove this frame", "Quitar este fotograma")
    }

    pub(crate) const fn clip_preview(&self) -> &'static str {
        self.pick("Preview", "Vista previa")
    }

    pub(crate) const fn no_clip_hint(&self) -> &'static str {
        self.pick(
            "Create a clip in the inspector, then click the cells it shows, in order.",
            "Crea un clip en el inspector y haz clic en las celdas que muestra, en orden.",
        )
    }

    /// Under the sheet, while the clip `clip` is selected.
    pub(crate) fn sheet_hint(&self, clip: &str) -> String {
        match self.language {
            Language::English => {
                format!("Click a cell to add it to the clip \"{clip}\".")
            }
            Language::Spanish => {
                format!("Haz clic en una celda para añadirla al clip «{clip}».")
            }
        }
    }

    pub(crate) fn frames_unreadable(&self, reason: &str) -> String {
        match self.language {
            Language::English => format!("Could not open the animations: {reason}"),
            Language::Spanish => format!("No se pudieron abrir las animaciones: {reason}"),
        }
    }

    pub(crate) fn frames_not_saved(&self, reason: &str) -> String {
        match self.language {
            Language::English => format!("Could not save the animations: {reason}"),
            Language::Spanish => format!("No se pudieron guardar las animaciones: {reason}"),
        }
    }

    /// The menu entry of an image that makes a `.frames` file of it.
    pub(crate) const fn new_animations(&self) -> &'static str {
        self.pick("New animations", "Nuevas animaciones")
    }

    pub(crate) const fn new_animations_hint(&self) -> &'static str {
        self.pick(
            "Makes a .frames file of this image, to cut it into cells and make clips of them.",
            "Crea un archivo .frames de esta imagen, para cortarla en celdas y hacer clips con ellas.",
        )
    }

    /// The label of the row naming a new `.frames` file.
    pub(crate) const fn name_of_new_animations(&self) -> &'static str {
        self.pick("New animations:", "Animaciones nuevas:")
    }

    /// The `.frames` file of a sprite, in the inspector.
    pub(crate) const fn animations(&self) -> &'static str {
        self.pick("Animations", "Animaciones")
    }

    pub(crate) const fn animations_hint(&self) -> &'static str {
        self.pick(
            "A .frames file: the sprite takes its image and cells, and plays its clips.",
            "Un archivo .frames: el sprite toma su imagen y sus celdas, y reproduce sus clips.",
        )
    }

    pub(crate) const fn no_animations(&self) -> &'static str {
        self.pick("None", "Ninguna")
    }

    /// The clip a scene's sprite plays from the start.
    pub(crate) const fn starting_clip(&self) -> &'static str {
        self.pick("Starting clip", "Clip inicial")
    }

    pub(crate) const fn no_starting_clip(&self) -> &'static str {
        self.pick("None", "Ninguno")
    }

    /// The section of a node's 3D model, and the part the inspector adds.
    pub(crate) const fn model_section(&self) -> &'static str {
        self.pick("Model", "Modelo")
    }

    pub(crate) const fn model_file(&self) -> &'static str {
        self.pick("File", "Archivo")
    }

    /// Why a node cannot get a model yet.
    pub(crate) const fn no_models_hint(&self) -> &'static str {
        self.pick(
            "Put a .glb or .gltf model in the assets folder first.",
            "Primero pon un modelo .glb o .gltf en la carpeta de assets.",
        )
    }

    /// The section of a node's bone, the part the inspector adds and the
    /// node “Create” makes.
    pub(crate) const fn bone_2d(&self) -> &'static str {
        self.pick("2D bone", "Hueso 2D")
    }

    pub(crate) const fn bone_length(&self) -> &'static str {
        self.pick("Length", "Longitud")
    }

    /// The section of a node's animator, and the part the inspector adds.
    pub(crate) const fn animator_section(&self) -> &'static str {
        self.pick("Animator", "Animador")
    }

    /// Why a node cannot get an animator yet.
    pub(crate) const fn no_keyframes_hint(&self) -> &'static str {
        self.pick(
            "Create an .anim file first: Create > Keyframe animations, in the assets panel.",
            "Crea antes un archivo .anim: Crear > Animaciones de keyframes, en el panel de assets.",
        )
    }

    /// The `.anim` file of an animator, in the inspector.
    pub(crate) const fn keyframes(&self) -> &'static str {
        self.pick("Animations", "Animaciones")
    }

    pub(crate) const fn keyframes_hint(&self) -> &'static str {
        self.pick(
            "An .anim file: the animator plays its animations on its parent and the nodes under it.",
            "Un archivo .anim: el animador reproduce sus animaciones en su padre y los nodos bajo él.",
        )
    }

    /// The animation a scene's animator plays from the start.
    pub(crate) const fn starting_animation(&self) -> &'static str {
        self.pick("Starting animation", "Animación inicial")
    }

    /// The row of an animator's `.states` file.
    pub(crate) const fn animator_states(&self) -> &'static str {
        self.pick("States", "Estados")
    }

    pub(crate) const fn animator_states_hint(&self) -> &'static str {
        self.pick(
            "A .states file: a state machine that chooses which of the animations play, from values the scripts set.",
            "Un archivo .states: una máquina de estados que elige qué animaciones suenan, según valores que ponen los scripts.",
        )
    }

    pub(crate) const fn no_states(&self) -> &'static str {
        self.pick("None", "Ninguno")
    }
}
