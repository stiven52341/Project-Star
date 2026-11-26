//! The `export` fields of a node's class in the inspector: each shows the
//! value the scene gives it or, until it gives one, the class's own, and
//! changing it writes it in the node's `[node.fields]`.

use std::collections::BTreeMap;

use eframe::egui;
use star_engine::star_scene::FieldValue;
use star_script::semantic::{
    COLOR_CLASS, ClassTable, FieldDefault, FieldInfo, InspectorHints, NumberRange, Type,
};

use super::fields::DRAG_SPEED;
use super::key_buttons::KeyButtons;
use crate::style::ERROR_COLOR;
use crate::texts;

/// What kind of value a field holds, as the inspector edits it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum FieldKind {
    /// A whole number between `min` and `max`.
    Whole {
        min: i64,
        max: i64,
    },
    Decimal,
    Bool,
    Text,
    Color,
}

/// One `export` field of a class.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ExportedField {
    pub(super) name: String,
    pub(super) kind: FieldKind,
    /// What the class gives it, or the empty value of its kind if the
    /// class computes it.
    pub(super) default: FieldValue,
    /// What its decorators say: its range, help, title and text box.
    pub(super) hints: InspectorHints,
}

/// The `export` fields of the class `class` and its bases, bases first;
/// `None` if the program has no such class.
pub(super) fn exported_fields(classes: &ClassTable, class: &str) -> Option<Vec<ExportedField>> {
    let id = classes.find(class)?;
    let mut owners: Vec<_> = classes.ancestors(id).collect();
    owners.reverse();
    Some(
        owners
            .into_iter()
            .flat_map(|owner| &classes.class(owner).fields)
            .filter(|field| field.is_exported)
            .filter_map(|field| exported_field(classes, field))
            .collect(),
    )
}

fn exported_field(classes: &ClassTable, field: &FieldInfo) -> Option<ExportedField> {
    let kind = match &field.ty {
        Type::Short => FieldKind::Whole {
            min: i16::MIN.into(),
            max: i16::MAX.into(),
        },
        Type::Int => FieldKind::Whole {
            min: i32::MIN.into(),
            max: i32::MAX.into(),
        },
        Type::BigInt => FieldKind::Whole {
            min: i64::MIN,
            max: i64::MAX,
        },
        Type::Float | Type::Double => FieldKind::Decimal,
        Type::Bool => FieldKind::Bool,
        Type::String => FieldKind::Text,
        Type::Class(id) if classes.class(*id).name == COLOR_CLASS => FieldKind::Color,
        _ => return None,
    };
    let default = field
        .default
        .as_ref()
        .and_then(|default| shown(kind, &plain(default)))
        .unwrap_or_else(|| empty(kind));
    Some(ExportedField {
        name: field.name.clone(),
        kind,
        default,
        hints: field.hints.clone(),
    })
}

/// A first value of a class, as scenes write values.
#[allow(clippy::cast_possible_truncation)] // Color parts are from 0 to 1.
fn plain(default: &FieldDefault) -> FieldValue {
    match default {
        FieldDefault::Bool(truth) => FieldValue::Bool(*truth),
        FieldDefault::Int(number) => FieldValue::Int(*number),
        FieldDefault::Float(number) => FieldValue::Float(*number),
        FieldDefault::Text(text) => FieldValue::Text(text.clone()),
        FieldDefault::Color(parts) => FieldValue::Color(parts.map(|part| part as f32)),
    }
}

/// The value of a field of `kind` when nothing gives one.
const fn empty(kind: FieldKind) -> FieldValue {
    match kind {
        FieldKind::Whole { .. } => FieldValue::Int(0),
        FieldKind::Decimal => FieldValue::Float(0.0),
        FieldKind::Bool => FieldValue::Bool(false),
        FieldKind::Text => FieldValue::Text(String::new()),
        FieldKind::Color => FieldValue::Color([1.0; 4]),
    }
}

/// `value` as a field of `kind` shows it: whole numbers also fill decimal
/// fields. `None` if it does not fit.
#[allow(clippy::cast_precision_loss)] // As games take whole numbers for decimals.
fn shown(kind: FieldKind, value: &FieldValue) -> Option<FieldValue> {
    match (kind, value) {
        (FieldKind::Whole { min, max }, FieldValue::Int(number))
            if (min..=max).contains(number) =>
        {
            Some(value.clone())
        }
        (FieldKind::Decimal, FieldValue::Int(number)) => Some(FieldValue::Float(*number as f64)),
        (FieldKind::Decimal, FieldValue::Float(_))
        | (FieldKind::Bool, FieldValue::Bool(_))
        | (FieldKind::Text, FieldValue::Text(_))
        | (FieldKind::Color, FieldValue::Color(_)) => Some(value.clone()),
        _ => None,
    }
}

/// The rows of `fields`, with the values of `values`, and below them the
/// values `values` gives fields the class does not export, to remove.
/// Returns whether `values` changed.
pub(super) fn edit_exported(
    ui: &mut egui::Ui,
    values: &mut BTreeMap<String, FieldValue>,
    fields: &[ExportedField],
    keys: Option<&KeyButtons>,
) -> bool {
    let mut changed = false;
    for field in fields {
        changed |= field_row(ui, values, field, keys);
    }
    let strays: Vec<String> = values
        .keys()
        .filter(|name| !fields.iter().any(|field| &field.name == *name))
        .cloned()
        .collect();
    for name in strays {
        ui.colored_label(ERROR_COLOR, &name)
            .on_hover_text(texts::current().not_an_export_field);
        if ui.small_button(texts::current().remove).clicked() {
            values.remove(&name);
            changed = true;
        }
        ui.end_row();
    }
    changed
}

/// The row of one field: its key button, if `keys` are shown, its name
/// (strong if the scene gives it a value), what edits its value, and a
/// button back to the class's value.
fn field_row(
    ui: &mut egui::Ui,
    values: &mut BTreeMap<String, FieldValue>,
    field: &ExportedField,
    keys: Option<&KeyButtons>,
) -> bool {
    if let Some(header) = &field.hints.header {
        ui.strong(header);
        ui.end_row();
    }
    let given = values
        .get(&field.name)
        .and_then(|value| shown(field.kind, value));
    let name = ui
        .horizontal(|ui| {
            if let Some(keys) = keys {
                keys.show(ui, &[field.name.as_str()]);
            }
            if given.is_some() {
                ui.strong(&field.name)
            } else {
                ui.weak(&field.name)
            }
        })
        .inner;
    if let Some(tooltip) = &field.hints.tooltip {
        name.on_hover_text(tooltip);
    }
    let mut value = given.clone().unwrap_or_else(|| field.default.clone());
    let mut reset = false;
    let edited = ui
        .horizontal(|ui| {
            let edited = edit_value(ui, field.kind, &field.hints, &mut value);
            if given.is_some() || values.contains_key(&field.name) {
                reset = ui
                    .small_button(texts::current().reset)
                    .on_hover_text(texts::current().reset_field_hint)
                    .clicked();
            }
            edited
        })
        .inner;
    ui.end_row();
    if reset {
        values.remove(&field.name);
        return true;
    }
    if edited {
        values.insert(field.name.clone(), value);
    }
    edited
}

/// What edits `value`, a value of `kind`, as `hints` say: a slider for
/// numbers with a range, a box of several lines for a multiline text.
/// Returns whether it changed.
fn edit_value(
    ui: &mut egui::Ui,
    kind: FieldKind,
    hints: &InspectorHints,
    value: &mut FieldValue,
) -> bool {
    match (kind, value) {
        (FieldKind::Whole { min, max }, FieldValue::Int(number)) => match hints.range {
            Some(range) => {
                let (low, high) = whole_limits(range, min, max);
                let step = range.step.unwrap_or(1.0).max(1.0);
                ui.add(egui::Slider::new(number, low..=high).step_by(step))
                    .changed()
            }
            None => ui
                .add(egui::DragValue::new(number).range(min..=max))
                .changed(),
        },
        (FieldKind::Decimal, FieldValue::Float(number)) => match hints.range {
            Some(range) => {
                let slider = egui::Slider::new(number, range.min..=range.max);
                let slider = match range.step {
                    Some(step) => slider.step_by(step),
                    None => slider,
                };
                ui.add(slider).changed()
            }
            None => ui
                .add(egui::DragValue::new(number).speed(DRAG_SPEED))
                .changed(),
        },
        (FieldKind::Bool, FieldValue::Bool(truth)) => ui.checkbox(truth, "").changed(),
        (FieldKind::Text, FieldValue::Text(text)) if hints.multiline => ui
            .add(egui::TextEdit::multiline(text).desired_rows(MULTILINE_ROWS))
            .changed(),
        (FieldKind::Text, FieldValue::Text(text)) => ui.text_edit_singleline(text).changed(),
        (FieldKind::Color, FieldValue::Color(parts)) => {
            ui.color_edit_button_rgba_unmultiplied(parts).changed()
        }
        _ => false,
    }
}

/// How many lines a multiline text box shows.
const MULTILINE_ROWS: usize = 3;

/// The whole numbers of `range` that a field between `min` and `max` can
/// hold: its limits rounded inwards, and kept within the field's.
#[allow(clippy::cast_possible_truncation)] // Clamped to the field's limits.
fn whole_limits(range: NumberRange, min: i64, max: i64) -> (i64, i64) {
    #[allow(clippy::cast_precision_loss)] // Limits of whole fields are exact enough.
    let (field_min, field_max) = (min as f64, max as f64);
    let low = range.min.ceil().clamp(field_min, field_max) as i64;
    let high = range.max.floor().clamp(field_min, field_max) as i64;
    (low, high.max(low))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes() -> ClassTable {
        let source = "class Unit extends Node {\n  export var lives = 3\n  init() {\n    \
                      super.init(\"Unit\")\n  }\n}\n\
                      class Rock extends Unit {\n  export var speed = 2\n  \
                      export var tint = Color(1.0, 0.5, 0.0, 1.0)\n  export var title = \"a\" + \"b\"\n  \
                      var hidden = 1\n  export var small: short = 0\n}\n";
        star_script::check_source(source)
            .expect("valid program")
            .analysis
            .classes()
            .clone()
    }

    #[test]
    fn exported_fields_come_with_their_bases_first_and_their_values() {
        let fields = exported_fields(&classes(), "Rock").unwrap();

        let names: Vec<_> = fields.iter().map(|field| field.name.as_str()).collect();
        assert_eq!(names, ["lives", "speed", "tint", "title", "small"]);
        assert_eq!(fields[0].default, FieldValue::Int(3));
        assert_eq!(fields[1].default, FieldValue::Int(2));
        assert_eq!(fields[2].default, FieldValue::Color([1.0, 0.5, 0.0, 1.0]));
        assert_eq!(fields[3].default, FieldValue::Text(String::new()));
        assert_eq!(
            fields[4].kind,
            FieldKind::Whole {
                min: i64::from(i16::MIN),
                max: i64::from(i16::MAX)
            }
        );
        assert_eq!(exported_fields(&classes(), "Ghost"), None);
    }

    #[test]
    fn whole_ranges_are_rounded_inwards_and_kept_within_the_field() {
        let range = |min, max| NumberRange {
            min,
            max,
            step: None,
        };

        assert_eq!(whole_limits(range(0.5, 9.5), -100, 100), (1, 9));
        assert_eq!(whole_limits(range(-1000.0, 1000.0), -100, 100), (-100, 100));
    }

    #[test]
    fn decorators_reach_the_fields() {
        let source = "class Rock extends Node {\n  @header(\"Motion\")\n  @range(0, 5)\n  \
                      export var speed = 2.0\n  init() {\n    super.init(\"Rock\")\n  }\n}\n";
        let checked = star_script::check_source(source).expect("valid program");

        let fields = exported_fields(checked.analysis.classes(), "Rock").unwrap();

        assert_eq!(fields[0].hints.header.as_deref(), Some("Motion"));
        assert!(fields[0].hints.range.is_some());
    }

    #[test]
    fn values_are_shown_as_their_field_takes_them() {
        assert_eq!(
            shown(FieldKind::Decimal, &FieldValue::Int(2)),
            Some(FieldValue::Float(2.0))
        );
        assert_eq!(
            shown(FieldKind::Whole { min: 0, max: 9 }, &FieldValue::Int(12)),
            None
        );
        assert_eq!(shown(FieldKind::Text, &FieldValue::Bool(true)), None);
    }
}
