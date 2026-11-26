//! Reading glTF 2.0 models (`.glb`, and `.gltf` with the files it names)
//! into a [`Model`], through the `gltf` crate, which stays hidden here.
//!
//! The model is the default scene of the file (or its first one): every
//! mesh its nodes show, placed where the nodes put it, with the base
//! color, metalness, roughness and base color texture of its material.
//! Triangles are all that is read; points and lines are left out. The
//! file's first skin becomes the model's skeleton, with the animations
//! that move its joints (`skeleton`).

use std::collections::HashMap;

use gltf::buffer::Source as BufferSource;
use gltf::image::Source as ImageSource;
use gltf::mesh::Mode;
use star_core::model::IDENTITY_COLUMNS;
use star_core::{EngineError, EngineResult, Image, Model, ModelMaterial, ModelPart, ModelVertex};

use crate::jpeg_image::decode_jpeg;
use crate::png_image::decode_png;

/// What URIs of data inside the file start with, which are not read: such
/// files are better exported as `.glb`.
const DATA_URI: &str = "data:";
/// The set of texture coordinates the base color reads.
const FIRST_TEX_COORDS: u32 = 0;
/// The set of joints and weights skinned meshes read.
const FIRST_SET: u32 = 0;

/// Reads the bytes of a file a `.gltf` names, by the URI it gives, from
/// the model's folder.
pub type ReadBeside<'a> = &'a dyn Fn(&str) -> EngineResult<Vec<u8>>;

/// Decodes a glTF model from the bytes of its file; `read_beside` reads
/// the buffers and images a `.gltf` keeps in other files.
///
/// # Errors
///
/// [`EngineError::Asset`] if the bytes are not valid glTF, a file it
/// names cannot be read, it keeps data inside URIs, an image is not PNG
/// or JPEG, or it shows no triangles.
pub fn decode_gltf(bytes: &[u8], read_beside: ReadBeside<'_>) -> EngineResult<Model> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|error| invalid(&format!("{error}")))?;
    let buffers = read_buffers(&gltf, read_beside)?;
    let mut reader = PartReader {
        buffers: &buffers,
        read_beside,
        images: Vec::new(),
        image_of: HashMap::new(),
        parts: Vec::new(),
    };
    let scene = gltf.default_scene().or_else(|| gltf.scenes().next());
    for node in scene.iter().flat_map(gltf::Scene::nodes) {
        reader.read_node(&node, IDENTITY_COLUMNS)?;
    }
    let Some(skin) = gltf.skins().next() else {
        return Model::new(reader.parts, reader.images);
    };
    let read = skeleton::read_skeleton(&gltf, &skin, &buffers)?;
    let mut parts = reader.parts;
    read.renumber(&mut parts);
    let animations = read.animations(&gltf, &buffers);
    Model::new(parts, reader.images)?.with_skeleton(read.skeleton, animations)
}

/// The bytes of every buffer of the file, in its order.
fn read_buffers(gltf: &gltf::Gltf, read_beside: ReadBeside<'_>) -> EngineResult<Vec<Vec<u8>>> {
    gltf.buffers()
        .map(|buffer| match buffer.source() {
            BufferSource::Bin => gltf
                .blob
                .clone()
                .ok_or_else(|| invalid("its binary buffer is missing")),
            BufferSource::Uri(uri) => read_uri(uri, read_beside),
        })
        .collect()
}

fn read_uri(uri: &str, read_beside: ReadBeside<'_>) -> EngineResult<Vec<u8>> {
    if uri.starts_with(DATA_URI) {
        return Err(invalid(
            "it keeps data inside the file as text: export it as .glb, or with its .bin beside it",
        ));
    }
    read_beside(uri)
}

/// What reading the parts of a model keeps as it goes.
struct PartReader<'a> {
    buffers: &'a [Vec<u8>],
    read_beside: ReadBeside<'a>,
    images: Vec<Image>,
    /// The place in `images` of each image of the file decoded so far.
    image_of: HashMap<usize, usize>,
    parts: Vec<ModelPart>,
}

impl PartReader<'_> {
    /// Reads the meshes of `node` and the nodes under it, placed by
    /// `parent`, the place of its parent in the model.
    fn read_node(&mut self, node: &gltf::Node<'_>, parent: [[f32; 4]; 4]) -> EngineResult<()> {
        let place = multiply(parent, node.transform().matrix());
        if let Some(mesh) = node.mesh() {
            let skinned = node.skin().is_some();
            for primitive in mesh.primitives() {
                if primitive.mode() == Mode::Triangles {
                    let part = self.read_primitive(&primitive, place, skinned)?;
                    self.parts.push(part);
                }
            }
        }
        for child in node.children() {
            self.read_node(&child, place)?;
        }
        Ok(())
    }

    fn read_primitive(
        &mut self,
        primitive: &gltf::Primitive<'_>,
        transform: [[f32; 4]; 4],
        skinned: bool,
    ) -> EngineResult<ModelPart> {
        let buffers = self.buffers;
        let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
        let positions: Vec<[f32; 3]> = reader
            .read_positions()
            .ok_or_else(|| invalid("a mesh has no positions"))?
            .collect();
        let indices: Vec<u32> = match reader.read_indices() {
            Some(indices) => indices.into_u32().collect(),
            None => (0..u32::try_from(positions.len())
                .map_err(|_| invalid("a mesh is too big"))?)
                .collect(),
        };
        let normals: Vec<[f32; 3]> = reader
            .read_normals()
            .map_or_else(|| smooth_normals(&positions, &indices), Iterator::collect);
        let uvs: Vec<[f32; 2]> = reader
            .read_tex_coords(FIRST_TEX_COORDS)
            .map(|coords| coords.into_f32().collect())
            .unwrap_or_default();
        let joints: Vec<[u16; 4]> = reader
            .read_joints(FIRST_SET)
            .map(|joints| joints.into_u16().collect())
            .unwrap_or_default();
        let weights: Vec<[f32; 4]> = reader
            .read_weights(FIRST_SET)
            .map(|weights| weights.into_f32().collect())
            .unwrap_or_default();
        let skinned = skinned && !joints.is_empty() && !weights.is_empty();
        let vertices = positions
            .iter()
            .enumerate()
            .map(|(index, &position)| {
                let mut vertex = ModelVertex::new(
                    position,
                    normals.get(index).copied().unwrap_or([0.0, 0.0, 1.0]),
                    uvs.get(index).copied().unwrap_or_default(),
                );
                if skinned {
                    vertex.joints = joints.get(index).copied().unwrap_or_default();
                    vertex.weights = normalized_weights(weights.get(index).copied());
                }
                vertex
            })
            .collect();
        let material = self.read_material(&primitive.material())?;
        Ok(ModelPart {
            vertices,
            indices,
            material,
            transform,
            skinned,
        })
    }

    fn read_material(&mut self, material: &gltf::Material<'_>) -> EngineResult<ModelMaterial> {
        let pbr = material.pbr_metallic_roughness();
        let texture = pbr
            .base_color_texture()
            .map(|info| self.image(&info.texture().source()))
            .transpose()?;
        Ok(ModelMaterial {
            base_color: pbr.base_color_factor(),
            metallic: pbr.metallic_factor(),
            roughness: pbr.roughness_factor(),
            texture,
        })
    }

    /// The place in the model's images of `image`, decoded the first time
    /// a material shows it.
    fn image(&mut self, image: &gltf::Image<'_>) -> EngineResult<usize> {
        if let Some(&known) = self.image_of.get(&image.index()) {
            return Ok(known);
        }
        let decoded = match image.source() {
            ImageSource::View { view, mime_type } => {
                let bytes = self
                    .buffers
                    .get(view.buffer().index())
                    .and_then(|buffer| buffer.get(view.offset()..view.offset() + view.length()))
                    .ok_or_else(|| invalid("an image is outside its buffer"))?;
                decode_image(bytes, mime_type)?
            }
            ImageSource::Uri { uri, mime_type } => {
                let bytes = read_uri(uri, self.read_beside)?;
                decode_image(&bytes, mime_type.unwrap_or(uri))?
            }
        };
        self.images.push(decoded);
        let index = self.images.len() - 1;
        self.image_of.insert(image.index(), index);
        Ok(index)
    }
}

/// Decodes an image named by its MIME type or its file name.
fn decode_image(bytes: &[u8], kind: &str) -> EngineResult<Image> {
    let kind = kind.to_ascii_lowercase();
    if kind.ends_with("png") {
        decode_png(bytes)
    } else if kind.ends_with("jpeg") || kind.ends_with("jpg") {
        decode_jpeg(bytes)
    } else {
        Err(invalid(&format!(
            "its image \"{kind}\" is not PNG nor JPEG"
        )))
    }
}

/// `weights` scaled to add up to 1; a vertex without weights follows its
/// first joint alone.
fn normalized_weights(weights: Option<[f32; 4]>) -> [f32; 4] {
    let weights = weights.unwrap_or_default();
    let total: f32 = weights.iter().sum();
    if total > f32::EPSILON {
        weights.map(|weight| weight / total)
    } else {
        [1.0, 0.0, 0.0, 0.0]
    }
}

/// The product of two matrices given as columns: `first` after `second`.
pub(crate) fn multiply(first: [[f32; 4]; 4], second: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut product = [[0.0; 4]; 4];
    for (column, out) in second.iter().zip(&mut product) {
        for (row, cell) in out.iter_mut().enumerate() {
            *cell = (0..4).map(|k| first[k][row] * column[k]).sum();
        }
    }
    product
}

/// Normals for a mesh that has none: at each corner, the average of the
/// faces of the triangles that share it.
fn smooth_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut sums = vec![[0.0_f32; 3]; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let corners: Vec<usize> = triangle
            .iter()
            .filter_map(|&index| usize::try_from(index).ok())
            .collect();
        let [Some(a), Some(b), Some(c)] = [0, 1, 2].map(|k| {
            corners
                .get(k)
                .and_then(|&index| positions.get(index).copied())
        }) else {
            continue;
        };
        let face = cross(sub(b, a), sub(c, a));
        for &corner in &corners {
            if let Some(sum) = sums.get_mut(corner) {
                for (total, part) in sum.iter_mut().zip(face) {
                    *total += part;
                }
            }
        }
    }
    sums.into_iter().map(normalized).collect()
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalized(vector: [f32; 3]) -> [f32; 3] {
    let length = vector.iter().map(|part| part * part).sum::<f32>().sqrt();
    if length > f32::EPSILON {
        vector.map(|part| part / length)
    } else {
        [0.0, 0.0, 1.0]
    }
}

pub(crate) fn invalid(reason: &str) -> EngineError {
    EngineError::Asset(format!("invalid glTF model: {reason}"))
}

mod skeleton;
#[cfg(test)]
mod tests;
