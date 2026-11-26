//! Texts of the timeline, where the keyframe animations of an animator
//! are edited, and of the keys of the inspector, in each language.

use star_engine::star_scene::Interpolation;

use super::{Language, Texts};

impl Texts {
    /// The tab of the panel under the view that shows the timeline.
    pub(crate) const fn animation_tab(&self) -> &'static str {
        self.pick("Animation", "Animación")
    }

    pub(crate) const fn timeline_no_animator(&self) -> &'static str {
        self.pick(
            "Select an Animator node to edit its animations here.",
            "Selecciona un nodo con Animador para editar aquí sus animaciones.",
        )
    }

    pub(crate) const fn timeline_no_animations(&self) -> &'static str {
        self.pick(
            "This file has no animations yet: create one with “New animation”.",
            "Este archivo aún no tiene animaciones: crea una con «Nueva animación».",
        )
    }

    pub(crate) fn animations_unreadable(&self, path: &str, error: &str) -> String {
        match self.language {
            Language::English => format!("\"{path}\" cannot be read: {error}"),
            Language::Spanish => format!("No se puede leer «{path}»: {error}"),
        }
    }

    pub(crate) const fn new_animation(&self) -> &'static str {
        self.pick("New animation", "Nueva animación")
    }

    pub(crate) const fn rename_animation(&self) -> &'static str {
        self.pick("Rename", "Renombrar")
    }

    pub(crate) const fn duplicate_animation(&self) -> &'static str {
        self.pick("Duplicate", "Duplicar")
    }

    pub(crate) const fn delete_animation(&self) -> &'static str {
        self.pick("Delete", "Borrar")
    }

    /// Gives an animation the name typed.
    pub(crate) const fn apply_name(&self) -> &'static str {
        self.pick("Rename", "Renombrar")
    }

    pub(crate) const fn animation_name_taken(&self) -> &'static str {
        self.pick(
            "Another animation has that name, or it is blank.",
            "Otra animación se llama así, o el nombre está vacío.",
        )
    }

    pub(crate) const fn animation_length(&self) -> &'static str {
        self.pick("Length", "Duración")
    }

    pub(crate) const fn animation_length_hint(&self) -> &'static str {
        self.pick(
            "How long the animation lasts. It cannot end before its last key or call.",
            "Cuánto dura la animación. No puede acabar antes de su última clave o llamada.",
        )
    }

    pub(crate) const fn play_animation(&self) -> &'static str {
        self.pick("Play", "Reproducir")
    }

    pub(crate) const fn pause(&self) -> &'static str {
        self.pick("Pause", "Pausa")
    }

    pub(crate) const fn stop_animation(&self) -> &'static str {
        self.pick("Stop", "Parar")
    }

    pub(crate) const fn remove_track(&self) -> &'static str {
        self.pick(
            "Remove this track and its keys",
            "Quitar esta pista y sus claves",
        )
    }

    pub(crate) const fn track_node_missing(&self) -> &'static str {
        self.pick(
            "No node of the scene has this path from the animated node: the game will skip this track.",
            "Ningún nodo de la escena tiene esta ruta desde el nodo animado: el juego se saltará esta pista.",
        )
    }

    pub(crate) const fn track_hint(&self) -> &'static str {
        self.pick(
            "The node, by its path from the Animator's parent, and the property this track changes.",
            "El nodo, por su ruta desde el padre del Animador, y la propiedad que cambia esta pista.",
        )
    }

    pub(crate) const fn interpolation_name(&self, interpolation: Interpolation) -> &'static str {
        match interpolation {
            Interpolation::Step => self.pick("Step", "Escalón"),
            Interpolation::Linear => self.pick("Linear", "Lineal"),
            Interpolation::EaseIn => self.pick("Ease in", "Suave al entrar"),
            Interpolation::EaseOut => self.pick("Ease out", "Suave al salir"),
            Interpolation::EaseInOut => self.pick("Ease in-out", "Suave en ambos"),
        }
    }

    pub(crate) const fn interpolation_hint(&self) -> &'static str {
        self.pick(
            "How the value goes from one key to the next. Decimals and colors follow it; whole numbers, booleans and texts always jump.",
            "Cómo va el valor de una clave a la siguiente. Los decimales y los colores lo siguen; los enteros, los booleanos y los textos siempre saltan.",
        )
    }

    pub(crate) const fn calls(&self) -> &'static str {
        self.pick("Calls", "Llamadas")
    }

    pub(crate) const fn calls_hint(&self) -> &'static str {
        self.pick(
            "Methods called when the animation passes their time.",
            "Métodos que se llaman cuando la animación pasa por su instante.",
        )
    }

    pub(crate) const fn add_call(&self) -> &'static str {
        self.pick("Add call", "Añadir llamada")
    }

    pub(crate) const fn no_callable_methods(&self) -> &'static str {
        self.pick(
            "No node animated has a class with public methods without arguments (neither async nor parallel).",
            "Ningún nodo animado tiene una clase con métodos públicos sin argumentos (ni async ni parallel).",
        )
    }

    pub(crate) const fn timeline_hint(&self) -> &'static str {
        self.pick(
            "Click a key to select it, drag it to move it, double-click a lane to add one. Delete, Ctrl+C and Ctrl+V work on the keys selected.",
            "Haz clic en una clave para seleccionarla, arrástrala para moverla, doble clic en una pista para añadir una. Supr, Ctrl+C y Ctrl+V actúan sobre las claves seleccionadas.",
        )
    }

    pub(crate) fn marks_selected(&self, count: usize) -> String {
        match self.language {
            Language::English => format!("{count} keys and calls selected"),
            Language::Spanish => format!("{count} claves y llamadas seleccionadas"),
        }
    }

    pub(crate) const fn key_time(&self) -> &'static str {
        self.pick("Time", "Instante")
    }

    pub(crate) fn key_value(&self, property: &str) -> String {
        match self.language {
            Language::English => format!("Value of `{property}`"),
            Language::Spanish => format!("Valor de `{property}`"),
        }
    }

    /// The help of a key button of the inspector.
    pub(crate) fn add_key_hint(&self, seconds: f32) -> String {
        match self.language {
            Language::English => format!(
                "Put a key with this value at {seconds:.2} s. Filled: there is one there; outlined: it has a track."
            ),
            Language::Spanish => format!(
                "Pon una clave con este valor en {seconds:.2} s. Relleno: ya hay una ahí; contorno: tiene pista."
            ),
        }
    }

    pub(crate) const fn call_method(&self) -> &'static str {
        self.pick("Method", "Método")
    }
}
