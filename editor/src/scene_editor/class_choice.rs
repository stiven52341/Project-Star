//! A script with several classes dropped on a node, in the tree, the view
//! or the inspector: a menu where it was dropped asks which class the
//! node is built as.

use eframe::egui;

use crate::texts;

/// The menu, open until a class is chosen, `Escape` is pressed or the
/// user clicks elsewhere.
#[derive(Debug)]
pub(super) struct ClassChoice {
    pub(super) node: usize,
    pub(super) classes: Vec<String>,
    pub(super) at: egui::Pos2,
}

/// What the user did with the menu this frame.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ChoiceOutcome {
    Open,
    Chosen(String),
    Dismissed,
}

impl ClassChoice {
    /// Draws the menu over everything else.
    pub(super) fn show(&self, ctx: &egui::Context) -> ChoiceOutcome {
        let mut chosen = None;
        let menu = egui::Area::new(egui::Id::unique("scene_class_choice"))
            .fixed_pos(self.at)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.weak(texts::current().choose_class);
                    for class in &self.classes {
                        if ui.button(class).clicked() {
                            chosen = Some(class.clone());
                        }
                    }
                });
            });
        if let Some(class) = chosen {
            ChoiceOutcome::Chosen(class)
        } else if menu.response.clicked_elsewhere()
            || ctx.input(|input| input.key_pressed(egui::Key::Escape))
        {
            ChoiceOutcome::Dismissed
        } else {
            ChoiceOutcome::Open
        }
    }
}
