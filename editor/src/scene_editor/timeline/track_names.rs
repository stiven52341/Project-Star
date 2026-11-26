//! The name of each track beside its lane: the node and the property it
//! changes, how it goes from key to key (with a drawing of that curve),
//! and a button that removes it.

use eframe::egui;
use star_engine::star_scene::{Interpolation, NodeOutline, PropertyTrack};

use super::lanes::{self, LANE_HEIGHT, NAMES_WIDTH};
use crate::style::WARNING_COLOR;
use crate::texts;

/// How wide the drawing of a curve is.
const CURVE_WIDTH: f32 = 22.0;
/// How tall the drawing of a curve is.
const CURVE_HEIGHT: f32 = 14.0;
/// How many straight pieces draw a curve.
const CURVE_PIECES: u16 = 16;
/// How wide the choice of the interpolation is.
const INTERPOLATION_WIDTH: f32 = 92.0;

/// Every interpolation, in the order they are offered.
pub(super) const INTERPOLATIONS: [Interpolation; 5] = [
    Interpolation::Step,
    Interpolation::Linear,
    Interpolation::EaseIn,
    Interpolation::EaseOut,
    Interpolation::EaseInOut,
];

/// What the user did with a track's name.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct NameOutcome {
    /// Its interpolation changed.
    pub(super) changed: bool,
    /// It is to be removed.
    pub(super) removed: bool,
}

/// The name of `track`, which animates a node under the node at `root` of
/// `nodes`, in a cell as wide as the column of names.
pub(super) fn show(
    ui: &mut egui::Ui,
    track: &mut PropertyTrack,
    nodes: &[NodeOutline],
    root: usize,
) -> NameOutcome {
    let texts = texts::current();
    let size = egui::vec2(NAMES_WIDTH, LANE_HEIGHT);
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    ui.allocate_ui_with_layout(size, layout, |ui| {
        ui.set_min_size(size);
        let mut outcome = NameOutcome {
            removed: ui
                .small_button("×")
                .on_hover_text(texts.remove_track())
                .clicked(),
            changed: false,
        };
        let label = format!(
            "{} · {}",
            lanes::node_label(nodes, root, &track.node),
            track.property
        );
        let shown = if lanes::node_exists(nodes, root, &track.node) {
            ui.add(egui::Label::new(label).truncate())
        } else {
            ui.add(egui::Label::new(egui::RichText::new(label).color(WARNING_COLOR)).truncate())
                .on_hover_text(texts.track_node_missing())
        };
        shown.on_hover_text(texts.track_hint());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let id = egui::IdSalt::new(("interpolation", &track.node, &track.property));
            outcome.changed = interpolation_choice(ui, id, &mut track.interpolation);
            draw_curve(ui, track.interpolation);
        });
        outcome
    })
    .inner
}

/// The interpolation of a track, to choose another; each choice shows its
/// curve.
fn interpolation_choice(
    ui: &mut egui::Ui,
    id: egui::IdSalt,
    interpolation: &mut Interpolation,
) -> bool {
    let texts = texts::current();
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .width(INTERPOLATION_WIDTH)
        .selected_text(texts.interpolation_name(*interpolation))
        .show_ui(ui, |ui| {
            for choice in INTERPOLATIONS {
                ui.horizontal(|ui| {
                    draw_curve(ui, choice);
                    let current = choice == *interpolation;
                    if ui
                        .selectable_label(current, texts.interpolation_name(choice))
                        .clicked()
                        && !current
                    {
                        *interpolation = choice;
                        changed = true;
                    }
                });
            }
        })
        .response
        .on_hover_text(texts.interpolation_hint());
    changed
}

/// A small drawing of how `interpolation` goes from one key (bottom left)
/// to the next (top right).
fn draw_curve(ui: &mut egui::Ui, interpolation: Interpolation) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(CURVE_WIDTH, CURVE_HEIGHT), egui::Sense::hover());
    let color = ui.visuals().text_color();
    let points: Vec<egui::Pos2> = (0..=CURVE_PIECES)
        .map(|piece| {
            let progress = f32::from(piece) / f32::from(CURVE_PIECES);
            // A step holds until the next key, where it jumps.
            let amount = if piece == CURVE_PIECES {
                1.0
            } else {
                interpolation.apply(progress)
            };
            egui::pos2(
                rect.left() + progress * rect.width(),
                rect.bottom() - amount * rect.height(),
            )
        })
        .collect();
    ui.painter()
        .add(egui::Shape::line(points, egui::Stroke::new(1.5, color)));
}
