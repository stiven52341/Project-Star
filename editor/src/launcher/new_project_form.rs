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
        ui.heading(texts::current().new_project);
        ui.add_space(SECTION_SPACING);
        egui::Grid::new("new_project_form")
            .num_columns(2)
            .spacing(FORM_SPACING)
            .show(ui, |ui| self.show_fields(ui));
        ui.add_space(SECTION_SPACING);
        self.show_submit(ui)
    }

    fn show_fields(&mut self, ui: &mut egui::Ui) {
        ui.label(texts::current().project_name);
        ui.add(
            egui::TextEdit::singleline(&mut self.name)
                .hint_text(texts::current().project_name_hint)
                .desired_width(TEXT_FIELD_WIDTH),
        );
        ui.end_row();

        ui.label(texts::current().location);
        self.show_location(ui);
        ui.end_row();

        ui.label(texts::current().dimension);
        self.show_dimension_choice(ui);
        ui.end_row();

        ui.label(texts::current().platforms);
        self.show_platform_checks(ui);
        ui.end_row();
    }

    fn show_location(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.location).desired_width(TEXT_FIELD_WIDTH));
            if ui.button(texts::current().browse).clicked()
                && let Some(folder) = pick_folder(
                    texts::current().choose_location,
                    Path::new(self.location.trim()),
                )
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
            self.show_platform_group(ui, texts::current().desktop, TargetPlatform::is_desktop);
            self.show_platform_group(ui, texts::current().mobile, TargetPlatform::is_mobile);
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
                    texts::current().will_be_created_in,
                    directory.display()
                ));
            }
            Err(problem) => {
                ui.colored_label(ERROR_COLOR, problem);
            }
        }
        ui.add_space(SECTION_SPACING);

        let button =
            egui::Button::new(texts::current().create_project).min_size(CREATE_BUTTON_SIZE.into());
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
            .map_err(|error| texts::current().settings_error(error))?;
        let location = self.location.trim();
        if location.is_empty() {
            return Err(texts::current().location_required.to_owned());
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
                self.creation_error = Some(texts::current().creation_error(&error));
                None
            }
        }
    }
}

fn dimension_description(dimension: ProjectDimension) -> &'static str {
    match dimension {
        ProjectDimension::TwoD => texts::current().dimension_2d_description,
        ProjectDimension::ThreeD => texts::current().dimension_3d_description,
    }
}
