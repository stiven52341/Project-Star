//! Texts of the states editor (an animator's state machine), in each
//! language.

use star_engine::star_scene::ValueKind;

use super::{Language, Texts};

impl Texts {
    pub(crate) const fn states_graph_hint(&self) -> &'static str {
        self.pick(
            "Right-click the graph to add states and transitions; drag states to move them. Select one to edit it in the inspector.",
            "Haz clic derecho en el grafo para añadir estados y transiciones; arrastra los estados para moverlos. Selecciona uno para editarlo en el inspector.",
        )
    }

    pub(crate) const fn linking_hint(&self) -> &'static str {
        self.pick(
            "Click the state the transition goes to (Esc gives up).",
            "Haz clic en el estado al que va la transición (Esc cancela).",
        )
    }

    pub(crate) const fn states_animations_hint(&self) -> &'static str {
        self.pick(
            "The .anim file whose animations the states play: the animator's, which names this machine too.",
            "El archivo .anim cuyas animaciones reproducen los estados: el del animador que también nombra esta máquina.",
        )
    }

    pub(crate) const fn any_state(&self) -> &'static str {
        self.pick("Any state", "Cualquier estado")
    }

    pub(crate) const fn new_state(&self) -> &'static str {
        self.pick("New state", "Nuevo estado")
    }

    pub(crate) const fn new_transition(&self) -> &'static str {
        self.pick("New transition from here", "Nueva transición desde aquí")
    }

    pub(crate) const fn new_transition_from_any(&self) -> &'static str {
        self.pick(
            "New transition from any state",
            "Nueva transición desde cualquier estado",
        )
    }

    pub(crate) const fn set_start_state(&self) -> &'static str {
        self.pick("Start here", "Empezar aquí")
    }

    pub(crate) const fn delete_state(&self) -> &'static str {
        self.pick("Delete state", "Borrar estado")
    }

    pub(crate) const fn delete_transition(&self) -> &'static str {
        self.pick("Delete transition", "Borrar transición")
    }

    pub(crate) const fn state_name(&self) -> &'static str {
        self.pick("Name", "Nombre")
    }

    pub(crate) const fn start_state(&self) -> &'static str {
        self.pick("Starts here", "Empieza aquí")
    }

    pub(crate) const fn state_plays(&self) -> &'static str {
        self.pick("Plays", "Reproduce")
    }

    pub(crate) const fn plays_animation(&self) -> &'static str {
        self.pick("An animation", "Una animación")
    }

    pub(crate) const fn plays_blend(&self) -> &'static str {
        self.pick("A blend", "Una mezcla")
    }

    pub(crate) const fn blend_needs_number(&self) -> &'static str {
        self.pick(
            "A blend needs a number value: add one below.",
            "Una mezcla necesita un valor numérico: añade uno abajo.",
        )
    }

    pub(crate) const fn blend_value(&self) -> &'static str {
        self.pick("By the value", "Según el valor")
    }

    pub(crate) const fn blend_points(&self) -> &'static str {
        self.pick(
            "Animations, and the value where each plays alone:",
            "Animaciones, y el valor en que cada una suena sola:",
        )
    }

    pub(crate) const fn blend_at(&self) -> &'static str {
        self.pick("at", "en")
    }

    pub(crate) const fn add_blend_point(&self) -> &'static str {
        self.pick("Add animation", "Añadir animación")
    }

    /// What a state's box says it plays when it blends `points`
    /// animations by `value`.
    pub(crate) fn blend_label(&self, value: &str, points: usize) -> String {
        match self.language {
            Language::English => format!("blend of {points} by {value}"),
            Language::Spanish => format!("mezcla de {points} según {value}"),
        }
    }

    pub(crate) const fn transition_from(&self) -> &'static str {
        self.pick("From", "Desde")
    }

    pub(crate) const fn transition_to(&self) -> &'static str {
        self.pick("To", "Hasta")
    }

    pub(crate) const fn transition_fade(&self) -> &'static str {
        self.pick("Fade", "Fundido")
    }

    pub(crate) const fn transition_fade_hint(&self) -> &'static str {
        self.pick(
            "Seconds the animations take to go from one state to the other; 0 cuts at once.",
            "Segundos que tardan las animaciones en pasar de un estado al otro; 0 corta de golpe.",
        )
    }

    pub(crate) const fn transition_at_end(&self) -> &'static str {
        self.pick("At the end", "Al terminar")
    }

    pub(crate) const fn transition_at_end_hint(&self) -> &'static str {
        self.pick(
            "Waits until the state has played its animation once.",
            "Espera a que el estado haya reproducido su animación una vez.",
        )
    }

    pub(crate) const fn conditions(&self) -> &'static str {
        self.pick("Conditions", "Condiciones")
    }

    pub(crate) const fn conditions_hint(&self) -> &'static str {
        self.pick(
            "All of them must hold to take the transition; without any, it is taken at once.",
            "Deben cumplirse todas para tomar la transición; sin ninguna, se toma enseguida.",
        )
    }

    pub(crate) const fn add_condition(&self) -> &'static str {
        self.pick("Add condition", "Añadir condición")
    }

    pub(crate) const fn condition_needs_value(&self) -> &'static str {
        self.pick(
            "Conditions test values: add one below.",
            "Las condiciones prueban valores: añade uno abajo.",
        )
    }

    pub(crate) const fn above(&self) -> &'static str {
        self.pick("above", "mayor que")
    }

    pub(crate) const fn below(&self) -> &'static str {
        self.pick("below", "menor que")
    }

    pub(crate) const fn is(&self) -> &'static str {
        self.pick("is", "es")
    }

    pub(crate) const fn triggered(&self) -> &'static str {
        self.pick("is triggered", "se activa")
    }

    pub(crate) const fn remove(&self) -> &'static str {
        self.pick("Remove", "Quitar")
    }

    pub(crate) const fn state_values(&self) -> &'static str {
        self.pick("Values", "Valores")
    }

    pub(crate) const fn state_values_hint(&self) -> &'static str {
        self.pick(
            "What scripts set (set_value, set_flag, trigger) and conditions and blends read.",
            "Lo que ponen los scripts (set_value, set_flag, trigger) y leen las condiciones y las mezclas.",
        )
    }

    pub(crate) const fn add_state_value(&self) -> &'static str {
        self.pick("Add value", "Añadir valor")
    }

    pub(crate) const fn value_kind(&self, kind: ValueKind) -> &'static str {
        match kind {
            ValueKind::Number => self.pick("number", "número"),
            ValueKind::Flag => self.pick("flag", "bandera"),
            ValueKind::Trigger => self.pick("trigger", "disparador"),
        }
    }

    pub(crate) fn states_unreadable(&self, reason: &str) -> String {
        match self.language {
            Language::English => format!("Could not open the state machine: {reason}"),
            Language::Spanish => format!("No se pudo abrir la máquina de estados: {reason}"),
        }
    }

    pub(crate) fn states_not_saved(&self, reason: &str) -> String {
        match self.language {
            Language::English => format!("Could not save the state machine: {reason}"),
            Language::Spanish => format!("No se pudo guardar la máquina de estados: {reason}"),
        }
    }
}
