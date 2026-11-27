//! Main editor window: menu, scene hierarchy, viewport, inspector and assets.

mod asset_actions;
mod asset_files;
mod asset_icons;
mod assets_panel;
mod camera_preview;
mod class_rename;
mod console_panel;
mod documents;
mod editor_camera;
mod grid;
mod inspector_panel;
mod live_check;
mod menu_bar;
mod output_panel;
mod problems_panel;
mod project_classes;
mod thumbnails;
mod unsaved_dialog;
mod viewport;
mod viewport_assets;
mod viewport_tools;

use eframe::{egui, egui_wgpu};
use star_engine::prelude::{ENGINE_NAME, ENGINE_VERSION};
use star_script::{ShowFps, Span};

pub(crate) use self::asset_files::{AssetFileError, NewAsset};
use self::assets_panel::{AssetEvent, AssetsPanel, OpenedAsset};
pub(crate) use self::assets_panel::{DEFAULT_ICON_SIZE, ICON_SIZES};
use self::class_rename::ClassRename;
use self::console_panel::ConsolePanel;
use self::documents::{ActiveDocument, Documents};
use self::inspector_panel::show_inspector;
use self::live_check::LiveCheck;
use self::menu_bar::{
    MenuAction, MenuState, REDO_OTHER_SHORTCUT, REDO_SHORTCUT, SAVE_SHORTCUT, UNDO_SHORTCUT,
    UndoSteps, show_menu_bar,
};
use self::output_panel::OutputPanel;
use self::problems_panel::{CheckEnd, ProblemsPanel};
use self::unsaved_dialog::{UnsavedChoice, show_unsaved_dialog};
use self::viewport::SceneViewport;
use crate::editor_audio::EditorAudio;
use crate::editor_log::LogHistory;
use crate::game_run::{EngineWarmUp, PlayMode, PlayRequest};
use crate::last_scenes::LastSceneStore;
use crate::project::{
    ASSETS_DIRECTORY, BLUEPRINT_EXTENSION, Project, SCRIPT_EXTENSION, read_show_fps, set_show_fps,
};
use crate::project_check::Problem;
use crate::scene_editor::{AssetFiles, SceneDocument};
use crate::style::ERROR_COLOR;
use crate::texts::{self, Language};

const SIDE_PANEL_WIDTH: f32 = 260.0;
const SIDE_PANEL_MIN_WIDTH: f32 = 180.0;
const BOTTOM_PANEL_HEIGHT: f32 = 200.0;
const BOTTOM_PANEL_MIN_HEIGHT: f32 = 100.0;

/// What the editor window asks the application to do.
pub(crate) enum EditorAction {
    CloseProject,
    /// Close the window: the user chose to lose unsaved changes.
    Quit,
    /// Show the editor in another language.
    ChangeLanguage(Language),
    /// Remember this size of the icons of “Assets”, in percent.
    ChangeIconSize(u16),
}

/// Something that closes blueprints, waiting for the user to say what to do
/// with their unsaved changes.
#[derive(Clone)]
enum Leaving {
    CloseTab(usize),
    /// Open the scene at this path of the assets folder in place of the
    /// one open.
    OpenScene(String),
    CloseProject,
    Quit,
}

/// How “Play” or “Play compiled” plays the project once it is checked.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PlayWay {
    /// With the player, interpreted.
    Interpreted,
    /// Built with the compiler and Cargo.
    Compiled,
}

/// What the panel below the view shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    Assets,
    Animation,
    Problems,
    Output,
    Console,
}

/// Editor for one open project.
pub(crate) struct EditorWindow {
    project: Project,
    assets: AssetsPanel,
    problems: ProblemsPanel,
    output: OutputPanel,
    /// The engine built in the background, so the first “Play” is quick.
    warm_up: Option<EngineWarmUp>,
    console: ConsolePanel,
    /// “Play” and “Play compiled” wait for their check: the game is played
    /// this way if it finds nothing.
    play_after_check: Option<PlayWay>,
    bottom_tab: BottomTab,
    documents: Documents,
    /// The scene to open again the next time the project is opened.
    last_scene: LastSceneStore,
    /// Checks the open scripts as they are written.
    live_check: LiveCheck,
    /// What is waiting for the answer about unsaved changes.
    leaving: Option<Leaving>,
    /// Whether the user already chose to quit without saving.
    quit_confirmed: bool,
    /// The engine view, or why it could not be created.
    viewport: Result<SceneViewport, String>,
    /// The editor's sound output, to hear the project's sounds.
    audio: EditorAudio,
    about_open: bool,
    /// When the game shows its frames per second, as `project.toml` says.
    show_fps: ShowFps,
    /// A size of the icons chosen this frame, to hand to the preferences.
    icon_size_chosen: Option<u16>,
}

impl EditorWindow {
    /// Opens `project`, rendering its scene on the UI's GPU (`render_state`),
    /// showing the messages of `log_history` in its console and the icons
    /// of “Assets” at `icon_size` percent.
    pub(crate) fn new(
        project: Project,
        context: &egui::Context,
        render_state: Option<&egui_wgpu::RenderState>,
        log_history: LogHistory,
        icon_size: u16,
    ) -> Self {
        let dimension = project.settings().dimension();
        let assets = AssetsPanel::new(project.directory(), dimension, icon_size);
        let viewport = render_state
            .ok_or_else(|| texts::current().no_gpu_for_viewport.to_owned())
            .and_then(|state| {
                SceneViewport::new(state, dimension, assets.directory())
                    .map_err(|error| error.to_string())
            });
        if let Err(error) = &viewport {
            log::error!("Scene viewport unavailable: {error}");
        }
        let last_scene = LastSceneStore::new(project.directory());
        // The inspector needs the classes of the scripts from the start,
        // for the `export` fields of the nodes.
        let mut live_check = LiveCheck::new();
        live_check.request();
        let show_fps = read_show_fps(project.directory());
        let mut documents = Documents::new();
        if let Some(scene) = last_scene.scene(assets.directory()) {
            documents.open_scene(assets.directory(), scene, dimension);
        }
        Self {
            project,
            assets,
            problems: ProblemsPanel::new(),
            output: OutputPanel::new(),
            warm_up: start_warm_up(context),
            console: ConsolePanel::new(log_history),
            play_after_check: None,
            bottom_tab: BottomTab::Assets,
            documents,
            last_scene,
            live_check,
            leaving: None,
            quit_confirmed: false,
            viewport,
            audio: EditorAudio::default(),
            about_open: false,
            show_fps,
            icon_size_chosen: None,
        }
    }

    /// Title for the native window while this project is open.
    pub(crate) fn title(&self) -> String {
        format!("{} — {ENGINE_NAME}", self.project.settings().name())
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<EditorAction> {
        self.follow_check(ui.ctx());
        let documents = &self.documents;
        self.live_check
            .update(ui.ctx(), self.project.directory(), || {
                documents.open_texts()
            });
        self.output.update();
        self.handle_quit_request(ui.ctx());
        if ui.input_mut(|input| input.consume_shortcut(&SAVE_SHORTCUT)) {
            self.documents.save_active();
        }
        self.undo_from_keys(ui.ctx());
        self.follow_timeline();
        let mut action = self.show_menu(ui);
        egui::Panel::bottom("status_bar").show(ui, |ui| self.show_status_bar(ui));
        egui::Panel::left("hierarchy")
            .resizable(true)
            .default_size(SIDE_PANEL_WIDTH)
            .min_size(SIDE_PANEL_MIN_WIDTH)
            .show(ui, |ui| self.show_hierarchy(ui));
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(SIDE_PANEL_WIDTH)
            .min_size(SIDE_PANEL_MIN_WIDTH)
            .show(ui, |ui| match self.documents.active() {
                ActiveDocument::Scene => show_inspector(
                    ui,
                    &self.project,
                    self.documents.scene_mut(),
                    self.assets.images(),
                    AssetFiles {
                        folder: self.assets.directory(),
                        paths: self.assets.animations(),
                        keyframes: self.assets.keyframes(),
                        states: self.assets.states(),
                        models: self.assets.models(),
                        sounds: self.assets.sounds(),
                    },
                    self.live_check.outline().map(|outline| &outline.classes),
                ),
                ActiveDocument::Blueprint(tab) => {
                    ui.strong(texts::current().inspector);
                    ui.separator();
                    tab.show_inspector(ui, self.problems.problems());
                }
                ActiveDocument::Script(_) => {
                    ui.strong(texts::current().inspector);
                    ui.separator();
                    ui.weak(texts::current().script_inspector_hint());
                }
                ActiveDocument::Sound(_) => {
                    ui.strong(texts::current().inspector);
                    ui.separator();
                    ui.weak(texts::current().sound_inspector_hint());
                }
                ActiveDocument::Frames(tab) => {
                    ui.strong(texts::current().inspector);
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| tab.show_inspector(ui));
                }
                ActiveDocument::States(tab) => {
                    ui.strong(texts::current().inspector);
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| tab.show_inspector(ui));
                }
            });
        egui::Panel::bottom("bottom_panel")
            .resizable(true)
            .default_size(BOTTOM_PANEL_HEIGHT)
            .min_size(BOTTOM_PANEL_MIN_HEIGHT)
            .show(ui, |ui| self.show_bottom_panel(ui));
        egui::CentralPanel::default().show(ui, |ui| self.show_central_area(ui));
        if let Some(scene) = self.documents.scene_mut() {
            scene.show_class_choice(ui.ctx());
            scene.settle(ui.ctx());
        }
        if action.is_none() {
            action = self.ask_about_unsaved_changes(ui.ctx());
        }
        if action.is_none() {
            action = self
                .icon_size_chosen
                .take()
                .map(EditorAction::ChangeIconSize);
        }
        self.show_about(ui.ctx());
        action
    }

    /// The menu bar, and what its entries do. Returns what the editor
    /// window must do because of it.
    fn show_menu(&mut self, ui: &mut egui::Ui) -> Option<EditorAction> {
        let steps = self
            .shown_scene()
            .map_or_else(UndoSteps::default, |scene| UndoSteps {
                can_undo: scene.can_undo(),
                can_redo: scene.can_redo(),
            });
        let state = MenuState {
            can_save: self.documents.active_is_unsaved(),
            steps,
            compiling: self.problems.is_compiling(),
            playing: self.output.is_playing(),
            show_fps: self.show_fps,
        };
        let mut action = None;
        egui::Panel::top("menu_bar").show(ui, |ui| {
            match show_menu_bar(ui, state) {
                Some(MenuAction::Save) => self.documents.save_active(),
                Some(MenuAction::Undo) => {
                    if let Some(scene) = self.shown_scene() {
                        scene.undo();
                    }
                }
                Some(MenuAction::Redo) => {
                    if let Some(scene) = self.shown_scene() {
                        scene.redo();
                    }
                }
                Some(MenuAction::CloseProject) => action = self.leave(Leaving::CloseProject),
                Some(MenuAction::Compile) => {
                    // “Compile” checks the files, so it checks what is
                    // shown.
                    self.documents.save_all();
                    self.problems.compile(self.project.directory(), ui.ctx());
                }
                Some(MenuAction::Play) => self.check_and_play(PlayWay::Interpreted, ui.ctx()),
                Some(MenuAction::PlayCompiled) => {
                    self.check_and_play(PlayWay::Compiled, ui.ctx());
                }
                Some(MenuAction::Stop) => self.output.stop(),
                Some(MenuAction::ShowAbout) => self.about_open = true,
                Some(MenuAction::ChangeLanguage(language)) => {
                    action = Some(EditorAction::ChangeLanguage(language));
                }
                Some(MenuAction::SetShowFps(choice)) => {
                    match set_show_fps(self.project.directory(), choice) {
                        Ok(()) => self.show_fps = choice,
                        Err(error) => log::warn!("{error}"),
                    }
                }
                None => {}
            }
        });
        action
    }

    /// The scene, if its tab is the one shown.
    fn shown_scene(&mut self) -> Option<&mut SceneDocument> {
        match self.documents.active() {
            ActiveDocument::Scene => self.documents.scene_mut(),
            ActiveDocument::Blueprint(_)
            | ActiveDocument::Script(_)
            | ActiveDocument::Frames(_)
            | ActiveDocument::States(_)
            | ActiveDocument::Sound(_) => None,
        }
    }

    /// `Ctrl+Z` undoes the last change to the scene shown, and
    /// `Ctrl+Shift+Z` or `Ctrl+Y` redo it, while no text is being typed:
    /// text fields undo their own text.
    fn undo_from_keys(&mut self, context: &egui::Context) {
        if context.egui_wants_keyboard_input() {
            return;
        }
        let Some(scene) = self.shown_scene() else {
            return;
        };
        let (redo, undo) = context.input_mut(|input| {
            // `Ctrl+Z` also matches `Ctrl+Shift+Z`, so redo goes first.
            let redo = input.consume_shortcut(&REDO_SHORTCUT)
                || input.consume_shortcut(&REDO_OTHER_SHORTCUT);
            (redo, !redo && input.consume_shortcut(&UNDO_SHORTCUT))
        });
        if redo {
            scene.redo();
        } else if undo {
            scene.undo();
        }
    }

    /// Takes the result of a check that has just finished: its problems are
    /// shown, and “Play” plays the game if there are none.
    fn follow_check(&mut self, context: &egui::Context) {
        let Some(end) = self.problems.update() else {
            return;
        };
        match (end, self.play_after_check.take()) {
            (CheckEnd::WithProblems, Some(_)) => {
                self.output.refuse(texts::current().play_blocked);
                self.bottom_tab = BottomTab::Problems;
            }
            (CheckEnd::WithProblems, None) => self.bottom_tab = BottomTab::Problems,
            (CheckEnd::Clean, Some(way)) => self.play(way, context),
            (CheckEnd::Clean, None) => {}
        }
    }

    /// Saves and checks the project, to play it `way` if the check finds
    /// nothing: the game is played from what is shown.
    fn check_and_play(&mut self, way: PlayWay, context: &egui::Context) {
        self.documents.save_all();
        self.problems.compile(self.project.directory(), context);
        self.play_after_check = Some(way);
        self.bottom_tab = BottomTab::Output;
    }

    /// Plays `way` the project the last check found ready.
    fn play(&mut self, way: PlayWay, context: &egui::Context) {
        let mode = match (self.problems.played_program(), way) {
            (Err(reason), _) => return self.output.refuse(reason),
            (Ok(_), PlayWay::Interpreted) => PlayMode::Interpreted,
            (Ok(program), PlayWay::Compiled) => PlayMode::Compiled {
                name: self.project.settings().name().to_owned(),
                program: program.clone(),
            },
        };
        self.output.play(
            PlayRequest {
                directory: self.project.directory().to_path_buf(),
                mode,
            },
            context,
        );
    }

    /// Closing the window waits for an answer if blueprints are unsaved.
    fn handle_quit_request(&mut self, context: &egui::Context) {
        let requested = context.input(|input| input.viewport().close_requested());
        if requested && !self.quit_confirmed && self.documents.any_unsaved() {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.leaving = Some(Leaving::Quit);
        }
    }

    /// Does what `leaving` does, or asks first if it would lose unsaved
    /// changes. Returns what the application must do.
    fn leave(&mut self, leaving: Leaving) -> Option<EditorAction> {
        let unsaved = match &leaving {
            Leaving::CloseTab(index) => self.documents.is_unsaved(*index),
            Leaving::OpenScene(path) => {
                self.documents.scene_is_unsaved() && !self.documents.is_scene_open(path)
            }
            Leaving::CloseProject | Leaving::Quit => self.documents.any_unsaved(),
        };
        if unsaved {
            self.leaving = Some(leaving);
            None
        } else {
            self.finish_leaving(leaving)
        }
    }

    fn finish_leaving(&mut self, leaving: Leaving) -> Option<EditorAction> {
        match leaving {
            Leaving::CloseTab(index) => {
                self.documents.close(index);
                None
            }
            Leaving::OpenScene(path) => {
                self.last_scene.remember(&path);
                self.documents.open_scene(
                    self.assets.directory(),
                    path,
                    self.project.settings().dimension(),
                );
                None
            }
            Leaving::CloseProject => Some(EditorAction::CloseProject),
            Leaving::Quit => {
                self.quit_confirmed = true;
                Some(EditorAction::Quit)
            }
        }
    }

    /// The question about unsaved changes, while something waits for it.
    fn ask_about_unsaved_changes(&mut self, context: &egui::Context) -> Option<EditorAction> {
        let leaving = self.leaving.clone()?;
        let question = match leaving {
            Leaving::CloseTab(_) => texts::current().unsaved_tab_question,
            Leaving::OpenScene(_) => texts::current().unsaved_scene_question,
            Leaving::CloseProject | Leaving::Quit => texts::current().unsaved_files_question,
        };
        let choice = show_unsaved_dialog(context, question)?;
        self.leaving = None;
        match choice {
            UnsavedChoice::Cancel => None,
            UnsavedChoice::Discard => self.finish_leaving(leaving),
            UnsavedChoice::Save => {
                let saved = match leaving {
                    Leaving::CloseTab(index) => self.documents.save(index),
                    Leaving::OpenScene(_) => self.documents.save_scene(),
                    Leaving::CloseProject | Leaving::Quit => self.documents.save_all(),
                };
                // If a save fails, its tab says why and nothing is closed.
                if saved {
                    self.finish_leaving(leaving)
                } else {
                    None
                }
            }
        }
    }

    /// The tabs of the scene and the open blueprints, and the one shown.
    fn show_central_area(&mut self, ui: &mut egui::Ui) {
        if let Some(index) = self.documents.show_tabs(ui) {
            self.leave(Leaving::CloseTab(index));
        }
        match self.documents.active() {
            ActiveDocument::Scene => match &mut self.viewport {
                Ok(viewport) => viewport.show(ui, self.documents.scene_mut()),
                Err(error) => {
                    ui.colored_label(ERROR_COLOR, texts::current().viewport_error(error));
                }
            },
            ActiveDocument::Blueprint(tab) => tab.show(ui, self.problems.problems()),
            ActiveDocument::Frames(tab) => tab.show(ui, self.assets.images()),
            ActiveDocument::States(tab) => tab.show(ui, self.assets.keyframes()),
            ActiveDocument::Sound(tab) => tab.show(ui, &mut self.audio),
            ActiveDocument::Script(tab) => {
                let edited = tab.show(ui, self.live_check.problems(), self.live_check.outline());
                if edited.changed {
                    self.live_check.request();
                }
                if let Some(span) = edited.go_to {
                    self.go_to_definition(span);
                }
            }
        }
    }

    fn show_bottom_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::Assets,
                texts::current().assets,
            );
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::Animation,
                texts::current().animation_tab(),
            );
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::Problems,
                self.problems.title(),
            );
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::Output,
                texts::current().output,
            );
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::Console,
                texts::current().console,
            );
        });
        ui.separator();
        match self.bottom_tab {
            BottomTab::Assets => match self.assets.show(ui) {
                Some(AssetEvent::Opened(OpenedAsset::Blueprint(path))) => {
                    self.documents.open_blueprint(self.assets.directory(), path);
                }
                Some(AssetEvent::Opened(OpenedAsset::Scene(path))) => {
                    self.leave(Leaving::OpenScene(path));
                }
                Some(AssetEvent::Opened(OpenedAsset::Script(path))) => {
                    self.documents.open_script(self.assets.directory(), path);
                    self.live_check.request();
                }
                Some(AssetEvent::Opened(OpenedAsset::States(path))) => {
                    self.documents.open_states(self.assets.directory(), path);
                }
                Some(AssetEvent::Opened(OpenedAsset::Frames(path))) => {
                    self.documents.open_frames(self.assets.directory(), path);
                }
                Some(AssetEvent::Opened(OpenedAsset::Sound(path))) => {
                    self.documents.open_sound(self.assets.directory(), path);
                }
                Some(AssetEvent::Listen(path)) => {
                    self.audio.listen(self.assets.directory(), &path);
                }
                Some(AssetEvent::Gone(path)) => self.forget(&path),
                Some(AssetEvent::Renamed { from, to }) => {
                    self.forget(&from);
                    self.rename_class(&from, &to);
                }
                Some(AssetEvent::IconSizeChosen(size)) => self.icon_size_chosen = Some(size),
                None => {}
            },
            BottomTab::Problems => {
                if let Some(problem) = self.problems.show(ui) {
                    self.show_problem(&problem);
                }
            }
            BottomTab::Animation => self.show_timeline(ui),
            BottomTab::Output => self.output.show(ui),
            BottomTab::Console => self.console.show(ui),
        }
    }

    /// The timeline of the scene shown, if the scene is shown.
    fn show_timeline(&mut self, ui: &mut egui::Ui) {
        let classes = self.live_check.outline().map(|outline| &outline.classes);
        let scene_shown = matches!(self.documents.active(), ActiveDocument::Scene);
        match self.documents.scene_mut() {
            Some(scene) if scene_shown => scene.show_timeline(ui, classes),
            _ => {
                ui.weak(texts::current().timeline_no_animator());
            }
        }
    }

    /// Shows the timeline when an animator is selected, and tells the
    /// scene whether it is shown, so its animation is previewed only then.
    fn follow_timeline(&mut self) {
        let scene_shown = matches!(self.documents.active(), ActiveDocument::Scene);
        let Some(scene) = self.documents.scene_mut() else {
            return;
        };
        if scene.take_animator_selected() {
            self.bottom_tab = BottomTab::Animation;
        }
        scene.set_timeline_shown(scene_shown && self.bottom_tab == BottomTab::Animation);
    }

    /// Closes what was open from the file or folder at `path`, which is
    /// gone.
    fn forget(&mut self, path: &str) {
        self.documents.forget(path);
        if self.documents.scene_mut().is_none() {
            self.last_scene.forget();
        }
    }

    /// Renames, in the whole project, the class of the script or blueprint
    /// moved from `from` to `to` if it was called as the file. Files with
    /// unsaved changes are left as they are, and named in the console.
    fn rename_class(&mut self, from: &str, to: &str) {
        let assets = self.assets.directory().to_path_buf();
        let Some(rename) = ClassRename::after_moving(&assets, from, to) else {
            return;
        };
        let skipped = self.documents.unsaved_paths();
        let (changed, failed) = rename.apply(&assets, &skipped);
        self.documents
            .reload(&assets, &changed, self.project.settings().dimension());
        log::info!(
            "Renamed the class {} to {} in {}",
            rename.old,
            rename.new,
            changed.join(", ")
        );
        let left: Vec<String> = skipped
            .into_iter()
            .filter(|path| rename.uses(&assets, path))
            .chain(failed)
            .collect();
        if !left.is_empty() {
            log::warn!(
                "The class {} is still called so in {}: rename it there by hand",
                rename.old,
                left.join(", ")
            );
        }
        self.live_check.request();
    }

    /// Opens the blueprint or script of `problem` where the problem is.
    fn show_problem(&mut self, problem: &Problem) {
        let Some(path) = problem
            .file
            .strip_prefix(ASSETS_DIRECTORY)
            .and_then(|path| path.strip_prefix('/'))
        else {
            return;
        };
        let assets = self.assets.directory();
        if let Some(origin) = &problem.origin {
            self.documents.open_blueprint(assets, path.to_owned());
            if let ActiveDocument::Blueprint(tab) = self.documents.active() {
                tab.reveal(origin);
            }
        } else if path.ends_with(SCRIPT_EXTENSION) {
            self.documents.open_script(assets, path.to_owned());
            self.live_check.request();
            if let (ActiveDocument::Script(tab), Some(position)) =
                (self.documents.active(), problem.position)
            {
                tab.reveal(position);
            }
        }
    }

    /// Opens the file of the program's place `span` there: a script with
    /// the cursor on it, or a blueprint (whose StarScript is never shown).
    fn go_to_definition(&mut self, span: Span) {
        let Some((path, position)) = self.live_check.locate(span) else {
            return;
        };
        let assets = self.assets.directory();
        if path.ends_with(SCRIPT_EXTENSION) {
            self.documents.open_script(assets, path);
            if let ActiveDocument::Script(tab) = self.documents.active() {
                tab.reveal(position);
            }
        } else if path.ends_with(BLUEPRINT_EXTENSION) {
            self.documents.open_blueprint(assets, path);
        }
    }

    /// The “Scene” panel: the tree of the open scene's nodes.
    fn show_hierarchy(&mut self, ui: &mut egui::Ui) {
        ui.strong(texts::current().hierarchy);
        ui.separator();
        match self.documents.scene_mut() {
            Some(scene) => scene.show_tree(ui, self.assets.images()),
            None => {
                ui.weak(texts::current().no_scene_open);
            }
        }
    }

    fn show_status_bar(&mut self, ui: &mut egui::Ui) {
        let preparing = self.is_preparing_engine();
        let settings = self.project.settings();
        ui.horizontal(|ui| {
            ui.label(settings.name());
            ui.weak(settings.dimension().label());
            ui.weak(self.project.directory().display().to_string());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!("{ENGINE_NAME} {ENGINE_VERSION}"));
                ui.separator();
                self.problems.show_summary(ui);
                self.output.show_summary(ui);
                if preparing {
                    show_engine_preparing(ui);
                }
            });
        });
    }

    fn show_about(&mut self, context: &egui::Context) {
        egui::Window::new(format!("{} {ENGINE_NAME}", texts::current().about))
            .open(&mut self.about_open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(context, |ui| {
                ui.strong(format!("{ENGINE_NAME} {ENGINE_VERSION}"));
                ui.label(texts::current().about_description);
                ui.separator();
                ui.label(texts::current().license_notice);
                ui.label(texts::current().game_exception_notice);
                ui.weak(texts::current().no_warranty);
                ui.hyperlink_to(texts::current().source_code, REPOSITORY_URL);
            });
    }
}

impl EditorWindow {
    /// Whether the engine is still being prepared, while no game is: a
    /// “Play” waiting for it already says it is building.
    fn is_preparing_engine(&mut self) -> bool {
        let preparing = self
            .warm_up
            .as_mut()
            .is_some_and(EngineWarmUp::is_preparing);
        if !preparing {
            self.warm_up = None;
        }
        preparing && !self.output.is_playing()
    }
}

/// Starts preparing the engine, redrawing the editor when it is ready.
fn start_warm_up(context: &egui::Context) -> Option<EngineWarmUp> {
    let context = context.clone();
    EngineWarmUp::start(move || context.request_repaint())
}

fn show_engine_preparing(ui: &mut egui::Ui) {
    let texts = texts::current();
    ui.spinner();
    ui.weak(texts.preparing_engine)
        .on_hover_text(texts.preparing_engine_hint);
}

/// Where the source code is published, as the GPL asks programs to tell users.
const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");
