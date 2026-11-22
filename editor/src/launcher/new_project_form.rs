//! Form that creates a new project.

use std::path::{Path, PathBuf};

use eframe::egui;
use star_engine::prelude::TargetPlatform;

use crate::folder_dialog::pick_folder;
use crate::project::{
    ProjectDimension, ProjectSettings, create_project, default_projects_location, project_directory,
};
use crate::style::{ERROR_COLOR, SECTION_SPACING};
use crate::texts;

const FORM_SPACING: [f32; 2] = [16.0, 14.0];
const TEXT_FIELD_WIDTH: f32 = 300.0;
const CREATE_BUTTON_SIZE: [f32; 2] = [160.0, 32.0];

/// Name, location, 2D/3D choice and target platforms of a new project.
pub(crate) struct NewProjectForm {
    name: String,
    location: String,
    dimension: ProjectDimension,
    /// One flag per entry of [`TargetPlatform::SUPPORTED`].
    selected_platforms: [bool; TargetPlatform::SUPPORTED.len()],
    creation_error: Option<String>,
}

impl NewProjectForm {
    /// Empty form targeting the platform the editor runs on.
    pub(crate) fn new() -> Self {
        let current = TargetPlatform::current();
        Self {
            name: String::new(),
            location: default_projects_location().display().to_string(),
            dimension: ProjectDimension::default(),
            selected_platforms: TargetPlatform::SUPPORTED.map(|platform| platform == current),
            creation_error: None,
        }
    }

    /// Draws the form. Returns the folder of the project created this frame.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        ui.heading(texts::NEW_PROJECT);
        ui.add_space(SECTION_SPACING);
        egui::Grid::new("new_project_form")
            .num_columns(2)
            .spacing(FORM_SPACING)
            .show(ui, |ui| self.show_fields(ui));
        ui.add_space(SECTION_SPACING);
        self.show_submit(ui)
    }

    fn show_fields(&mut self, ui: &mut egui::Ui) {
        ui.label(texts::PROJECT_NAME);
        ui.add(
            egui::TextEdit::singleline(&mut self.name)
                .hint_text(texts::PROJECT_NAME_HINT)
                .desired_width(TEXT_FIELD_WIDTH),
        );
        ui.end_row();

        ui.label(texts::LOCATION);
        self.show_location(ui);
        ui.end_row();

        ui.label(texts::DIMENSION);
        self.show_dimension_choice(ui);
        ui.end_row();

        ui.label(texts::PLATFORMS);
        self.show_platform_checks(ui);
        ui.end_row();
    }

    fn show_location(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.location).desired_width(TEXT_FIELD_WIDTH));
            if ui.button(texts::BROWSE).clicked()
                && let Some(folder) =
                    pick_folder(texts::CHOOSE_LOCATION, Path::new(self.location.trim()))
            {
                self.location = folder.display().to_string();
            }
        });
    }

    fn show_dimension_choice(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            for dimension in ProjectDimension::ALL {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.dimension, dimension, dimension.label());
                    ui.weak(dimension_description(dimension));
                });
            }
        });
    }

    fn show_platform_checks(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            self.show_platform_group(ui, texts::DESKTOP, TargetPlatform::is_desktop);
            self.show_platform_group(ui, texts::MOBILE, TargetPlatform::is_mobile);
        });
    }

    fn show_platform_group(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        belongs_to_group: fn(TargetPlatform) -> bool,
    ) {
        ui.horizontal(|ui| {
            ui.weak(title);
            for (platform, selected) in TargetPlatform::SUPPORTED
                .into_iter()
                .zip(&mut self.selected_platforms)
            {
                if belongs_to_group(platform) {
                    ui.checkbox(selected, platform.name());
                }
            }
        });
    }

    fn show_submit(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let input = self.validated_input();
        match &input {
            Ok((settings, location)) => {
                let directory = project_directory(settings, location);
                ui.label(format!(
                    "{} {}",
                    texts::WILL_BE_CREATED_IN,
                    directory.display()
                ));
            }
            Err(problem) => {
                ui.colored_label(ERROR_COLOR, problem);
            }
        }
        ui.add_space(SECTION_SPACING);

        let button = egui::Button::new(texts::CREATE_PROJECT).min_size(CREATE_BUTTON_SIZE.into());
        let clicked = ui.add_enabled(input.is_ok(), button).clicked();
        let created = match (clicked, input) {
            (true, Ok((settings, location))) => self.create(&settings, &location),
            _ => None,
        };
        if let Some(error) = &self.creation_error {
            ui.colored_label(ERROR_COLOR, error);
        }
        created
    }

    fn selected_platforms(&self) -> Vec<TargetPlatform> {
        TargetPlatform::SUPPORTED
            .into_iter()
            .zip(self.selected_platforms)
            .filter_map(|(platform, selected)| selected.then_some(platform))
            .collect()
    }

    /// The validated project and its location, or the text explaining what is missing.
    fn validated_input(&self) -> Result<(ProjectSettings, PathBuf), String> {
        let settings = ProjectSettings::new(&self.name, self.dimension, self.selected_platforms())
            .map_err(texts::settings_error)?;
        let location = self.location.trim();
        if location.is_empty() {
            return Err(texts::LOCATION_REQUIRED.to_owned());
        }
        Ok((settings, PathBuf::from(location)))
    }

    fn create(&mut self, settings: &ProjectSettings, location: &Path) -> Option<PathBuf> {
        match create_project(settings, location) {
            Ok(directory) => {
                log::info!("Project created in {}", directory.display());
                self.creation_error = None;
                Some(directory)
            }
            Err(error) => {
                log::error!("{error}");
                self.creation_error = Some(texts::creation_error(&error));
                None
            }
        }
    }
}

fn dimension_description(dimension: ProjectDimension) -> &'static str {
    match dimension {
        ProjectDimension::TwoD => texts::DIMENSION_2D_DESCRIPTION,
        ProjectDimension::ThreeD => texts::DIMENSION_3D_DESCRIPTION,
    }
}
