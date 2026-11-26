//! Drawing an open scene in the view: where each node ends up, what it
//! shows, and a mark on the node selected.

use star_engine::MeshFile;
use star_engine::prelude::{
    Affine2, AmbientLight, Camera, Color, DirectionalLight, Frame, GlobalTransform2D,
    GlobalTransform3D, MAX_LIGHTS, Mat4, Material, Mesh3dId, MeshId, MeshShape, SceneOutline,
    SpriteNode, TextureFilter, TextureId, Vec2, Vec3, Vertex,
};
use star_engine::star_assets::AssetPath;
use star_engine::star_platform::WindowConfig;
use star_engine::star_scene::{
    Camera2dNode, Camera3dNode, Light2dNode, LightNode, NodeOutline, SpriteOutline,
};

use super::body_drawing::{BodyMarks, draw_body_2d, draw_body_3d};

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
/// How far from a 3D camera the base of the pyramid it sees is drawn, in
/// world units.
const CAMERA_PYRAMID_DEPTH: f32 = 1.0;

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
    /// A square from `(-0.5, -0.5)` to `(0.5, 0.5)`, on the XY plane, in
    /// the color of cameras: what a 2D camera shows.
    pub(crate) view: MeshId,
    /// A pyramid from `(0, 0, 0)` to a base from `(-1, -1, -1)` to
    /// `(1, 1, -1)`, in the color of cameras: what a 3D camera sees.
    pub(crate) pyramid: MeshId,
    /// A cone from `(0, 0, 0)` to a circle of radius 1 at `z = -1`, in the
    /// color of lights: what a spot lights.
    pub(crate) cone: MeshId,
    /// A circle of radius 1 around the origin, on the XY plane, in the
    /// color of lights: how far a 2D light reaches.
    pub(crate) reach: MeshId,
    /// A segment from `(0, 0)` to `(1, 0)`, in the color of lights: an
    /// edge of the cone of a 2D spot.
    pub(crate) ray: MeshId,
    /// The meshes of the shapes of bodies.
    pub(crate) bodies: BodyMarks,
}

/// Adds to `frame` what the nodes of `outline` show, where the game would
/// show them, what their cameras see and their spots light, the shapes of
/// their bodies, and the mark of the node `selected`.
pub(crate) fn draw_scene(
    outline: &SceneOutline,
    selected: Option<usize>,
    gpu: &mut impl SceneGpu,
    marks: Marks,
    frame: &mut Frame,
) {
    let nodes = outline.nodes();
    let places = places(nodes);
    if !draw_shown(nodes, &places, gpu, frame) {
        light_for_preview(frame);
    }
    for (node, place) in nodes.iter().zip(&places) {
        if let (Some(body), Some(global)) = (&node.body, place.flat) {
            draw_body_2d(body, global, &marks.bodies, frame);
        }
        if let (Some(body), Some(global)) = (&node.body_3d, place.solid) {
            draw_body_3d(body, global, &marks.bodies, frame);
        }
        // Not lit by them: the 2D ambient light is set by the game's code,
        // which the editor cannot know, so only their reach is drawn.
        if let (Some(light), Some(global)) = (node.light_2d, place.flat) {
            draw_light_2d_reach(light, global, marks, frame);
        }
        if let (Some(camera), Some(global)) = (node.camera, place.flat) {
            draw_camera_2d(camera, global, marks, frame);
        }
        if let (Some(camera), Some(global)) = (node.camera_3d, place.solid) {
            draw_camera_3d(camera, global, marks, frame);
        }
        if let (Some(LightNode::Spot { range, angle, .. }), Some(global)) =
            (node.light, place.solid)
        {
            draw_spot_cone(range, angle, global, marks, frame);
        }
    }
    let selected = selected.and_then(|index| Some((nodes.get(index)?, places.get(index)?)));
    if let Some((node, place)) = selected {
        mark(node, place, marks, frame);
    }
}

/// Adds to `frame` what the game shows of the nodes of `outline`, as it
/// shows them before its code changes anything: its sprites, meshes and
/// lights, on a black background, with no ambient light in 3D and a white
/// one in 2D.
pub(crate) fn draw_game_view(outline: &SceneOutline, gpu: &mut impl SceneGpu, frame: &mut Frame) {
    let nodes = outline.nodes();
    let places = places(nodes);
    frame.set_clear_color(Color::BLACK);
    frame.set_ambient_light(AmbientLight::new(Color::BLACK, 0.0));
    frame.set_ambient_light_2d(AmbientLight::FULL);
    draw_shown(nodes, &places, gpu, frame);
    for (node, place) in nodes.iter().zip(&places).filter(|(_, place)| place.shown) {
        if let (Some(light), Some(global)) = (node.light_2d, place.flat)
            && frame.lights_2d().len() < MAX_LIGHTS
        {
            frame.add_light_2d(light.to_light(GlobalTransform2D::from_affine(global)));
        }
    }
}

/// Adds to `frame` the sprites and meshes of `nodes` that are shown,
/// placed at `places`, and the lights of their 3D nodes. Returns whether any light was added.
fn draw_shown(
    nodes: &[NodeOutline],
    places: &[Place],
    gpu: &mut impl SceneGpu,
    frame: &mut Frame,
) -> bool {
    let mut lit = false;
    for (node, place) in nodes.iter().zip(places).filter(|(_, place)| place.shown) {
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
    lit
}

/// The camera to preview in `outline`: the node `selected` if it is a
/// camera, or else the scene's main camera. With the node's position.
pub(crate) fn preview_camera(
    outline: &SceneOutline,
    selected: Option<usize>,
) -> Option<(usize, Camera)> {
    let nodes = outline.nodes();
    let is_camera = |index: &usize| {
        nodes
            .get(*index)
            .is_some_and(|node| node.camera.is_some() || node.camera_3d.is_some())
    };
    let index = selected
        .filter(is_camera)
        .or_else(|| outline.main_camera())?;
    let node = nodes.get(index)?;
    let place = places(nodes).get(index).copied()?;
    let camera = match (node.camera, place.flat, node.camera_3d, place.solid) {
        (Some(camera), Some(global), _, _) => Camera::centered_2d(
            GlobalTransform2D::from_affine(global).position(),
            camera.view_height,
        ),
        (_, _, Some(camera), Some(global)) => Camera::along_transform(global, camera.fov),
        _ => return None,
    };
    Some((index, camera))
}

/// Where a node ends up in the world, in 2D and in 3D: only in the spaces
/// it has a transform for, as the engine places nodes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Place {
    pub(super) flat: Option<Affine2>,
    pub(super) solid: Option<Mat4>,
    /// Whether the game draws it: neither it nor its ancestors are hidden.
    pub(super) shown: bool,
}

/// The place of every node.
pub(super) fn places(nodes: &[NodeOutline]) -> Vec<Place> {
    let handed = handed_down(nodes);
    let mut places: Vec<Place> = Vec::with_capacity(nodes.len());
    for node in nodes {
        let (parent_flat, parent_solid) = parent_place(node, &handed);
        // Parents come first in a scene, so theirs is already known.
        let parent_shown = node
            .parent
            .and_then(|parent| places.get(parent))
            .is_none_or(|parent| parent.shown);
        places.push(Place {
            flat: node
                .transform
                .map(|transform| parent_flat * transform.to_affine()),
            solid: node
                .transform_3d
                .map(|transform| parent_solid * transform.to_mat4()),
            shown: parent_shown && node.visible,
        });
    }
    places
}

/// Where every node places its children, in 2D and in 3D. Parents come
/// first in a scene, so one pass finds them all; a node without a
/// transform passes its parent's on.
pub(super) fn handed_down(nodes: &[NodeOutline]) -> Vec<(Affine2, Mat4)> {
    let mut handed: Vec<(Affine2, Mat4)> = Vec::with_capacity(nodes.len());
    for node in nodes {
        let (parent_flat, parent_solid) = parent_place(node, &handed);
        handed.push((
            node.transform
                .map_or(parent_flat, |transform| parent_flat * transform.to_affine()),
            node.transform_3d
                .map_or(parent_solid, |transform| parent_solid * transform.to_mat4()),
        ));
    }
    handed
}

/// Where the parent of `node` places it, from what [`handed_down`] found.
pub(super) fn parent_place(node: &NodeOutline, handed: &[(Affine2, Mat4)]) -> (Affine2, Mat4) {
    node.parent
        .and_then(|parent| handed.get(parent).copied())
        .unwrap_or((Affine2::IDENTITY, Mat4::IDENTITY))
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
        .with_layer(sprite.layer)
        .with_sheet(sprite.sheet);
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

/// The shape of the game's window, width over height: the one it opens
/// with, since the editor cannot know how players will size it.
pub(crate) fn game_aspect_ratio() -> f32 {
    WindowConfig::default().size.aspect_ratio().unwrap_or(1.0)
}

/// The rectangle a 2D camera placed at `global` shows: centered on it,
/// neither turned nor scaled, as the game shows it.
fn draw_camera_2d(camera: Camera2dNode, global: Affine2, marks: Marks, frame: &mut Frame) {
    let position = GlobalTransform2D::from_affine(global).position();
    let size = Vec3::new(
        camera.view_height * game_aspect_ratio(),
        camera.view_height,
        1.0,
    );
    frame.draw(
        marks.view,
        Mat4::from_translation(Vec3::new(position.x, position.y, 0.0)) * Mat4::from_scale(size),
    );
}

/// The pyramid a 3D camera placed at `global` sees, a little way along its
/// -z: turned as it is, but not scaled, as the game sees it.
fn draw_camera_3d(camera: Camera3dNode, global: Mat4, marks: Marks, frame: &mut Frame) {
    let half_height = (camera.fov / 2.0).tan() * CAMERA_PYRAMID_DEPTH;
    let size = Vec3::new(
        half_height * game_aspect_ratio(),
        half_height,
        CAMERA_PYRAMID_DEPTH,
    );
    frame.draw(
        marks.pyramid,
        without_scale(global) * Mat4::from_scale(size),
    );
}

/// How far a 2D light placed at `global` reaches, and for spots the edges
/// of their cone, along the node's +x axis.
fn draw_light_2d_reach(light: Light2dNode, global: Affine2, marks: Marks, frame: &mut Frame) {
    let global = GlobalTransform2D::from_affine(global);
    let position = global.position();
    let (Light2dNode::Point { range, .. } | Light2dNode::Spot { range, .. }) = light;
    let place = Mat4::from_translation(Vec3::new(position.x, position.y, 0.0));
    let size = Mat4::from_scale(Vec3::new(range, range, 1.0));
    frame.draw(marks.reach, place * size);
    if let Light2dNode::Spot { angle, .. } = light {
        for edge in [angle, -angle] {
            frame.draw(
                marks.ray,
                place * Mat4::from_rotation_z(global.rotation() + edge) * size,
            );
        }
    }
}

/// The cone a spot of `range` and `angle` placed at `global` lights, along
/// its -z: turned as it is, but not scaled, as the game lights it.
fn draw_spot_cone(range: f32, angle: f32, global: Mat4, marks: Marks, frame: &mut Frame) {
    let radius = angle.tan() * range;
    frame.draw(
        marks.cone,
        without_scale(global) * Mat4::from_scale(Vec3::new(radius, radius, range)),
    );
}

/// `matrix` with its axes made one unit long: placed and turned as it is,
/// but not scaled.
pub(super) fn without_scale(matrix: Mat4) -> Mat4 {
    let mut columns = matrix.to_columns();
    for column in columns.iter_mut().take(3) {
        let [x, y, z, _] = *column;
        if let Some(Vec3 { x, y, z }) = Vec3::new(x, y, z).normalized() {
            *column = [x, y, z, 0.0];
        }
    }
    Mat4::from_columns(columns)
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

/// The color of what cameras see: unlike the marks, since a camera can be
/// selected too.
const CAMERA_COLOR: Color = Color::rgb(0.35, 0.8, 0.85);

/// The segments of [`Marks::view`].
pub(crate) fn view_lines() -> Vec<Vertex> {
    let half = 0.5;
    let corners = [(-half, -half), (half, -half), (half, half), (-half, half)]
        .map(|(x, y)| Vec3::new(x, y, 0.0));
    colored_loop(&corners, CAMERA_COLOR)
}

/// The segments of [`Marks::pyramid`].
pub(crate) fn pyramid_lines() -> Vec<Vertex> {
    let base =
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| Vec3::new(x, y, -1.0));
    let mut lines = colored_loop(&base, CAMERA_COLOR);
    for corner in base {
        lines.extend([
            Vertex::new(Vec3::ZERO, CAMERA_COLOR),
            Vertex::new(corner, CAMERA_COLOR),
        ]);
    }
    lines
}

/// The color of what spots light, like the light icon of the scene's tree.
const LIGHT_COLOR: Color = Color::rgb(0.94, 0.78, 0.31);
/// How many segments make the circle at the end of a spot's cone.
const CONE_SEGMENTS: usize = 32;

/// The segments of [`Marks::cone`].
pub(crate) fn cone_lines() -> Vec<Vertex> {
    #[allow(clippy::cast_precision_loss)]
    let rim = |step: usize| {
        let turn = std::f32::consts::TAU * step as f32 / CONE_SEGMENTS as f32;
        Vec3::new(turn.cos(), turn.sin(), -1.0)
    };
    let corners: Vec<Vec3> = (0..CONE_SEGMENTS).map(rim).collect();
    let mut lines = colored_loop(&corners, LIGHT_COLOR);
    for side in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        lines.extend([
            Vertex::new(Vec3::ZERO, LIGHT_COLOR),
            Vertex::new(Vec3::new(side.0, side.1, -1.0), LIGHT_COLOR),
        ]);
    }
    lines
}

/// The segments of [`Marks::reach`].
pub(crate) fn reach_lines() -> Vec<Vertex> {
    #[allow(clippy::cast_precision_loss)]
    let corners: Vec<Vec3> = (0..CONE_SEGMENTS)
        .map(|step| {
            let turn = std::f32::consts::TAU * step as f32 / CONE_SEGMENTS as f32;
            Vec3::new(turn.cos(), turn.sin(), 0.0)
        })
        .collect();
    colored_loop(&corners, LIGHT_COLOR)
}

/// The segments of [`Marks::ray`].
pub(crate) fn ray_lines() -> Vec<Vertex> {
    vec![
        Vertex::new(Vec3::ZERO, LIGHT_COLOR),
        Vertex::new(Vec3::new(1.0, 0.0, 0.0), LIGHT_COLOR),
    ]
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
    colored_loop(corners, MARK_COLOR)
}

/// [`closed_loop`] in `color`.
fn colored_loop(corners: &[Vec3], color: Color) -> Vec<Vertex> {
    corners
        .iter()
        .zip(corners.iter().cycle().skip(1))
        .flat_map(|(from, to)| [Vertex::new(*from, color), Vertex::new(*to, color)])
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
        view: MeshId::from_index(3),
        pyramid: MeshId::from_index(4),
        cone: MeshId::from_index(5),
        reach: MeshId::from_index(6),
        ray: MeshId::from_index(7),
        bodies: super::super::body_drawing::TEST_MARKS,
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
    fn hidden_nodes_and_their_children_are_not_drawn() {
        let scene = outline(
            "[[node]]\nname = \"Root\"\nposition = [0.0, 0.0]\n\
             [[node]]\nname = \"Ghost\"\nparent = 0\nposition = [0.0, 0.0]\nvisible = false\n\
             [node.sprite]\ntexture = \"ghost.png\"\nsize = [1.0, 1.0]\n\
             [[node]]\nname = \"Chain\"\nparent = 1\nposition = [0.0, 0.0]\n\
             [node.sprite]\ntexture = \"chain.png\"\nsize = [1.0, 1.0]\n\
             [[node]]\nname = \"Hero\"\nparent = 0\nposition = [0.0, 0.0]\n\
             [node.sprite]\ntexture = \"hero.png\"\nsize = [1.0, 1.0]\n",
        );

        let (frame, gpu) = draw(&scene, None);

        assert_eq!(frame.sprites().len(), 1);
        assert_eq!(gpu.textures, 1);
        let shown: Vec<bool> = places(scene.nodes())
            .iter()
            .map(|place| place.shown)
            .collect();
        assert_eq!(shown, [true, false, false, true]);
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
        for lines in [
            square_lines(),
            cube_lines(),
            cross_lines(),
            view_lines(),
            pyramid_lines(),
            cone_lines(),
            reach_lines(),
            ray_lines(),
        ] {
            assert!(!lines.is_empty() && lines.len() % 2 == 0);
        }
        assert_eq!(cube_lines().len(), 2 * 12);
        assert_eq!(pyramid_lines().len(), 2 * 8);
    }

    #[test]
    fn cameras_show_what_they_see_whether_selected_or_not() {
        let cameras = "[[node]]\nname = \"Level\"\nposition = [0.0, 0.0]\n\
                       [[node]]\nname = \"Eye\"\nparent = 0\nposition = [1.0, 0.0]\n[node.camera]\n\
                       [[node]]\nname = \"Eye3D\"\nparent = 0\n[node.transform_3d]\n[node.camera_3d]\n\
                       [[node]]\nname = \"Lost\"\nparent = 0\n[node.camera_3d]\n";

        let (frame, _) = draw(&outline(cameras), None);

        assert_eq!(
            frame.draw_count(),
            2,
            "a camera without its transform shows nothing"
        );
    }

    #[test]
    fn the_preview_sees_from_the_camera_selected_or_else_the_main_one() {
        let cameras = "[[node]]\nname = \"Level\"\nposition = [0.0, 0.0]\n\
                       [[node]]\nname = \"Eye\"\nparent = 0\nposition = [1.0, 2.0]\n\
                       [node.camera]\nview_height = 4.0\n\
                       [[node]]\nname = \"Main\"\nparent = 0\nposition = [5.0, 0.0]\n\
                       [node.camera]\nmain = true\n";
        let scene = outline(cameras);

        let (selected, eye) = preview_camera(&scene, Some(1)).unwrap();
        let (main, _) = preview_camera(&scene, Some(0)).unwrap();

        assert_eq!((selected, main), (1, 2));
        assert_eq!(eye.target, Vec3::new(1.0, 2.0, 0.0));
        assert_eq!(preview_camera(&outline(SOLAR), Some(1)), None);
    }

    #[test]
    fn the_game_view_shows_no_marks_and_lights_only_with_the_scene() {
        let scene = outline(&format!(
            "{SOLAR}[[node]]\nname = \"Torch\"\nparent = 0\nposition = [0.0, 0.0]\n\
             [node.light_2d]\nkind = \"point\"\nrange = 2.0\n\
             [[node]]\nname = \"Box\"\nparent = 0\n[node.transform_3d]\n\
             [node.mesh]\nshape = \"cube\"\nsize = [1.0, 1.0, 1.0]\n"
        ));
        let mut frame = Frame::new();

        draw_game_view(&scene, &mut FakeGpu::default(), &mut frame);

        assert_eq!(frame.draw_count(), 0);
        assert_eq!(frame.sprites().len(), 1);
        assert_eq!(frame.mesh_3d_count(), 1);
        assert_eq!(frame.lights_2d().len(), 1);
        assert!(frame.lights().is_empty(), "no preview sun");
        assert!(frame.ambient_light().intensity.abs() < EPSILON);
    }

    #[test]
    fn bodies_show_their_shapes_whether_selected_or_not() {
        let bodies = "[[node]]\nname = \"Floor\"\nposition = [0.0, 0.0]\n\
                      [node.body]\nkind = \"static\"\nshape = \"rectangle\"\nsize = [4.0, 1.0]\n\
                      [[node]]\nname = \"Pill\"\nparent = 0\n[node.transform_3d]\n\
                      [node.body_3d]\nshape = \"capsule\"\nheight = 2.0\nradius = 0.5\n";

        let (frame, _) = draw(&outline(bodies), None);

        assert_eq!(
            frame.draw_count(),
            1 + 3,
            "a rectangle, and a capsule's three pieces"
        );
    }

    #[test]
    fn lights_2d_show_their_reach_without_lighting_the_view() {
        let lights = "[[node]]\nname = \"Torch\"\nposition = [0.0, 0.0]\n\
                      [node.light_2d]\nkind = \"point\"\nrange = 3.0\n\
                      [[node]]\nname = \"Beam\"\nparent = 0\nposition = [1.0, 0.0]\n\
                      [node.light_2d]\nkind = \"spot\"\nrange = 3.0\n";

        let (frame, _) = draw(&outline(lights), None);

        assert_eq!(
            frame.draw_count(),
            1 + 3,
            "a circle, and a circle and two edges"
        );
        assert_eq!(frame.lights_2d(), []);
    }

    #[test]
    fn spots_show_the_cone_they_light() {
        let spot = "[[node]]\nname = \"Lamp\"\n[node.transform_3d]\n\
                    [node.light]\nkind = \"spot\"\nrange = 5.0\n";

        let (frame, _) = draw(&outline(spot), None);

        assert_eq!(frame.draw_count(), 1);
        assert!(matches!(
            frame.lights()[0],
            star_engine::prelude::Light::Spot(_)
        ));
    }

    #[test]
    fn cameras_are_drawn_turned_but_not_scaled() {
        let turned = Mat4::from_rotation_y(1.0);
        let scaled = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0))
            * turned
            * Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));

        let point = without_scale(scaled).transform_point(Vec3::new(0.0, 0.0, -1.0));

        let expected = Vec3::new(1.0, 2.0, 3.0) + turned.transform_point(Vec3::new(0.0, 0.0, -1.0));
        assert!((point - expected).length() < EPSILON);
    }
}
