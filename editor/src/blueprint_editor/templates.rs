//! Ready-made blueprints a project can start from: moving with the
//! player's input, turning, flying straight, disappearing after a while.
//!
//! Each one is copied into the project under the name the user gives it,
//! so it is the project's own to change, as a new blueprint is.

use star_blueprint::{Blueprint, Literal};

use crate::project::ProjectDimension;

/// The input actions the movement templates read, with the keys, arrows
/// and stick directions they are bound to when the project has none.
const MOVEMENT_ACTIONS: [(&str, &[&str]); 4] = [
    ("left", &["A", "ArrowLeft", "GamepadLeftStickLeft"]),
    ("right", &["D", "ArrowRight", "GamepadLeftStickRight"]),
    ("up", &["W", "ArrowUp", "GamepadLeftStickUp"]),
    ("down", &["S", "ArrowDown", "GamepadLeftStickDown"]),
];

/// A ready-made blueprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlueprintTemplate {
    Movement2D,
    Movement3D,
    Spinner2D,
    Spinner3D,
    Mover2D,
    Mover3D,
    Lifetime,
}

impl BlueprintTemplate {
    /// Every template, in the order the menu lists them.
    pub(crate) const ALL: [Self; 7] = [
        Self::Movement2D,
        Self::Movement3D,
        Self::Spinner2D,
        Self::Spinner3D,
        Self::Mover2D,
        Self::Mover3D,
        Self::Lifetime,
    ];

    /// The templates that fit a project of `dimension`.
    pub(crate) fn of(dimension: ProjectDimension) -> impl Iterator<Item = Self> {
        Self::ALL
            .into_iter()
            .filter(move |template| template.dimension().is_none_or(|own| own == dimension))
    }

    /// The name of its class, which a copy keeps until the user changes it.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Movement2D => "Movement2D",
            Self::Movement3D => "Movement3D",
            Self::Spinner2D => "Spinner2D",
            Self::Spinner3D => "Spinner3D",
            Self::Mover2D => "Mover2D",
            Self::Mover3D => "Mover3D",
            Self::Lifetime => "Lifetime",
        }
    }

    /// The kind of project it is made for; `None` if it fits both.
    const fn dimension(self) -> Option<ProjectDimension> {
        match self {
            Self::Movement2D | Self::Spinner2D | Self::Mover2D => Some(ProjectDimension::TwoD),
            Self::Movement3D | Self::Spinner3D | Self::Mover3D => Some(ProjectDimension::ThreeD),
            Self::Lifetime => None,
        }
    }

    /// The input actions it reads, with the bindings they get if the
    /// project does not have them yet.
    pub(crate) const fn actions(self) -> &'static [(&'static str, &'static [&'static str])] {
        match self {
            Self::Movement2D | Self::Movement3D => &MOVEMENT_ACTIONS,
            Self::Spinner2D | Self::Spinner3D | Self::Mover2D | Self::Mover3D | Self::Lifetime => {
                &[]
            }
        }
    }

    const fn source(self) -> &'static str {
        match self {
            Self::Movement2D => include_str!("../../templates/blueprints/Movement2D.blueprint"),
            Self::Movement3D => include_str!("../../templates/blueprints/Movement3D.blueprint"),
            Self::Spinner2D => include_str!("../../templates/blueprints/Spinner2D.blueprint"),
            Self::Spinner3D => include_str!("../../templates/blueprints/Spinner3D.blueprint"),
            Self::Mover2D => include_str!("../../templates/blueprints/Mover2D.blueprint"),
            Self::Mover3D => include_str!("../../templates/blueprints/Mover3D.blueprint"),
            Self::Lifetime => include_str!("../../templates/blueprints/Lifetime.blueprint"),
        }
    }

    /// The text of a copy whose class is called `class`; `None` if the
    /// template cannot be read or written (which its tests rule out).
    pub(crate) fn text(self, class: &str) -> Option<String> {
        let mut blueprint = Blueprint::from_text(self.source())
            .inspect_err(|error| log::warn!("{error}"))
            .ok()?;
        let template_name = Literal::String(blueprint.class.clone());
        // A node's first argument is its name, the class's own by default.
        for argument in &mut blueprint.base_arguments {
            if *argument == template_name {
                *argument = Literal::String(class.to_owned());
            }
        }
        class.clone_into(&mut blueprint.class);
        blueprint
            .to_text()
            .inspect_err(|error| log::warn!("{error}"))
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use star_script::{GameSettings, InputAction};

    use super::*;

    /// The settings of a game that has every action the templates read.
    fn settings() -> GameSettings {
        let actions = BlueprintTemplate::ALL
            .iter()
            .flat_map(|template| template.actions())
            .map(|(name, bindings)| InputAction {
                name: (*name).to_owned(),
                bindings: bindings.iter().map(|&binding| binding.to_owned()).collect(),
            })
            .collect();
        GameSettings {
            actions,
            main_scene: None,
            ..Default::default()
        }
    }

    #[test]
    fn every_template_is_valid_and_compiles() {
        for template in BlueprintTemplate::ALL {
            let text = template.text("Copy").unwrap();
            let blueprint = Blueprint::from_text(&text).unwrap();

            assert_eq!(star_blueprint::validate(&blueprint), Ok(()), "{template:?}");
            let translation = star_blueprint::translate(&blueprint).unwrap();
            if let Err(errors) = star_script::check_source_with(translation.source(), &settings()) {
                panic!("{template:?}\n{}\n{errors:?}", translation.source());
            }
        }
    }

    /// The conformance programs, which compiled and interpreted runs
    /// compare (decision 73).
    const PROGRAMS: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../crates/star_script/tests/programs"
    );

    /// Each template has a conformance program that starts with its current
    /// translation, so both ways of running StarScript run the templates as
    /// they are, and declares the actions it reads.
    #[test]
    fn every_template_has_its_conformance_program() {
        let mut failures = Vec::new();
        for template in BlueprintTemplate::ALL {
            let blueprint = Blueprint::from_text(template.source()).unwrap();
            let translation = star_blueprint::translate(&blueprint).unwrap();
            let program = format!("{PROGRAMS}/blueprint_{}", template.name().to_lowercase());
            let source = std::fs::read_to_string(format!("{program}.star")).unwrap_or_default();
            if !source.starts_with(translation.source()) {
                failures.push(format!(
                    "{program}.star must start with the translation of {template:?}:\n{}",
                    translation.source()
                ));
            }
            let declared =
                std::fs::read_to_string(format!("{program}.actions")).unwrap_or_default();
            let declared: Vec<&str> = declared.lines().collect();
            let read: Vec<&str> = template.actions().iter().map(|(name, _)| *name).collect();
            if declared != read {
                failures.push(format!("{program}.actions must list {read:?}"));
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
    }

    #[test]
    fn copies_take_the_name_they_are_given() {
        let blueprint =
            Blueprint::from_text(&BlueprintTemplate::Lifetime.text("Fade").unwrap()).unwrap();

        assert_eq!(blueprint.class, "Fade");
        assert_eq!(
            blueprint.base_arguments,
            [Literal::String("Fade".to_owned())]
        );
    }

    #[test]
    fn each_kind_of_project_gets_its_templates() {
        let two_d: Vec<_> = BlueprintTemplate::of(ProjectDimension::TwoD).collect();

        assert!(two_d.contains(&BlueprintTemplate::Movement2D));
        assert!(two_d.contains(&BlueprintTemplate::Lifetime));
        assert!(!two_d.contains(&BlueprintTemplate::Spinner3D));
    }

    #[test]
    fn the_movement_reads_its_axes() {
        let text = BlueprintTemplate::Movement2D.text("Player").unwrap();
        let blueprint = Blueprint::from_text(&text).unwrap();
        let code = star_blueprint::translate(&blueprint).unwrap();

        assert!(
            code.source()
                .contains("x = x + (Input.axis(\"left\", \"right\") * (speed * delta))"),
            "{}",
            code.source()
        );
    }
}
