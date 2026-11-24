//! The values a scene gives the `export` fields of the StarScript class a
//! node is built as, in its `[node.fields]` table.

use serde::{Deserialize, Serialize};

/// A value of a `[node.fields]` table, as TOML writes it: `speed = 2.5`,
/// `lives = 3`, `alive = true`, `title = "Boss"`, `tint = [1.0, 0.0, 0.0,
/// 1.0]`. Which field takes which is checked against the class when the
/// project is checked and when the scene is loaded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldValue {
    /// `true` or `false`.
    Bool(bool),
    /// A whole number; a field of decimals takes it too.
    Int(i64),
    /// A number with decimals.
    Float(f64),
    /// A text.
    Text(String),
    /// A color: red, green, blue and opacity, from 0 to 1.
    Color([f32; 4]),
}
