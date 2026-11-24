//! The values a scene gives the `export` fields of a node's class, and
//! their conversion to the type of each field.
//!
//! A scene names fields as text, so the generated code checks each value
//! when the scene is loaded: a wrong one stops the scene with a reason,
//! as an unknown class does.

/// A value of a scene's `[node.fields]`.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneValue {
    /// `true` or `false`.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A number with decimals.
    Float(f64),
    /// A text.
    Text(String),
    /// Red, green, blue and opacity, from `0.0` to `1.0`.
    Color([f32; 4]),
}

/// Why `value` does not fit the field `field`, of type `ty`.
fn mismatch(field: &str, ty: &str) -> String {
    format!("the field `{field}` takes a value of type `{ty}`")
}

/// `value` for the `short` field `field`.
///
/// # Errors
///
/// Returns why if it is not a whole number that fits.
pub fn short(field: &str, value: &SceneValue) -> Result<i16, String> {
    match value {
        SceneValue::Int(number) => i16::try_from(*number).ok(),
        _ => None,
    }
    .ok_or_else(|| mismatch(field, "short"))
}

/// `value` for the `int` field `field`.
///
/// # Errors
///
/// Returns why if it is not a whole number that fits.
pub fn int(field: &str, value: &SceneValue) -> Result<i32, String> {
    match value {
        SceneValue::Int(number) => i32::try_from(*number).ok(),
        _ => None,
    }
    .ok_or_else(|| mismatch(field, "int"))
}

/// `value` for the `bigint` field `field`.
///
/// # Errors
///
/// Returns why if it is not a whole number.
pub fn bigint(field: &str, value: &SceneValue) -> Result<i64, String> {
    match value {
        SceneValue::Int(number) => Ok(*number),
        _ => Err(mismatch(field, "bigint")),
    }
}

/// `value` for the `double` field `field`: whole numbers are taken too.
///
/// # Errors
///
/// Returns why if it is not a number.
#[allow(clippy::cast_precision_loss)]
pub fn double(field: &str, value: &SceneValue) -> Result<f64, String> {
    match value {
        SceneValue::Float(number) => Ok(*number),
        // A scene writes `speed = 3` as readily as `speed = 3.0`.
        SceneValue::Int(number) => Ok(*number as f64),
        _ => Err(mismatch(field, "double")),
    }
}

/// `value` for the `float` field `field`: whole numbers are taken too.
///
/// # Errors
///
/// Returns why if it is not a number.
#[allow(clippy::cast_possible_truncation)]
pub fn float(field: &str, value: &SceneValue) -> Result<f32, String> {
    // `float` is how StarScript keeps decimals: the closest one is kept.
    double(field, value)
        .map(|number| number as f32)
        .map_err(|_| mismatch(field, "float"))
}

/// `value` for the `bool` field `field`.
///
/// # Errors
///
/// Returns why if it is not `true` or `false`.
pub fn boolean(field: &str, value: &SceneValue) -> Result<bool, String> {
    match value {
        SceneValue::Bool(truth) => Ok(*truth),
        _ => Err(mismatch(field, "bool")),
    }
}

/// `value` for the `string` field `field`.
///
/// # Errors
///
/// Returns why if it is not a text.
pub fn text(field: &str, value: &SceneValue) -> Result<String, String> {
    match value {
        SceneValue::Text(text) => Ok(text.clone()),
        _ => Err(mismatch(field, "string")),
    }
}

/// `value` for the `Color` field `field`, as red, green, blue and opacity.
///
/// # Errors
///
/// Returns why if it is not a color.
pub fn color(field: &str, value: &SceneValue) -> Result<[f32; 4], String> {
    match value {
        SceneValue::Color(color) => Ok(*color),
        _ => Err(mismatch(field, "Color")),
    }
}

/// Why a scene cannot give a value to `field` of `class`: it is not one of
/// its `export` fields.
#[must_use]
pub fn unknown_field(class: &str, field: &str) -> String {
    format!("`{class}` has no `export` field `{field}`")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_fit_the_fields_they_can() {
        assert_eq!(int("lives", &SceneValue::Int(3)), Ok(3));
        assert_eq!(float("speed", &SceneValue::Int(3)), Ok(3.0));
        assert_eq!(double("speed", &SceneValue::Float(2.5)), Ok(2.5));
        assert_eq!(
            short("small", &SceneValue::Int(70_000)),
            Err("the field `small` takes a value of type `short`".to_owned())
        );
        assert_eq!(
            int("lives", &SceneValue::Float(3.0)),
            Err("the field `lives` takes a value of type `int`".to_owned())
        );
    }

    #[test]
    fn other_values_fit_only_their_own_kind() {
        assert_eq!(boolean("alive", &SceneValue::Bool(true)), Ok(true));
        assert_eq!(
            text("title", &SceneValue::Text("Boss".to_owned())),
            Ok("Boss".to_owned())
        );
        assert_eq!(
            color("tint", &SceneValue::Color([1.0, 0.0, 0.0, 1.0])),
            Ok([1.0, 0.0, 0.0, 1.0])
        );
        assert!(color("tint", &SceneValue::Bool(true)).is_err());
        assert!(text("title", &SceneValue::Int(1)).is_err());
    }
}
