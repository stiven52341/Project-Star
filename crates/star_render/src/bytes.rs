//! Serialization of plain values into the byte layout GPU buffers expect.

/// Appends `values` to `bytes` in little-endian order, the byte order of
/// every GPU the engine targets.
pub(crate) fn extend_with_f32s(bytes: &mut Vec<u8>, values: &[f32]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_are_appended_in_little_endian_order() {
        let mut bytes = vec![0xAA];

        extend_with_f32s(&mut bytes, &[1.0]);

        assert_eq!(bytes, [0xAA, 0x00, 0x00, 0x80, 0x3F]);
    }
}
