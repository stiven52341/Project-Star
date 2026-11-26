//! The “Animation” tab: a bar to choose, create, rename, duplicate and
//! delete the animations of the file, change their length and looping, and
//! play them; and their lanes of keys under it.

use eframe::egui;
use star_engine::prelude::SceneOutline;
use star_engine::star_scene::{TrackAnimation, TrackAnimations};
use star_script::semantic::ClassTable;

use super::{Timeline, edits, lanes, paths};
use crate::style::ERROR_COLOR;
use crate::texts;

/// How much the length changes per pixel dragged, in seconds.
const LENGTH_DRAG_SPEED: f32 = 0.01;

/// The `.anim` file of the animator shown.
pub(in crate::scene_editor) struct TimelineFile<'file> {
    /// Where it is in the assets folder.
    pub(in crate::scene_editor) path: &'file str,
    /// Its animations, or why it could not be read.
    pub(in crate::scene_editor) animations: Result<&'file mut TrackAnimations, &'file str>,
}

/// What the timeline changed this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::scene_editor) enum Change {
    /// The animations changed.
    Edited,
    /// The animation `from` is called `to` now: animators that start with
    /// it start with `to`.
    Renamed { from: String, to: String },
    /// The animation was deleted: animators that start with it start with
    /// none.
    Deleted(String),
}

impl Timeline {
    /// Shows the animations of `file`, the file of the animator shown in
    /// `outline`, offering calls to the methods of `classes`. Returns what
    /// changed, if anything.
    pub(in crate::scene_editor) fn show(
        &mut self,
        ui: &mut egui::Ui,
        outline: &SceneOutline,
        file: Option<TimelineFile<'_>>,
        classes: Option<&ClassTable>,
    ) -> Option<Change> {
        let texts = texts::current();
        let (Some(animator), Some(file)) = (self.animator, file) else {
            ui.weak(texts.timeline_no_animator());
            return None;
        };
        let animations = match file.animations {
            Ok(animations) => animations,
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts.animations_unreadable(file.path, error));
                return None;
            }
        };
        self.keep_animation_valid(animations);
        let mut change = self.show_bar(ui, file.path, animations);
        let Some(animation) = self.animation.as_deref().and_then(|name| {
            animations
                .animations
                .iter_mut()
                .find(|animation| animation.name == name)
        }) else {
            ui.weak(texts.timeline_no_animations());
            return change;
        };
        self.advance(ui, animation);
        ui.separator();
        let Some(root) = paths::animated_root(outline.nodes(), animator) else {
            return change;
        };
        if lanes::show(ui, self, animation, outline.nodes(), root, classes) {
            change = change.or(Some(Change::Edited));
        }
        change
    }

    /// Shows the first animation if the one shown is gone, and keeps the
    /// selection and the playhead within the one shown.
    fn keep_animation_valid(&mut self, animations: &TrackAnimations) {
        if self
            .animation
            .as_deref()
            .is_none_or(|name| animations.animation(name).is_none())
        {
            self.animation = animations
                .animations
                .first()
                .map(|animation| animation.name.clone());
            self.selection.clear();
        }
        let Some(animation) = self
            .animation
            .as_deref()
            .and_then(|name| animations.animation(name))
        else {
            return;
        };
        self.time = self.time.clamp(0.0, animation.length);
        self.selection
            .retain(|&mark| edits::mark_time(animation, mark).is_some());
    }

    /// Moves the playhead on while playing, starting over at the end of a
    /// looping animation and stopping at the end of the others.
    fn advance(&mut self, ui: &egui::Ui, animation: &TrackAnimation) {
        if !self.playing {
            return;
        }
        self.time += ui.input(|input| input.stable_dt);
        if self.time >= animation.length {
            if animation.looping {
                self.time %= animation.length;
            } else {
                self.time = animation.length;
                self.playing = false;
            }
        }
        ui.ctx().request_repaint();
    }

    /// Shows the animation chosen, or its new name while it is renamed,
    /// the buttons that manage the animations, and those that play them.
    fn show_bar(
        &mut self,
        ui: &mut egui::Ui,
        path: &str,
        animations: &mut TrackAnimations,
    ) -> Option<Change> {
        ui.horizontal(|ui| {
            ui.weak(path);
            let mut change = if self.renaming.is_some() {
                self.show_renaming(ui, animations)
            } else {
                self.show_choice(ui, animations)
            };
            let Some(animation) = self.animation.as_deref().and_then(|name| {
                animations
                    .animations
                    .iter_mut()
                    .find(|animation| animation.name == name)
            }) else {
                return change;
            };
            ui.separator();
            if Self::show_length_and_looping(ui, animation) {
                change = change.or(Some(Change::Edited));
            }
            ui.separator();
            self.show_play_buttons(ui, animation);
            ui.weak(format!("{:.2} s", self.time));
            change
        })
        .inner
    }

    /// The animations to choose from, and the buttons that create,
    /// rename, duplicate and delete them.
    fn show_choice(
        &mut self,
        ui: &mut egui::Ui,
        animations: &mut TrackAnimations,
    ) -> Option<Change> {
        let texts = texts::current();
        let shown = self.animation.clone().unwrap_or_default();
        egui::ComboBox::from_id_salt("timeline_animation")
            .selected_text(&shown)
            .show_ui(ui, |ui| {
                for animation in &animations.animations {
                    let current = animation.name == shown;
                    if ui.selectable_label(current, &animation.name).clicked() && !current {
                        self.show_animation(animation.name.clone());
                    }
                }
            });
        if ui.button(texts.new_animation()).clicked() {
            let name = edits::add_animation(animations);
            self.show_animation(name);
            return Some(Change::Edited);
        }
        self.animation.as_ref()?;
        if ui.button(texts.rename_animation()).clicked() {
            self.renaming = Some(shown.clone());
        }
        if ui.button(texts.duplicate_animation()).clicked() {
            let copy = edits::duplicate_animation(animations, &shown)?;
            self.show_animation(copy);
            return Some(Change::Edited);
        }
        if ui.button(texts.delete_animation()).clicked() {
            edits::delete_animation(animations, &shown);
            self.animation = None;
            return Some(Change::Deleted(shown));
        }
        None
    }

    /// The name being typed for the animation shown, with buttons to give
    /// it that name or leave it as it was. `Enter` gives it.
    fn show_renaming(
        &mut self,
        ui: &mut egui::Ui,
        animations: &mut TrackAnimations,
    ) -> Option<Change> {
        let texts = texts::current();
        let (Some(from), Some(to)) = (self.animation.clone(), self.renaming.as_mut()) else {
            self.renaming = None;
            return None;
        };
        let field = ui.text_edit_singleline(to);
        field.request_focus();
        let typed = to.trim().to_owned();
        let entered = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        let free = typed == from || animations.animation(&typed).is_none() && !typed.is_empty();
        let apply = ui
            .add_enabled(free, egui::Button::new(texts.apply_name()))
            .clicked()
            || entered && free;
        if !free {
            ui.colored_label(ERROR_COLOR, texts.animation_name_taken());
        }
        if ui.button(texts.cancel).clicked()
            || ui.input(|input| input.key_pressed(egui::Key::Escape))
        {
            self.renaming = None;
            return None;
        }
        if !apply {
            return None;
        }
        self.renaming = None;
        if typed == from || !edits::rename_animation(animations, &from, &typed) {
            return None;
        }
        self.animation = Some(typed.clone());
        Some(Change::Renamed { from, to: typed })
    }

    pub(super) fn show_animation(&mut self, name: String) {
        self.animation = Some(name);
        self.time = 0.0;
        self.playing = false;
        self.selection.clear();
        self.renaming = None;
    }

    /// How long the animation lasts, never less than its last key, and
    /// whether it loops.
    fn show_length_and_looping(ui: &mut egui::Ui, animation: &mut TrackAnimation) -> bool {
        let texts = texts::current();
        ui.weak(texts.animation_length());
        let shortest = edits::last_time(animation).max(edits::MIN_LENGTH);
        let lengthened = ui
            .add(
                egui::DragValue::new(&mut animation.length)
                    .speed(LENGTH_DRAG_SPEED)
                    .range(shortest..=f32::MAX)
                    .suffix(" s"),
            )
            .on_hover_text(texts.animation_length_hint())
            .changed();
        lengthened
            | ui.checkbox(&mut animation.looping, texts.clip_looping())
                .changed()
    }

    fn show_play_buttons(&mut self, ui: &mut egui::Ui, animation: &TrackAnimation) {
        let texts = texts::current();
        let label = if self.playing {
            texts.pause()
        } else {
            texts.play_animation()
        };
        if ui.button(label).clicked() {
            if !self.playing && self.time >= animation.length {
                self.time = 0.0;
            }
            self.playing = !self.playing;
        }
        if ui.button(texts.stop_animation()).clicked() {
            self.playing = false;
            self.time = 0.0;
        }
    }
}
