//! Every text the editor shows, in every language it speaks: English (the
//! default) and Spanish. Each language is a [`Texts`] with the same fields,
//! so the compiler checks that none is missing a text; texts built from
//! values are methods of [`Texts`] (see `messages`, `scene_messages` and
//! `console_messages`).

mod asset_messages;
mod console_messages;
mod english;
mod messages;
mod scene_messages;
mod script_messages;
mod spanish;

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};
use star_blueprint::Blueprint;

use self::english::ENGLISH;
use self::spanish::SPANISH;

/// A language the editor can be shown in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "es")]
    Spanish,
}

impl Language {
    /// Every language, in the order the editor lists them.
    pub(crate) const ALL: [Self; 2] = [Self::English, Self::Spanish];

    /// The language's name in itself, so anyone finds their own.
    pub(crate) const fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
        }
    }

    /// Its texts.
    pub(crate) const fn texts(self) -> &'static Texts {
        match self {
            Self::English => &ENGLISH,
            Self::Spanish => &SPANISH,
        }
    }

    /// Its place in [`Self::ALL`], which lists them as they are declared.
    const fn index(self) -> usize {
        self as usize
    }
}

/// The language shown, as its place in [`Language::ALL`]. It is global
/// because the whole interface, drawn from many places, speaks one language
/// at a time; an atomic keeps it safe without locks.
static CURRENT_LANGUAGE: AtomicUsize = AtomicUsize::new(Language::English.index());

/// The language the editor is shown in.
pub(crate) fn current_language() -> Language {
    Language::ALL
        .get(CURRENT_LANGUAGE.load(Ordering::Relaxed))
        .copied()
        .unwrap_or_default()
}

/// Shows the editor in `language` from the next frame on.
pub(crate) fn set_language(language: Language) {
    CURRENT_LANGUAGE.store(language.index(), Ordering::Relaxed);
}

/// The texts of the language the editor is shown in.
pub(crate) fn current() -> &'static Texts {
    current_language().texts()
}

/// The texts of one language. Texts built from values are its methods.
pub(crate) struct Texts {
    language: Language,
    // Launcher.
    pub(crate) project_manager: &'static str,
    pub(crate) recent_projects: &'static str,
    pub(crate) no_recent_projects: &'static str,
    pub(crate) open: &'static str,
    pub(crate) remove_from_list: &'static str,
    pub(crate) remove_from_list_hint: &'static str,
    pub(crate) project_unavailable: &'static str,
    pub(crate) open_other_project: &'static str,
    pub(crate) choose_project_folder: &'static str,
    // New project form.
    pub(crate) new_project: &'static str,
    pub(crate) project_name: &'static str,
    pub(crate) project_name_hint: &'static str,
    pub(crate) location: &'static str,
    pub(crate) browse: &'static str,
    pub(crate) choose_location: &'static str,
    pub(crate) dimension: &'static str,
    pub(crate) dimension_2d_description: &'static str,
    pub(crate) dimension_3d_description: &'static str,
    pub(crate) platforms: &'static str,
    pub(crate) desktop: &'static str,
    pub(crate) mobile: &'static str,
    pub(crate) create_project: &'static str,
    pub(crate) will_be_created_in: &'static str,
    pub(crate) location_required: &'static str,
    // Editor window.
    pub(crate) menu_file: &'static str,
    pub(crate) close_project: &'static str,
    pub(crate) menu_edit: &'static str,
    pub(crate) undo: &'static str,
    pub(crate) redo: &'static str,
    pub(crate) no_node_class: &'static str,
    pub(crate) choose_class: &'static str,
    pub(crate) quit: &'static str,
    pub(crate) menu_help: &'static str,
    pub(crate) about: &'static str,
    pub(crate) about_description: &'static str,
    pub(crate) license_notice: &'static str,
    pub(crate) game_exception_notice: &'static str,
    pub(crate) no_warranty: &'static str,
    pub(crate) source_code: &'static str,
    pub(crate) hierarchy: &'static str,
    pub(crate) no_scene_open: &'static str,
    pub(crate) node_section: &'static str,
    pub(crate) no_class: &'static str,
    /// The button that gives an `export` field the value of its class again.
    pub(crate) reset: &'static str,
    pub(crate) reset_field_hint: &'static str,
    /// The box that makes a blueprint variable an `export` field.
    pub(crate) export: &'static str,
    pub(crate) export_hint: &'static str,
    /// The rows of an `export` blueprint variable, one per decorator.
    pub(crate) header: &'static str,
    pub(crate) header_hint: &'static str,
    pub(crate) value_range: &'static str,
    pub(crate) range_hint: &'static str,
    pub(crate) range_step: &'static str,
    pub(crate) tooltip: &'static str,
    pub(crate) tooltip_hint: &'static str,
    pub(crate) multiline: &'static str,
    pub(crate) multiline_hint: &'static str,
    /// A value of the scene for a field its class does not export.
    pub(crate) not_an_export_field: &'static str,
    pub(crate) no_image: &'static str,
    pub(crate) add_component: &'static str,
    pub(crate) no_images_hint: &'static str,
    pub(crate) empty_node: &'static str,
    pub(crate) directional_light: &'static str,
    pub(crate) point_light: &'static str,
    pub(crate) camera_2d: &'static str,
    pub(crate) physics: &'static str,
    pub(crate) static_body: &'static str,
    pub(crate) rigid_body: &'static str,
    pub(crate) kinematic_body: &'static str,
    pub(crate) area: &'static str,
    pub(crate) camera_3d: &'static str,
    pub(crate) view_height: &'static str,
    pub(crate) field_of_view: &'static str,
    pub(crate) current_camera: &'static str,
    pub(crate) current_camera_hint: &'static str,
    pub(crate) duplicate: &'static str,
    pub(crate) root_cannot_be_duplicated: &'static str,
    pub(crate) root_cannot_be_deleted: &'static str,
    pub(crate) new_node_name: &'static str,
    pub(crate) search_nodes: &'static str,
    pub(crate) no_nodes_match: &'static str,
    pub(crate) circle: &'static str,
    pub(crate) rectangle: &'static str,
    pub(crate) capsule: &'static str,
    pub(crate) sphere: &'static str,
    pub(crate) box_shape: &'static str,
    pub(crate) cube: &'static str,
    pub(crate) plane: &'static str,
    pub(crate) radius: &'static str,
    pub(crate) height: &'static str,
    pub(crate) segments: &'static str,
    pub(crate) rings: &'static str,
    pub(crate) unsaved_scene_question: &'static str,
    pub(crate) transform_section: &'static str,
    pub(crate) transform_3d_section: &'static str,
    pub(crate) position: &'static str,
    pub(crate) rotation: &'static str,
    pub(crate) scale: &'static str,
    pub(crate) sprite_section: &'static str,
    pub(crate) image: &'static str,
    pub(crate) size: &'static str,
    pub(crate) color: &'static str,
    pub(crate) layer: &'static str,
    pub(crate) filter: &'static str,
    pub(crate) anchor: &'static str,
    pub(crate) region: &'static str,
    pub(crate) body_section: &'static str,
    pub(crate) body_3d_section: &'static str,
    pub(crate) body_kind: &'static str,
    pub(crate) shape: &'static str,
    pub(crate) gravity_scale: &'static str,
    pub(crate) friction: &'static str,
    pub(crate) bounciness: &'static str,
    pub(crate) density: &'static str,
    pub(crate) sensor: &'static str,
    pub(crate) fixed_rotation: &'static str,
    pub(crate) mesh_section: &'static str,
    pub(crate) metallic: &'static str,
    pub(crate) roughness: &'static str,
    pub(crate) light_section: &'static str,
    pub(crate) intensity: &'static str,
    pub(crate) range: &'static str,
    pub(crate) inspector: &'static str,
    pub(crate) project: &'static str,
    pub(crate) engine: &'static str,
    pub(crate) folder: &'static str,
    pub(crate) assets: &'static str,
    pub(crate) refresh: &'static str,
    pub(crate) empty_assets: &'static str,
    pub(crate) menu_project: &'static str,
    pub(crate) compile: &'static str,
    pub(crate) compile_hint: &'static str,
    pub(crate) compiling: &'static str,
    pub(crate) problems: &'static str,
    pub(crate) not_compiled: &'static str,
    pub(crate) check_failed: &'static str,
    pub(crate) check_lost: &'static str,
    // Playing.
    pub(crate) play: &'static str,
    pub(crate) play_hint: &'static str,
    pub(crate) stop: &'static str,
    pub(crate) stop_hint: &'static str,
    pub(crate) output: &'static str,
    pub(crate) console: &'static str,
    pub(crate) clear: &'static str,
    pub(crate) show_levels: &'static str,
    pub(crate) console_empty: &'static str,
    pub(crate) output_empty: &'static str,
    pub(crate) building_game: &'static str,
    pub(crate) game_running: &'static str,
    pub(crate) game_stopped: &'static str,
    pub(crate) game_build_failed: &'static str,
    pub(crate) play_blocked: &'static str,
    pub(crate) nothing_to_play: &'static str,
    pub(crate) no_game_to_play: &'static str,
    pub(crate) scene_tab: &'static str,
    pub(crate) close_tab: &'static str,
    pub(crate) open_file_hint: &'static str,
    // Saving.
    pub(crate) save: &'static str,
    pub(crate) unsaved_changes: &'static str,
    pub(crate) unsaved_tab_question: &'static str,
    pub(crate) unsaved_files_question: &'static str,
    pub(crate) discard_changes: &'static str,
    pub(crate) cancel: &'static str,
    // Blueprint editor.
    pub(crate) add_node: &'static str,
    pub(crate) menu_value: &'static str,
    pub(crate) menu_get_variable: &'static str,
    pub(crate) menu_set_variable: &'static str,
    pub(crate) menu_call: &'static str,
    /// The ready-made functions of the blueprint menu, and their groups.
    pub(crate) menu_functions: &'static str,
    /// The submenu of “Create” with the ready-made blueprints.
    pub(crate) blueprint_from_template: &'static str,
    pub(crate) search_functions: &'static str,
    pub(crate) no_function_matches: &'static str,
    pub(crate) transform: &'static str,
    pub(crate) math: &'static str,
    pub(crate) menu_operator: &'static str,
    pub(crate) node_branch: &'static str,
    pub(crate) node_return: &'static str,
    pub(crate) other_name: &'static str,
    pub(crate) call_own_method: &'static str,
    pub(crate) call_method_of_value: &'static str,
    pub(crate) call_class_function: &'static str,
    pub(crate) delete_node: &'static str,
    pub(crate) delete_nodes: &'static str,
    pub(crate) entry_cannot_be_deleted: &'static str,
    pub(crate) disconnect_pin: &'static str,
    pub(crate) bool_kind: &'static str,
    pub(crate) int_kind: &'static str,
    pub(crate) float_kind: &'static str,
    pub(crate) string_kind: &'static str,
    pub(crate) literal_value: &'static str,
    pub(crate) variable: &'static str,
    pub(crate) call_target: &'static str,
    /// Whose variable a variable node reads or sets.
    pub(crate) variable_owner: &'static str,
    /// The box that makes a variable node use the field of another object.
    pub(crate) of_another_object: &'static str,
    pub(crate) of_another_object_hint: &'static str,
    pub(crate) class: &'static str,
    pub(crate) method: &'static str,
    pub(crate) arguments: &'static str,
    pub(crate) add_argument: &'static str,
    pub(crate) remove: &'static str,
    pub(crate) returns_value: &'static str,
    pub(crate) pure_call: &'static str,
    pub(crate) pure_call_hint: &'static str,
    pub(crate) entry_hint: &'static str,
    pub(crate) graphs_menu: &'static str,
    pub(crate) new_graph: &'static str,
    pub(crate) delete_graph: &'static str,
    pub(crate) variables: &'static str,
    pub(crate) name: &'static str,
    pub(crate) extends: &'static str,
    pub(crate) base_arguments: &'static str,
    pub(crate) base_arguments_hint: &'static str,
    pub(crate) type_label: &'static str,
    pub(crate) variable_type_hint: &'static str,
    pub(crate) default_value: &'static str,
    pub(crate) private: &'static str,
    pub(crate) add_variable: &'static str,
    pub(crate) remove_variable: &'static str,
    pub(crate) parameters: &'static str,
    pub(crate) add_parameter: &'static str,
    pub(crate) returns: &'static str,
    pub(crate) overrides: &'static str,
    pub(crate) override_hint: &'static str,
    pub(crate) rename: &'static str,
    pub(crate) delete: &'static str,
    pub(crate) copy_path: &'static str,
    pub(crate) show_in_file_explorer: &'static str,
    pub(crate) icon_size: &'static str,
    pub(crate) rename_to: &'static str,
    pub(crate) create: &'static str,
    pub(crate) show_in_blueprint: &'static str,
    pub(crate) no_graphs: &'static str,
    pub(crate) canvas_controls: &'static str,
    pub(crate) no_gpu_for_viewport: &'static str,
    // Preferences.
    pub(crate) language_menu: &'static str,
}

/// The name of a tab with unsaved changes.
pub(crate) fn unsaved_title(name: &str) -> String {
    format!("{name} •")
}

/// `Player : Sprite`, or only the class if it extends nothing.
pub(crate) fn class_heading(blueprint: &Blueprint) -> String {
    match &blueprint.extends {
        Some(base) => format!("{} : {base}", blueprint.class),
        None => blueprint.class.clone(),
    }
}

pub(crate) fn folder_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_the_default_language() {
        assert_eq!(Language::default(), Language::English);
        assert_eq!(Language::ALL[0], Language::default());
    }

    /// Regression: `●` and arrows were written with the default fonts,
    /// which do not have them, so they were drawn as boxes and logged a
    /// warning on every frame.
    #[test]
    fn every_character_the_editor_writes_can_be_drawn() {
        let context = eframe::egui::Context::default();
        let mut output = context.run_ui(eframe::egui::RawInput::default(), |_| {});
        // Nothing is drawn, so the font textures are never uploaded.
        output.textures_delta.clear();
        let font = eframe::egui::FontId::proportional(eframe::egui::FontId::default().size);
        let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut missing = Vec::new();
        for path in
            crate::file_tree::file_paths(&crate::file_tree::scan_directory(&sources).unwrap())
        {
            let text = std::fs::read_to_string(sources.join(&path)).unwrap();
            let code = text
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"));
            for character in code
                .flat_map(str::chars)
                .filter(|character| !character.is_ascii())
            {
                if !context.fonts_mut(|fonts| fonts.has_glyph(&font, character)) {
                    missing.push(format!("{path}: {character:?}"));
                }
            }
        }
        assert_eq!(missing, Vec::<String>::new());
    }

    #[test]
    fn every_language_has_its_own_texts() {
        for language in Language::ALL {
            assert_eq!(language.texts().language, language);
            assert_eq!(Language::ALL.get(language.index()), Some(&language));
        }
        assert_eq!(Language::English.texts().save, "Save");
        assert_eq!(Language::Spanish.texts().save, "Guardar");
    }
}
