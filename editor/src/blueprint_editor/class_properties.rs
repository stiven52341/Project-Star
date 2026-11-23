//! The inspector of the blueprint itself, while no node is selected: its
//! class, its variables and the signature of the graph shown. Renaming a
//! variable, a method or a parameter renames it where nodes use it too.

use eframe::egui;
use star_blueprint::{Blueprint, Graph, Literal, Parameter, Variable};

use super::edit;
use super::literal_field::literal_field;
use crate::style::PROPERTY_SPACING;
use crate::texts;

/// Names `+` gives: `variable1`, `param1`…
const NEW_VARIABLE_PREFIX: &str = "variable";
const NEW_PARAMETER_PREFIX: &str = "param";
/// The type of a new parameter, the most common one in games.
const NEW_PARAMETER_TYPE: &str = "float";

/// The fields of `blueprint` and of its graph at `graph`. Returns whether
/// the blueprint changed.
pub(super) fn show_class_properties(
    ui: &mut egui::Ui,
    blueprint: &mut Blueprint,
    graph: usize,
) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(texts::current().class)
        .default_open(true)
        .show(ui, |ui| changed |= class_fields(ui, blueprint));
    egui::CollapsingHeader::new(texts::current().variables)
        .default_open(true)
        .show(ui, |ui| changed |= variables_fields(ui, blueprint));
    if graph < blueprint.graphs.len() {
        egui::CollapsingHeader::new(texts::current().method)
            .default_open(true)
            .show(ui, |ui| changed |= method_fields(ui, blueprint, graph));
    }
    changed
}

fn class_fields(ui: &mut egui::Ui, blueprint: &mut Blueprint) -> bool {
    let mut changed = false;
    egui::Grid::new("class_properties")
        .num_columns(2)
        .spacing(PROPERTY_SPACING)
        .show(ui, |ui| {
            ui.weak(texts::current().name);
            changed |= ui.text_edit_singleline(&mut blueprint.class).changed();
            ui.end_row();
            ui.weak(texts::current().extends);
            changed |= optional_text(ui, &mut blueprint.extends);
            ui.end_row();
            ui.weak(texts::current().base_arguments)
                .on_hover_text(texts::current().base_arguments_hint);
            changed |= base_arguments_field(ui, &mut blueprint.base_arguments);
            ui.end_row();
        });
    changed
}

/// A field for text that may be missing: empty means none.
fn optional_text(ui: &mut egui::Ui, value: &mut Option<String>) -> bool {
    let mut text = value.clone().unwrap_or_default();
    let changed = ui.text_edit_singleline(&mut text).changed();
    if changed {
        *value = (!text.is_empty()).then_some(text);
    }
    changed
}

fn base_arguments_field(ui: &mut egui::Ui, arguments: &mut Vec<Literal>) -> bool {
    let mut changed = false;
    let mut removed = None;
    ui.vertical(|ui| {
        for (index, argument) in arguments.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                changed |= literal_field(ui, ("base_argument", index), argument);
                if ui
                    .small_button("×")
                    .on_hover_text(texts::current().remove)
                    .clicked()
                {
                    removed = Some(index);
                }
            });
        }
        if ui
            .small_button("+")
            .on_hover_text(texts::current().add_argument)
            .clicked()
        {
            arguments.push(Literal::String(String::new()));
            changed = true;
        }
    });
    if let Some(index) = removed {
        arguments.remove(index);
        changed = true;
    }
    changed
}

fn variables_fields(ui: &mut egui::Ui, blueprint: &mut Blueprint) -> bool {
    let mut changed = false;
    let mut removed = None;
    let mut renamed = Vec::new();
    for (index, variable) in blueprint.variables.iter_mut().enumerate() {
        ui.group(|ui| {
            changed |= variable_fields(ui, index, variable, &mut renamed);
            if ui.small_button(texts::current().remove_variable).clicked() {
                removed = Some(index);
            }
        });
    }
    if ui.button(texts::current().add_variable).clicked() {
        let taken: Vec<&str> = blueprint
            .variables
            .iter()
            .map(|variable| variable.name.as_str())
            .collect();
        blueprint.variables.push(Variable {
            name: edit::unused_name(NEW_VARIABLE_PREFIX, &taken),
            ty: None,
            default: Some(Literal::Float(0.0)),
            private: false,
        });
        changed = true;
    }
    if let Some(index) = removed {
        blueprint.variables.remove(index);
        changed = true;
    }
    for (old, new) in renamed {
        edit::rename_variable(blueprint, &old, &new);
    }
    changed
}

/// The fields of the variable at `index`; a new name is put in `renamed`.
fn variable_fields(
    ui: &mut egui::Ui,
    index: usize,
    variable: &mut Variable,
    renamed: &mut Vec<(String, String)>,
) -> bool {
    let mut changed = false;
    egui::Grid::new(("variable", index))
        .num_columns(2)
        .spacing(PROPERTY_SPACING)
        .show(ui, |ui| {
            ui.weak(texts::current().name);
            let old = variable.name.clone();
            if ui.text_edit_singleline(&mut variable.name).changed() {
                renamed.push((old, variable.name.clone()));
                changed = true;
            }
            ui.end_row();
            ui.weak(texts::current().type_label)
                .on_hover_text(texts::current().variable_type_hint);
            changed |= optional_text(ui, &mut variable.ty);
            ui.end_row();
            ui.weak(texts::current().default_value);
            ui.horizontal(|ui| {
                let mut has_default = variable.default.is_some();
                if ui.checkbox(&mut has_default, "").changed() {
                    variable.default = has_default.then_some(Literal::Float(0.0));
                    changed = true;
                }
                if let Some(default) = &mut variable.default {
                    changed |= literal_field(ui, ("variable_default", index), default);
                }
            });
            ui.end_row();
            ui.weak(texts::current().private);
            changed |= ui.checkbox(&mut variable.private, "").changed();
            ui.end_row();
        });
    changed
}

/// The signature of the graph at `index`.
fn method_fields(ui: &mut egui::Ui, blueprint: &mut Blueprint, index: usize) -> bool {
    let Some(graph) = blueprint.graphs.get_mut(index) else {
        return false;
    };
    let old_name = graph.name.clone();
    let mut changed = false;
    egui::Grid::new("method_properties")
        .num_columns(2)
        .spacing(PROPERTY_SPACING)
        .show(ui, |ui| {
            ui.weak(texts::current().name);
            changed |= ui.text_edit_singleline(&mut graph.name).changed();
            ui.end_row();
            ui.weak(texts::current().parameters);
            changed |= parameters_field(ui, graph);
            ui.end_row();
            ui.weak(texts::current().returns);
            changed |= optional_text(ui, &mut graph.returns);
            ui.end_row();
            ui.weak(texts::current().overrides)
                .on_hover_text(texts::current().override_hint);
            changed |= ui.checkbox(&mut graph.is_override, "").changed();
            ui.end_row();
            ui.weak(texts::current().private);
            changed |= ui.checkbox(&mut graph.private, "").changed();
            ui.end_row();
        });
    if changed {
        edit::remove_broken_links(graph);
        let new_name = graph.name.clone();
        if new_name != old_name {
            edit::rename_method(blueprint, &old_name, &new_name);
        }
    }
    changed
}

/// One row per parameter (name and type) with `×`, and `+` to add one.
fn parameters_field(ui: &mut egui::Ui, graph: &mut Graph) -> bool {
    let mut changed = false;
    let mut removed = None;
    let mut renamed = Vec::new();
    ui.vertical(|ui| {
        for (index, parameter) in graph.parameters.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let old = parameter.name.clone();
                if ui.text_edit_singleline(&mut parameter.name).changed() {
                    renamed.push((old, parameter.name.clone()));
                    changed = true;
                }
                ui.label(":");
                changed |= ui.text_edit_singleline(&mut parameter.ty).changed();
                if ui
                    .small_button("×")
                    .on_hover_text(texts::current().remove)
                    .clicked()
                {
                    removed = Some(index);
                }
            });
        }
        if ui
            .small_button("+")
            .on_hover_text(texts::current().add_parameter)
            .clicked()
        {
            let taken: Vec<&str> = graph
                .parameters
                .iter()
                .map(|parameter| parameter.name.as_str())
                .collect();
            graph.parameters.push(Parameter {
                name: edit::unused_name(NEW_PARAMETER_PREFIX, &taken),
                ty: NEW_PARAMETER_TYPE.to_owned(),
            });
            changed = true;
        }
    });
    if let Some(index) = removed {
        graph.parameters.remove(index);
        changed = true;
    }
    for (old, new) in renamed {
        edit::rename_parameter(graph, &old, &new);
    }
    changed
}
