//! Models as parts of nodes: a 3D node that shows a glTF model whole, every
//! mesh and material of it (decision 83), and its `[node.model]` table.
//! The engine loads the file the first time it draws it.

use serde::{Deserialize, Serialize};
use star_assets::{AssetPath, ModelFormat};
use star_core::EngineResult;

/// Shows the model of a file at a 3D node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelNode {
    /// The `.glb` or `.gltf` file, in the asset folder.
    pub file: AssetPath,
}

/// A `[node.model]` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelData {
    file: String,
}

impl ModelData {
    pub(crate) fn to_model(&self) -> EngineResult<ModelNode> {
        let file = AssetPath::new(&self.file)?;
        ModelFormat::of(&file)?;
        Ok(ModelNode { file })
    }

    pub(crate) fn from_model(model: &ModelNode) -> Self {
        Self {
            file: model.file.as_str().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(text: &str) -> EngineResult<ModelNode> {
        toml::from_str::<ModelData>(text).unwrap().to_model()
    }

    #[test]
    fn models_name_a_gltf_file() {
        assert_eq!(
            model("file = \"chars/knight.glb\"").unwrap().file.as_str(),
            "chars/knight.glb"
        );
        assert!(model("file = \"knight.obj\"").is_err());
        assert!(model("file = \"../knight.glb\"").is_err());
    }
}
