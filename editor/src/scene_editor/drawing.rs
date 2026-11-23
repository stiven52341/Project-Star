//! Drawing an open scene in the view: where each node ends up, what it
//! shows, and a mark on the node selected.

use star_engine::MeshFile;
use star_engine::prelude::{
    Affine2, AmbientLight, Color, DirectionalLight, Frame, GlobalTransform2D, GlobalTransform3D,
    MAX_LIGHTS, Mat4, Material, Mesh3dId, MeshId, MeshShape, SceneOutline, SpriteNode,
    TextureFilter, TextureId, Vec2, Vec3, Vertex,
};
use star_engine::star_assets::AssetPath;
use star_engine::star_scene::{NodeOutline, SpriteOutline};

/// Lights scenes that have none of their own, so their meshes can be
/// seen: a sun from above and in front, and a soft ambient light.
const PREVIEW_SUN_DIRECTION: Vec3 = Vec3::new(-0.4, -1.0, -0.6);
const PREVIEW_SUN_INTENSITY: f32 = 2.0;
const PREVIEW_AMBIENT_INTENSITY: f32 = 0.3;
/// How far the mark of a selected sprite stands out of its edges, in
/// world units, so the sprite (drawn over lines) does not hide it.
const SPRITE_MARK_MARGIN: f32 = 0.05;
/// How big the mark of a selected node that shows nothing is, in world
/// units.
pub(super) const POINT_MARK_SIZE: f32 = 0.5;
/// How thick the mark of a selected plane is, so its box is visible.
const PLANE_MARK_THICKNESS: f32 = 0.02;

/// What the drawing needs from the GPU: the textures of image files and
/// the meshes of shapes, made once each. `None` if they cannot be made;
/// the node is then drawn without them.
pub(crate) trait SceneGpu {
    fn texture(&mut self, path: &AssetPath, filter: TextureFilter) -> Option<TextureId>;
    fn mesh(&mut self, unit: MeshShape) -> Option<Mesh3dId>;
}

/// The line meshes that mark the node selected, made once by the view.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Marks {
    /// A square from `(0, 0)` to `(1, 1)`, on the XY plane.
    pub(crate) square: MeshId,
    /// A cube from `-0.5` to `0.5` on every axis.
    pub(crate) cube: MeshId,
    /// Three segments from `-0.5` to `0.5` along each axis.
    pub(crate) cross: MeshId,
}

/// Adds to `frame` what the nodes of `outline` show, where the game would
/// show them, and the mark of the node `selected`.
pub(crate) fn draw_scene(
    outline: &SceneOutline,
    selected: Option<usize>,
    gpu: &mut impl SceneGpu,
    marks: Marks,
    frame: &mut Frame,
) {
    let nodes = outline.nodes();
    let places = places(nodes);
    let mut lit = false;
    for (node, place) in nodes.iter().zip(&places) {
        if let (Some(sprite), Some(global)) = (&node.sprite, place.flat) {
            draw_sprite(sprite, global, gpu, frame);
        }
        if let Some(global) = place.solid {
            draw_mesh(node, global, gpu, frame);
            if let Some(light) = node.light
                && frame.lights().len() < MAX_LIGHTS
            {
                frame.add_light(light.to_light(GlobalTransform3D::from_matrix(global)));
                lit = true;
            }
        }
    }
    if !lit {
        light_for_preview(frame);
    }
    let selected = selected.and_then(|index| Some((nodes.get(index)?, places.get(index)?)));
    if let Some((node, place)) = selected {
        mark(node, place, marks, frame);
    }
}

/// Where a node ends up in the world, in 2D and in 3D: only in the spaces
/// it has a transform for, as the engine places nodes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Place {
    pub(super) flat: Option<Affine2>,
    pub(super) solid: Option<Mat4>,
}

/// The place of every node. Parents come first in a scene, so one pass
/// finds them all; a node without a transform passes its parent's on to
/// its children.
pub(super) fn places(nodes: &[NodeOutline]) -> Vec<Place> {
    let mut inherited: Vec<(Affine2, Mat4)> = Vec::with_capacity(nodes.len());
    let mut places = Vec::with_capacity(nodes.len());
    for node in nodes {
        let (parent_flat, parent_solid) = node
            .parent
            .and_then(|parent| inherited.get(parent).copied())
            .unwrap_or((Affine2::IDENTITY, Mat4::IDENTITY));
        let flat = node
            .transform
            .map(|transform| parent_flat * transform.to_affine());
        let solid = node
            .transform_3d
            .map(|transform| parent_solid * transform.to_mat4());
        inherited.push((flat.unwrap_or(parent_flat), solid.unwrap_or(parent_solid)));
        places.push(Place { flat, solid });
    }
    places
}

fn draw_sprite(
    sprite: &SpriteOutline,
    global: Affine2,
    gpu: &mut impl SceneGpu,
    frame: &mut Frame,
) {
    let Some(texture) = gpu.texture(&sprite.texture, sprite.filter) else {
        return;
    };
    let node = SpriteNode::new(texture, sprite.size)
        .with_region(sprite.region)
        .with_anchor(sprite.anchor)
        .with_color(sprite.color)
        .with_layer(sprite.layer);
    frame.draw_sprite(node.to_sprite(GlobalTransform2D::from_affine(global)));
}

fn draw_mesh(node: &NodeOutline, global: Mat4, gpu: &mut impl SceneGpu, frame: &mut Frame) {
    let Some(outline) = &node.mesh else {
        return;
    };
    let Ok((unit, scale)) = MeshFile::new(outline.shape).unit_shape() else {
        return;
    };
    let Some(mesh) = gpu.mesh(unit) else {
        return;
    };
    let material = Material {
        base_color: outline.color,
        base_color_texture: outline
            .texture
            .as_ref()
            .and_then(|(path, filter)| gpu.texture(path, *filter)),
        metallic: outline.metallic,
        roughness: outline.roughness,
    };
    frame.draw_mesh_3d(mesh, material, global * Mat4::from_scale(scale));
}

fn light_for_preview(frame: &mut Frame) {
    frame.set_ambient_light(AmbientLight::new(Color::WHITE, PREVIEW_AMBIENT_INTENSITY));
    frame.add_light(DirectionalLight {
        direction: PREVIEW_SUN_DIRECTION,
        color: Color::WHITE,
        intensity: PREVIEW_SUN_INTENSITY,
    });
}

/// Marks `node`: the box of what it shows, or a cross where it is.
fn mark(node: &NodeOutline, place: &Place, marks: Marks, frame: &mut Frame) {
    if let Some(global) = place.solid {
        let shape = node
            .mesh
            .as_ref()
            .and_then(|mesh| MeshFile::new(mesh.shape).unit_shape().ok());
        match shape {
            Some((unit, scale)) => {
                let scale = match unit {
                    MeshShape::Plane { .. } => Vec3::new(scale.x, PLANE_MARK_THICKNESS, scale.z),
                    MeshShape::Cube { .. } | MeshShape::Sphere { .. } => scale,
                };
                frame.draw(marks.cube, global * Mat4::from_scale(scale));
            }
            None => frame.draw(
                marks.cross,
                Mat4::from_translation(global.translation())
                    * Mat4::from_scale(Vec3::ONE * POINT_MARK_SIZE),
            ),
        }
    }
    if let Some(global) = place.flat {
        let global = GlobalTransform2D::from_affine(global);
        if let Some(sprite) = &node.sprite {
            frame.draw(marks.square, sprite_mark(sprite, global));
        } else {
            let position = global.position();
            frame.draw(
                marks.cross,
                Mat4::from_translation(Vec3::new(position.x, position.y, 0.0))
                    * Mat4::from_scale(Vec3::ONE * POINT_MARK_SIZE),
            );
        }
    }
}

/// Where the square mark goes around a sprite placed at `global`: over
/// its rectangle, a margin bigger, turned and placed as the sprite is.
fn sprite_mark(sprite: &SpriteOutline, global: GlobalTransform2D) -> Mat4 {
    let scale = global.scale();
    let size = Vec2::new(sprite.size.x * scale.x, sprite.size.y * scale.y);
    let corner = Vec3::new(
        -sprite.anchor.x * size.x - SPRITE_MARK_MARGIN,
        -sprite.anchor.y * size.y - SPRITE_MARK_MARGIN,
        0.0,
    );
    let marked = Vec3::new(
        size.x + 2.0 * SPRITE_MARK_MARGIN,
        size.y + 2.0 * SPRITE_MARK_MARGIN,
        1.0,
    );
    let position = global.position();
    Mat4::from_translation(Vec3::new(position.x, position.y, 0.0))
        * Mat4::from_rotation_z(global.rotation())
        * Mat4::from_translation(corner)
        * Mat4::from_scale(marked)
}

/// The color of the marks: bright, unlike the grid and most sprites.
const MARK_COLOR: Color = Color::rgb(1.0, 0.65, 0.1);

/// The segments of [`Marks::square`].
pub(crate) fn square_lines() -> Vec<Vertex> {
    let corners =
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| Vec3::new(x, y, 0.0));
    closed_loop(&corners)
}

/// The segments of [`Marks::cube`].
pub(crate) fn cube_lines() -> Vec<Vertex> {
    let half = 0.5;
    let face = |y: f32| {
        [(-half, -half), (half, -half), (half, half), (-half, half)]
            .map(|(x, z)| Vec3::new(x, y, z))
    };
    let (bottom, top) = (face(-half), face(half));
    let mut lines = closed_loop(&bottom);
    lines.extend(closed_loop(&top));
    for (low, high) in bottom.iter().zip(&top) {
        lines.extend([
            Vertex::new(*low, MARK_COLOR),
            Vertex::new(*high, MARK_COLOR),
        ]);
    }
    lines
}

/// The segments of [`Marks::cross`].
pub(crate) fn cross_lines() -> Vec<Vertex> {
    let half = 0.5;
    [
        Vec3::new(half, 0.0, 0.0),
        Vec3::new(0.0, half, 0.0),
        Vec3::new(0.0, 0.0, half),
    ]
    .into_iter()
    .flat_map(|end| {
        [
            Vertex::new(end * -1.0, MARK_COLOR),
            Vertex::new(end, MARK_COLOR),
        ]
    })
    .collect()
}

/// Segments joining `corners` in order, and the last to the first.
fn closed_loop(corners: &[Vec3]) -> Vec<Vertex> {
    corners
        .iter()
        .zip(corners.iter().cycle().skip(1))
        .flat_map(|(from, to)| [Vertex::new(*from, MARK_COLOR), Vertex::new(*to, MARK_COLOR)])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-5;

    /// Gives every texture and mesh asked for, counting them, unless the
    /// image is named `missing.png`.
    #[derive(Default)]
    struct FakeGpu {
        textures: usize,
        meshes: Vec<MeshShape>,
    }

    impl SceneGpu for FakeGpu {
        fn texture(&mut self, path: &AssetPath, _filter: TextureFilter) -> Option<TextureId> {
            (path.as_str() != "missing.png").then(|| {
                self.textures += 1;
                TextureId::from_index(self.textures)
            })
        }

        fn mesh(&mut self, unit: MeshShape) -> Option<Mesh3dId> {
            self.meshes.push(unit);
            Some(Mesh3dId::from_index(self.meshes.len()))
        }
    }

    const MARKS: Marks = Marks {
        square: MeshId::from_index(0),
        cube: MeshId::from_index(1),
        cross: MeshId::from_index(2),
    };

    fn outline(text: &str) -> SceneOutline {
        SceneOutline::from_text(&format!("format = 1\n{text}")).unwrap()
    }

    fn draw(outline: &SceneOutline, selected: Option<usize>) -> (Frame, FakeGpu) {
        let mut frame = Frame::new();
        let mut gpu = FakeGpu::default();
        draw_scene(outline, selected, &mut gpu, MARKS, &mut frame);
        (frame, gpu)
    }

    const SOLAR: &str = "[[node]]\nname = \"Sun\"\nposition = [1.0, 0.0]\nscale = [2.0, 2.0]\n\
        [[node]]\nname = \"Planet\"\nparent = 0\nposition = [3.0, 0.0]\n\
        [node.sprite]\ntexture = \"disc.png\"\nsize = [1.0, 1.0]\n\
        [[node]]\nname = \"Lost\"\nparent = 0\nposition = [0.0, 0.0]\n\
        [node.sprite]\ntexture = \"missing.png\"\nsize = [1.0, 1.0]\n";

    #[test]
    fn sprites_are_drawn_where_their_parents_place_them() {
        let (frame, _) = draw(&outline(SOLAR), None);

        let [planet] = frame.sprites() else {
            panic!("only the sprite whose image loads is drawn");
        };
        assert!((planet.position - Vec2::new(7.0, 0.0)).length() < EPSILON);
        assert!((planet.size - Vec2::new(2.0, 2.0)).length() < EPSILON);
    }

    #[test]
    fn nodes_without_a_transform_pass_their_parents_place_on() {
        let scene = outline(
            "[[node]]\nname = \"Root\"\nposition = [2.0, 0.0]\n\
             [[node]]\nname = \"Group\"\nparent = 0\n\
             [[node]]\nname = \"Leaf\"\nparent = 1\nposition = [1.0, 0.0]\n",
        );

        let places = places(scene.nodes());

        assert_eq!(places[1].flat, None);
        let leaf = GlobalTransform2D::from_affine(places[2].flat.unwrap());
        assert!((leaf.position() - Vec2::new(3.0, 0.0)).length() < EPSILON);
    }

    #[test]
    fn meshes_are_drawn_from_unit_shapes_and_light_comes_from_the_scene() {
        let mesh = "[[node]]\nname = \"Box\"\n[node.transform_3d]\n\
                    [node.mesh]\nshape = \"cube\"\nsize = [2.0, 1.0, 1.0]\n";
        let lamp = "[[node]]\nname = \"Lamp\"\nparent = 0\n[node.transform_3d]\n\
                    [node.light]\nkind = \"point\"\nrange = 5.0\n";

        let (dark, gpu) = draw(&outline(mesh), None);
        let (lit, _) = draw(&outline(&format!("{mesh}{lamp}")), None);

        assert_eq!(dark.mesh_3d_count(), 1);
        assert_eq!(gpu.meshes, [MeshShape::Cube { size: Vec3::ONE }]);
        assert_eq!(dark.lights().len(), 1, "a preview sun lights it");
        assert!(dark.ambient_light().intensity > 0.0);
        assert_eq!(lit.lights().len(), 1, "only the scene's lamp");
        assert!(matches!(
            lit.lights()[0],
            star_engine::prelude::Light::Point(_)
        ));
    }

    #[test]
    fn only_the_node_selected_is_marked() {
        let scene = outline(SOLAR);

        let (unmarked, _) = draw(&scene, None);
        let (sprite, _) = draw(&scene, Some(1));
        let (point, _) = draw(&scene, Some(0));

        assert_eq!(unmarked.draw_count(), 0);
        assert_eq!(sprite.draw_count(), 1);
        assert_eq!(point.draw_count(), 1);
    }

    #[test]
    fn mark_meshes_are_segments() {
        for lines in [square_lines(), cube_lines(), cross_lines()] {
            assert!(!lines.is_empty() && lines.len() % 2 == 0);
        }
        assert_eq!(cube_lines().len(), 2 * 12);
    }
}
