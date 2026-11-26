//! The lane of the calls: its name, with a menu that adds a call at the
//! playhead to a method of a node animated, and how each call is drawn.
//! Only methods an animation can call are offered: public, of the node's
//! script class, not static, without arguments and neither `async` nor
//! `parallel` (decision 79).

use eframe::egui;
use star_engine::star_scene::{NodeOutline, TrackAnimation};
use star_script::semantic::{ClassOrigin, ClassTable, Execution};

use super::lanes::{LANE_HEIGHT, NAMES_WIDTH};
use super::{Timeline, edits, paths};
use crate::texts;

/// Half the size of a call's mark.
const CALL_RADIUS: f32 = 4.5;
/// The size of the method's name beside a call.
const CALL_TEXT_SIZE: f32 = 10.0;

/// The methods of the class `class`, and of its script bases, that an
/// animation can call, in the order they are written.
pub(super) fn callable_methods(classes: &ClassTable, class: &str) -> Vec<String> {
    let Some(id) = classes.find(class) else {
        return Vec::new();
    };
    let mut methods: Vec<String> = Vec::new();
    for ancestor in classes.ancestors(id) {
        let info = classes.class(ancestor);
        if info.origin == ClassOrigin::Native {
            continue;
        }
        for method in &info.methods {
            let callable = !method.is_static
                && !method.is_private
                && !method.is_override
                && method.execution == Execution::Direct
                && method.parameters.is_empty();
            if callable && !methods.contains(&method.name) {
                methods.push(method.name.clone());
            }
        }
    }
    methods
}

/// The nodes animated whose class has methods an animation can call, with
/// their paths and those methods.
fn callable_nodes(
    nodes: &[NodeOutline],
    root: usize,
    classes: &ClassTable,
) -> Vec<(String, Vec<String>)> {
    paths::animated_nodes(nodes, root)
        .into_iter()
        .filter_map(|(index, path)| {
            let class = nodes.get(index)?.class.as_deref()?;
            let methods = callable_methods(classes, class);
            (!methods.is_empty()).then_some((path, methods))
        })
        .collect()
}

/// The name of the calls' lane, with the menu that adds one. Returns
/// whether one was added.
pub(super) fn show_name(
    ui: &mut egui::Ui,
    timeline: &mut Timeline,
    animation: &mut TrackAnimation,
    nodes: &[NodeOutline],
    root: usize,
    classes: Option<&ClassTable>,
) -> bool {
    let texts = texts::current();
    let size = egui::vec2(NAMES_WIDTH, LANE_HEIGHT);
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    ui.allocate_ui_with_layout(size, layout, |ui| {
        ui.set_min_size(size);
        ui.label(texts.calls()).on_hover_text(texts.calls_hint());
        let callable = classes
            .map(|classes| callable_nodes(nodes, root, classes))
            .unwrap_or_default();
        let mut added = None;
        let menu = ui.add_enabled_ui(!callable.is_empty(), |ui| {
            ui.menu_button(texts.add_call(), |ui| {
                for (path, methods) in &callable {
                    ui.menu_button(super::lanes::node_label(nodes, root, path), |ui| {
                        for method in methods {
                            if ui.button(method).clicked() {
                                added = Some((path.clone(), method.clone()));
                                ui.close();
                            }
                        }
                    });
                }
            })
        });
        if callable.is_empty() {
            menu.response
                .on_disabled_hover_text(texts.no_callable_methods());
        }
        let Some((path, method)) = added else {
            return false;
        };
        let mark = edits::add_call(animation, &path, timeline.time, &method);
        timeline.selection = vec![mark];
        true
    })
    .inner
}

/// A call: a circle at `at` with its method's name beside it.
pub(super) fn draw_call(
    painter: &egui::Painter,
    at: egui::Pos2,
    method: &str,
    color: egui::Color32,
) {
    painter.circle_filled(at, CALL_RADIUS, color);
    painter.text(
        at + egui::vec2(CALL_RADIUS + 2.0, 0.0),
        egui::Align2::LEFT_CENTER,
        method,
        egui::FontId::proportional(CALL_TEXT_SIZE),
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animations_call_public_methods_without_arguments_of_script_classes() {
        let source = "class Unit extends Node {\n  init() {\n    super.init(\"Unit\")\n  }\n  \
                      func hit() {\n  }\n}\n\
                      class Knight extends Unit {\n  func swing() {\n  }\n  \
                      func aim(power: int) {\n  }\n  private func rest() {\n  }\n  \
                      static func make() {\n  }\n  async func charge() {\n  }\n  \
                      override func update(delta: float) {\n  }\n}\n";
        let classes = star_script::check_source(source)
            .expect("valid program")
            .analysis
            .classes()
            .clone();

        assert_eq!(callable_methods(&classes, "Knight"), ["swing", "hit"]);
        assert_eq!(callable_methods(&classes, "Missing"), Vec::<String>::new());
    }
}
