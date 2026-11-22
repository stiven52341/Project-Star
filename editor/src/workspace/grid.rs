//! Reference grid drawn on the ground of the scene viewport.

use star_engine::prelude::{Color, Vec3, Vertex};

use crate::project::ProjectDimension;

/// Grid lines extend this many units from the origin in each direction.
const GRID_HALF_EXTENT: i16 = 50;
const GRID_COLOR: Color = Color::rgb(0.24, 0.25, 0.29);
const X_AXIS_COLOR: Color = Color::rgb(0.75, 0.27, 0.27);
const Y_AXIS_COLOR: Color = Color::rgb(0.33, 0.68, 0.36);
const Z_AXIS_COLOR: Color = Color::rgb(0.30, 0.45, 0.85);

/// Line segments of a unit grid on the ground plane of `dimension` (XY in 2D,
/// XZ in 3D), with the two axes of that plane highlighted.
pub(crate) fn grid_lines(dimension: ProjectDimension) -> Vec<Vertex> {
    let (second_axis, second_axis_color) = match dimension {
        ProjectDimension::TwoD => (Vec3::new(0.0, 1.0, 0.0), Y_AXIS_COLOR),
        ProjectDimension::ThreeD => (Vec3::new(0.0, 0.0, 1.0), Z_AXIS_COLOR),
    };
    let first_axis = Vec3::new(1.0, 0.0, 0.0);
    let extent = f32::from(GRID_HALF_EXTENT);
    let mut vertices = Vec::new();
    for step in -GRID_HALF_EXTENT..=GRID_HALF_EXTENT {
        let offset = f32::from(step);
        // Lines at the origin are drawn by the axes, in color.
        let (along_first, along_second) = if step == 0 {
            (second_axis_color, X_AXIS_COLOR)
        } else {
            (GRID_COLOR, GRID_COLOR)
        };
        push_line(
            &mut vertices,
            first_axis * offset,
            second_axis,
            extent,
            along_first,
        );
        push_line(
            &mut vertices,
            second_axis * offset,
            first_axis,
            extent,
            along_second,
        );
    }
    vertices
}

/// Segment through `center`, along `direction`, `extent` units to each side.
fn push_line(vertices: &mut Vec<Vertex>, center: Vec3, direction: Vec3, extent: f32, color: Color) {
    vertices.push(Vertex::new(center - direction * extent, color));
    vertices.push(Vertex::new(center + direction * extent, color));
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINES_PER_DIRECTION: usize = 2 * GRID_HALF_EXTENT as usize + 1;

    #[test]
    fn grid_has_one_line_per_unit_in_each_direction() {
        let vertices = grid_lines(ProjectDimension::TwoD);

        assert_eq!(vertices.len(), 2 * 2 * LINES_PER_DIRECTION);
    }

    #[test]
    fn grid_3d_lies_on_the_ground() {
        let vertices = grid_lines(ProjectDimension::ThreeD);

        assert!(vertices.iter().all(|vertex| vertex.position.y == 0.0));
    }

    #[test]
    fn grid_2d_lies_on_the_screen_plane() {
        let vertices = grid_lines(ProjectDimension::TwoD);

        assert!(vertices.iter().all(|vertex| vertex.position.z == 0.0));
    }

    #[test]
    fn axes_are_highlighted() {
        let vertices = grid_lines(ProjectDimension::ThreeD);

        assert!(vertices.iter().any(|vertex| vertex.color == X_AXIS_COLOR));
        assert!(vertices.iter().any(|vertex| vertex.color == Z_AXIS_COLOR));
    }
}
