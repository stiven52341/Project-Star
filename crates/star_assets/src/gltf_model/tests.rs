use super::*;

/// The magic number, version and chunk types of `.glb` files.
const GLB_MAGIC: &[u8; 4] = b"glTF";
const GLB_VERSION: u32 = 2;
const JSON_CHUNK: u32 = 0x4E4F_534A;
const BIN_CHUNK: u32 = 0x004E_4942;

/// A triangle's three positions (as `f32`) and indices (as `u16`, padded
/// to 4 bytes).
fn triangle_bytes() -> Vec<u8> {
    let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let indices: [u16; 4] = [0, 1, 2, 0];
    positions
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .chain(indices.iter().flat_map(|value| value.to_le_bytes()))
        .collect()
}

/// A one-pixel PNG.
fn pixel_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&[10, 20, 30, 255]).unwrap();
    writer.finish().unwrap();
    bytes
}

/// The JSON of a red triangle moved by `(1, 2, 3)`, whose buffer is
/// `buffer` and has `extra` (JSON members) besides.
fn triangle_json(buffer: &str, extra: &str) -> String {
    format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "scene": 0,
        "scenes": [{{ "nodes": [0] }}],
        "nodes": [{{ "mesh": 0, "translation": [1.0, 2.0, 3.0] }}],
        "meshes": [{{ "primitives": [{{ "attributes": {{ "POSITION": 0 }}, "indices": 1, "material": 0 }}] }}],
        "materials": [{{ "pbrMetallicRoughness": {{
            "baseColorFactor": [1.0, 0.0, 0.0, 1.0], "metallicFactor": 0.5, "roughnessFactor": 0.25 }} }}],
        "buffers": [{buffer}],
        "bufferViews": [
            {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
            {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
        ],
        "accessors": [
            {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
               "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0] }},
            {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
        ]{extra}
    }}"#
    )
}

/// A `.glb` of `json` and its binary chunk `bin`.
fn glb(json: &str, bin: &[u8]) -> Vec<u8> {
    let mut json = json.as_bytes().to_vec();
    json.resize(json.len().next_multiple_of(4), b' ');
    let mut bin = bin.to_vec();
    bin.resize(bin.len().next_multiple_of(4), 0);
    let chunk_header = 8;
    let total = 12 + chunk_header + json.len() + chunk_header + bin.len();
    let length = |bytes: usize| u32::try_from(bytes).unwrap().to_le_bytes();
    let mut file = Vec::new();
    file.extend_from_slice(GLB_MAGIC);
    file.extend_from_slice(&GLB_VERSION.to_le_bytes());
    file.extend_from_slice(&length(total));
    file.extend_from_slice(&length(json.len()));
    file.extend_from_slice(&JSON_CHUNK.to_le_bytes());
    file.extend_from_slice(&json);
    file.extend_from_slice(&length(bin.len()));
    file.extend_from_slice(&BIN_CHUNK.to_le_bytes());
    file.extend_from_slice(&bin);
    file
}

fn nothing_beside(uri: &str) -> EngineResult<Vec<u8>> {
    Err(EngineError::Asset(format!("no file \"{uri}\"")))
}

#[test]
fn gltf_files_read_their_buffers_beside_them() {
    let json = triangle_json(r#"{ "byteLength": 44, "uri": "triangle.bin" }"#, "");
    let beside = |uri: &str| match uri {
        "triangle.bin" => Ok(triangle_bytes()),
        _ => nothing_beside(uri),
    };

    let model = decode_gltf(json.as_bytes(), &beside).unwrap();

    let [part] = model.parts() else {
        panic!("one mesh");
    };
    assert_eq!(part.indices, [0, 1, 2]);
    assert_eq!(part.vertices[1].position, [1.0, 0.0, 0.0]);
    assert_eq!(part.transform[3], [1.0, 2.0, 3.0, 1.0]);
    assert_eq!(part.material.base_color, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(
        (part.material.metallic, part.material.roughness),
        (0.5, 0.25)
    );
    assert_eq!(
        part.vertices[0].normal,
        [0.0, 0.0, 1.0],
        "made from the face"
    );
}

#[test]
fn glb_files_carry_their_buffers_and_images() {
    let png = pixel_png();
    let image_view = format!(
        r#"{{ "buffer": 0, "byteOffset": 44, "byteLength": {} }}"#,
        png.len()
    );
    let mut bin = triangle_bytes();
    bin.extend_from_slice(&png);
    let json = triangle_json(
        &format!(r#"{{ "byteLength": {} }}"#, bin.len().next_multiple_of(4)),
        r#", "images": [{ "bufferView": 2, "mimeType": "image/png" }],
            "textures": [{ "source": 0 }]"#,
    )
    .replace(
        r#""roughnessFactor": 0.25 }"#,
        r#""roughnessFactor": 0.25, "baseColorTexture": { "index": 0 } }"#,
    )
    .replace(
        r#"{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }"#,
        &format!(r#"{{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}, {image_view}"#),
    );

    let model = decode_gltf(&glb(&json, &bin), &nothing_beside).unwrap();

    assert_eq!(model.parts()[0].material.texture, Some(0));
    assert_eq!(model.images()[0].pixels(), [10, 20, 30, 255]);
}

#[test]
fn data_inside_uris_and_broken_files_are_rejected() {
    let json = triangle_json(
        r#"{ "byteLength": 44, "uri": "data:application/octet-stream;base64,AAAA" }"#,
        "",
    );

    let inside = decode_gltf(json.as_bytes(), &nothing_beside).unwrap_err();
    let broken = decode_gltf(b"not a model", &nothing_beside).unwrap_err();
    let missing = decode_gltf(
        triangle_json(r#"{ "byteLength": 44, "uri": "gone.bin" }"#, "").as_bytes(),
        &nothing_beside,
    )
    .unwrap_err();

    assert!(inside.to_string().contains(".glb"), "{inside}");
    assert!(broken.to_string().contains("glTF"), "{broken}");
    assert!(missing.to_string().contains("gone.bin"), "{missing}");
}

#[test]
fn matrices_multiply_as_columns() {
    let moved = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [5.0, 0.0, 0.0, 1.0],
    ];
    let doubled = [
        [2.0, 0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0, 0.0],
        [0.0, 0.0, 2.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    // Doubled first, then moved: the move is not doubled.
    assert_eq!(multiply(moved, doubled)[3], [5.0, 0.0, 0.0, 1.0]);
    assert_eq!(multiply(doubled, moved)[3], [10.0, 0.0, 0.0, 1.0]);
}
