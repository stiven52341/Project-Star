//! Whether a project is a 2D or a 3D game.

/// Main kind of rendering a project is set up for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ProjectDimension {
    /// Sprites and tilemaps seen through an orthographic camera.
    #[default]
    TwoD,
    /// Meshes and lights seen through a perspective camera.
    ThreeD,
}

impl ProjectDimension {
    /// Both dimensions, in the order they are offered to the user.
    pub(crate) const ALL: [Self; 2] = [Self::TwoD, Self::ThreeD];

    /// Short name shown to the user.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::TwoD => "2D",
            Self::ThreeD => "3D",
        }
    }

    /// Dimension whose [`id`](Self::id) is `id`.
    pub(crate) fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|dimension| dimension.id() == id)
    }

    /// Stable identifier stored in the project file.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::TwoD => "2d",
            Self::ThreeD => "3d",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_round_trip() {
        for dimension in ProjectDimension::ALL {
            assert_eq!(ProjectDimension::from_id(dimension.id()), Some(dimension));
        }
        assert_eq!(ProjectDimension::from_id("4d"), None);
    }

    #[test]
    fn identifiers_are_lowercase_labels() {
        for dimension in ProjectDimension::ALL {
            assert_eq!(dimension.id(), dimension.label().to_lowercase());
        }
    }
}
