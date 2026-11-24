//! Tests that draw on a real GPU, without a window, and read the pixels back.
//!
//! Ignored by default, since CI machines may have no GPU; run them with
//! `cargo test -p star_render -- --ignored`.

use std::f32::consts::FRAC_PI_2;

use star_math::{Mat4, Vec2, Vec3};

use crate::scene_renderer::SceneRenderer;
use crate::{
    AmbientLight, Camera, Color, DirectionalLight, Frame, Image, Material, MeshData, PointLight,
    Sprite, TextureFilter, TextureId,
};

/// Width and height of the target, in pixels.
const SIZE: u32 = 8;
/// Stores the values unchanged, so the expected bytes are easy to write.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The camera shows from -1 to 1 in both axes on the square target.
const VIEW_HEIGHT: f32 = 2.0;
const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

struct Gpu {
    scene: SceneRenderer,
    target: wgpu::Texture,
}

impl Gpu {
    fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("these tests need a GPU");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test target"),
            size: extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Self {
            scene: SceneRenderer::new(device, queue, FORMAT, SIZE, SIZE),
            target,
        }
    }

    /// A texture of `width` x `height` pixels with the given colors.
    fn texture(&mut self, width: u32, height: u32, pixels: &[[u8; 4]]) -> TextureId {
        let image = Image::from_rgba8(width, height, pixels.as_flattened().to_vec()).unwrap();
        self.scene
            .create_texture(&image, TextureFilter::Pixelated)
            .unwrap()
    }

    /// Draws `sprites` and returns the pixels of the target, row by row.
    fn draw(&mut self, sprites: &[Sprite]) -> Vec<[u8; 4]> {
        let mut frame = Frame::new();
        frame.set_camera(Camera::new_2d(VIEW_HEIGHT));
        for sprite in sprites {
            frame.draw_sprite(*sprite);
        }
        self.render(&frame)
    }

    /// Draws `frame` and returns the pixels of the target, row by row.
    fn render(&mut self, frame: &Frame) -> Vec<[u8; 4]> {
        let view = self
            .target
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.scene.render(&view, 1.0, frame).unwrap();
        self.read_target()
    }

    fn read_target(&self) -> Vec<[u8; 4]> {
        let device = self.scene.device();
        let padded_row = (SIZE * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test readback"),
            size: u64::from(padded_row * SIZE),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            self.target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: None,
                },
            },
            extent(),
        );
        self.scene.queue().submit([encoder.finish()]);
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, |result| result.unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let bytes = buffer.slice(..).get_mapped_range().unwrap();
        bytes
            .chunks(padded_row as usize)
            .flat_map(|row| row[..(SIZE * 4) as usize].chunks_exact(4))
            .map(|pixel| pixel.try_into().unwrap())
            .collect()
    }
}

const fn extent() -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    }
}

/// The pixel at `column`, `row` (from the top-left corner).
fn pixel(pixels: &[[u8; 4]], column: u32, row: u32) -> [u8; 4] {
    pixels[(row * SIZE + column) as usize]
}

/// A sprite that covers the whole target.
fn full_screen(texture: TextureId) -> Sprite {
    Sprite::new(texture, Vec2::ZERO, Vec2::new(VIEW_HEIGHT, VIEW_HEIGHT))
}

#[test]
#[ignore = "needs a GPU"]
fn sprites_are_tinted_by_their_color() {
    let mut gpu = Gpu::new();
    let white = gpu.texture(1, 1, &[[255; 4]]);

    let pixels = gpu.draw(&[full_screen(white).with_color(Color::rgb(1.0, 0.0, 0.0))]);

    assert!(pixels.iter().all(|&pixel| pixel == RED));
}

#[test]
#[ignore = "needs a GPU"]
fn targets_of_other_sizes_are_drawn_with_their_own_depth() {
    let mut gpu = Gpu::new();
    let white = gpu.texture(1, 1, &[[255; 4]]);
    let bigger = SIZE * 2;
    let device = gpu.scene.device().clone();
    let other = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("other target"),
            size: wgpu::Extent3d {
                width: bigger,
                height: bigger,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default());
    let depth = crate::depth::DepthBuffer::new(&device, bigger, bigger);
    let mut frame = Frame::new();
    frame.set_camera(Camera::new_2d(VIEW_HEIGHT));
    frame.draw_sprite(full_screen(white).with_color(Color::rgb(0.0, 0.0, 1.0)));

    gpu.scene
        .render_with_depth(&other, Some(depth.view()), 1.0, &frame)
        .unwrap();
    let pixels = gpu.draw(&[full_screen(white).with_color(Color::rgb(1.0, 0.0, 0.0))]);

    assert!(
        pixels.iter().all(|&pixel| pixel == RED),
        "each frame keeps its own sprites"
    );
}

/// A white full-screen sprite in the dark, lit by `light`: the red of its
/// left edge, center and right edge (the screen is two units wide).
fn lit_sprite(light: impl Into<crate::Light2d>) -> [u8; 3] {
    let mut gpu = Gpu::new();
    let white = gpu.texture(1, 1, &[[255; 4]]);
    let mut frame = Frame::new();
    frame.set_camera(Camera::new_2d(VIEW_HEIGHT));
    frame.set_ambient_light_2d(AmbientLight::NONE);
    frame.add_light_2d(light);
    frame.draw_sprite(full_screen(white));
    let pixels = gpu.render(&frame);
    let middle = SIZE / 2;
    [0, middle, SIZE - 1].map(|column| pixel(&pixels, column, middle)[0])
}

#[test]
#[ignore = "needs a GPU"]
fn sprites_show_their_own_colors_without_2d_lights() {
    let mut gpu = Gpu::new();
    let white = gpu.texture(1, 1, &[[255; 4]]);

    let pixels = gpu.draw(&[full_screen(white)]);

    assert!(pixels.iter().all(|&pixel| pixel == [255; 4]));
}

#[test]
#[ignore = "needs a GPU"]
fn point_lights_2d_fade_towards_their_range() {
    let [left, center, right] = lit_sprite(crate::PointLight2d {
        position: Vec2::ZERO,
        color: Color::WHITE,
        intensity: 1.0,
        range: 0.9,
    });

    assert!(center > 200, "{center}");
    assert_eq!((left, right), (0, 0));
}

#[test]
#[ignore = "needs a GPU"]
fn spots_2d_light_inside_their_cone_only() {
    let [left, _, right] = lit_sprite(crate::SpotLight2d {
        position: Vec2::ZERO,
        direction: Vec2::new(1.0, 0.0),
        color: Color::WHITE,
        intensity: 4.0,
        range: 10.0,
        angle: crate::DEFAULT_SPOT_ANGLE,
        blend: 0.0,
    });

    assert!(right > 0, "{right}");
    assert_eq!(left, 0);
}

#[test]
#[ignore = "needs a GPU"]
fn sprites_show_their_texture_the_right_way_up() {
    let mut gpu = Gpu::new();
    // Green on top, blue below.
    let texture = gpu.texture(1, 2, &[GREEN, BLUE]);

    let pixels = gpu.draw(&[full_screen(texture)]);

    assert_eq!(pixel(&pixels, 4, 0), GREEN);
    assert_eq!(pixel(&pixels, 4, SIZE - 1), BLUE);
}

#[test]
#[ignore = "needs a GPU"]
fn higher_layers_are_drawn_on_top() {
    let mut gpu = Gpu::new();
    let red = gpu.texture(1, 1, &[RED]);
    let blue = gpu.texture(1, 1, &[BLUE]);

    let pixels = gpu.draw(&[full_screen(red).with_layer(1), full_screen(blue)]);

    assert_eq!(pixel(&pixels, 4, 4), RED);
}

#[test]
#[ignore = "needs a GPU"]
fn sprites_only_cover_their_area() {
    let mut gpu = Gpu::new();
    let red = gpu.texture(1, 1, &[RED]);
    // The left half of the screen.
    let left_half = Sprite::new(red, Vec2::new(-0.5, 0.0), Vec2::new(1.0, 2.0));

    let pixels = gpu.draw(&[left_half]);

    assert_eq!(pixel(&pixels, 1, 4), RED);
    assert_eq!(pixel(&pixels, SIZE - 2, 4), [0, 0, 0, 255]);
}

#[test]
#[ignore = "needs a GPU"]
fn many_sprites_grow_the_buffers() {
    let mut gpu = Gpu::new();
    let red = gpu.texture(1, 1, &[RED]);
    let sprites = vec![full_screen(red); 1000];

    let pixels = gpu.draw(&sprites);

    assert_eq!(pixel(&pixels, 4, 4), RED);
}

#[test]
#[ignore = "needs a GPU"]
fn replaced_textures_show_their_new_image() {
    let mut gpu = Gpu::new();
    let texture = gpu.texture(1, 1, &[RED]);
    let blue_and_green = Image::from_rgba8(2, 1, [BLUE, GREEN].as_flattened().to_vec()).unwrap();

    gpu.scene
        .replace_texture(texture, &blue_and_green, TextureFilter::Pixelated)
        .unwrap();
    let pixels = gpu.draw(&[full_screen(texture)]);

    assert_eq!(pixel(&pixels, 0, 0), BLUE);
    assert_eq!(pixel(&pixels, SIZE - 1, 0), GREEN);
    assert!(
        gpu.scene
            .replace_texture(TextureId(7), &blue_and_green, TextureFilter::Smooth)
            .is_err()
    );
}

#[test]
#[ignore = "needs a GPU"]
fn unknown_textures_are_an_error() {
    let mut gpu = Gpu::new();
    let mut frame = Frame::new();
    frame.draw_sprite(full_screen(TextureId(7)));
    let view = gpu
        .target
        .create_view(&wgpu::TextureViewDescriptor::default());

    assert!(gpu.scene.render(&view, 1.0, &frame).is_err());
}

const BLACK: [u8; 4] = [0, 0, 0, 255];

/// A frame that looks at the origin from 5 units away along z, lit by an
/// ambient light that shows colors as they are.
fn frame_3d() -> Frame {
    let mut frame = Frame::new();
    frame.set_camera(Camera::new_3d(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO));
    frame.set_ambient_light(AmbientLight::new(Color::WHITE, 1.0));
    frame
}

/// A white square 2 units wide, facing up.
fn plane(gpu: &mut Gpu) -> crate::Mesh3dId {
    gpu.scene
        .create_mesh_3d(&MeshData::plane(Vec2::new(2.0, 2.0)).unwrap())
        .unwrap()
}

/// Turns a plane to face the camera of [`frame_3d`].
const FACE_THE_CAMERA: f32 = FRAC_PI_2;
const MATTE_WHITE: Material = Material::new(Color::WHITE).with_roughness(1.0);

#[test]
#[ignore = "needs a GPU"]
fn meshes_3d_show_their_material() {
    let mut gpu = Gpu::new();
    let cube = gpu
        .scene
        .create_mesh_3d(&MeshData::cube(Vec3::ONE).unwrap())
        .unwrap();
    let mut frame = frame_3d();
    frame.draw_mesh_3d(
        cube,
        Material::new(Color::rgb(1.0, 0.0, 0.0)),
        Mat4::from_rotation_y(0.5),
    );

    let pixels = gpu.render(&frame);

    assert_eq!(pixel(&pixels, SIZE / 2, SIZE / 2), RED);
    assert_eq!(pixel(&pixels, 0, 0), BLACK);
}

#[test]
#[ignore = "needs a GPU"]
fn the_back_of_3d_meshes_is_not_drawn() {
    let mut gpu = Gpu::new();
    let plane = plane(&mut gpu);
    let green = Material::new(Color::rgb(0.0, 1.0, 0.0));
    let mut center = |turn: f32| {
        let mut frame = frame_3d();
        frame.draw_mesh_3d(plane, green, Mat4::from_rotation_x(turn));
        pixel(&gpu.render(&frame), SIZE / 2, SIZE / 2)
    };

    // Turned to face the camera, then away from it.
    assert_eq!(center(FACE_THE_CAMERA), GREEN);
    assert_eq!(center(-FACE_THE_CAMERA), BLACK);
}

/// The red of the center of a matte white square facing the camera, lit
/// only by `light`.
fn lit_center(light: impl Into<crate::Light>) -> u8 {
    let mut gpu = Gpu::new();
    let plane = plane(&mut gpu);
    let mut frame = frame_3d();
    frame.set_ambient_light(AmbientLight::NONE);
    frame.add_light(light);
    frame.draw_mesh_3d(plane, MATTE_WHITE, Mat4::from_rotation_x(FACE_THE_CAMERA));
    pixel(&gpu.render(&frame), SIZE / 2, SIZE / 2)[0]
}

#[test]
#[ignore = "needs a GPU"]
fn surfaces_facing_a_light_are_lit_and_the_others_are_not() {
    let sun = |direction| DirectionalLight {
        direction,
        color: Color::WHITE,
        intensity: 1.0,
    };

    let facing = lit_center(sun(Vec3::new(0.0, 0.0, -1.0)));
    let behind = lit_center(sun(Vec3::new(0.0, 0.0, 1.0)));

    assert!(facing > 230, "{facing}");
    assert_eq!(behind, 0);
}

#[test]
#[ignore = "needs a GPU"]
fn point_lights_fade_with_distance_and_end_at_their_range() {
    let lamp = |distance: f32, range: f32| PointLight {
        position: Vec3::new(0.0, 0.0, distance),
        color: Color::WHITE,
        intensity: 1.0,
        range,
    };

    let near = lit_center(lamp(0.5, 10.0));
    let far = lit_center(lamp(2.0, 10.0));
    let out_of_range = lit_center(lamp(2.0, 1.0));

    assert!(near > far && far > 0, "{near} {far}");
    assert_eq!(out_of_range, 0);
}

#[test]
#[ignore = "needs a GPU"]
fn spots_light_inside_their_cone_only() {
    // Two units in front of the plane's center, pointing at it or away
    // to the side.
    let spot = |direction| crate::SpotLight {
        position: Vec3::new(0.0, 0.0, 2.0),
        direction,
        color: Color::WHITE,
        intensity: 4.0,
        range: 10.0,
        angle: crate::DEFAULT_SPOT_ANGLE,
        blend: crate::DEFAULT_SPOT_BLEND,
    };

    let at_it = lit_center(spot(Vec3::new(0.0, 0.0, -1.0)));
    let aside = lit_center(spot(Vec3::new(1.0, 0.0, -1.0)));

    assert!(at_it > 0, "{at_it}");
    assert_eq!(aside, 0);
}

#[test]
#[ignore = "needs a GPU"]
fn textures_tint_3d_surfaces() {
    let mut gpu = Gpu::new();
    let plane = plane(&mut gpu);
    let blue = gpu.texture(1, 1, &[BLUE]);
    let mut frame = frame_3d();

    frame.draw_mesh_3d(
        plane,
        Material::default().with_texture(blue),
        Mat4::from_rotation_x(FACE_THE_CAMERA),
    );

    assert_eq!(pixel(&gpu.render(&frame), SIZE / 2, SIZE / 2), BLUE);
}
