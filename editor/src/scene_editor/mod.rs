//! The scene editor: a `.scene` file of the assets folder, open in the
//! “Scene” tab. The “Scene” panel shows its nodes as a tree, whose menus
//! create, duplicate and delete nodes, dragging a node moves it to another
//! parent, and the inspector edits the node selected in
//! it. Changes are kept until the scene is saved, and can be undone.

mod body_drawing;
mod class_choice;
mod class_drop;
mod components;
mod drawing;
mod dropping;
mod exported_fields;
mod fields;
mod gizmo;
mod history;
mod picking;
mod properties;
mod reparenting;
mod tree;
mod tree_view;

use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui;
use star_engine::prelude::{Camera, Frame, SceneOutline, Vec3};
use star_engine::star_scene::NodeOutline;
use star_script::semantic::ClassTable;

pub(crate) use self::body_drawing::BodyMarks;
use self::class_choice::{ChoiceOutcome, ClassChoice};
pub(crate) use self::class_drop::ClassDrop;
pub(crate) use self::components::LightKind;
use self::components::NodeTemplate;
pub(crate) use self::drawing::{
    Marks, SceneGpu, cone_lines, cross_lines, cube_lines, game_aspect_ratio, pyramid_lines,
    ray_lines, reach_lines, square_lines, view_lines,
};
pub(crate) use self::dropping::ImageDrop;
pub(crate) use self::gizmo::{
    Drag, Handle, HandleShape, Lens, Target, Tip, Tool, handle_at, shapes,
};
use self::history::{Now, SceneHistory, Snapshot};
pub(crate) use self::picking::Ray;
use self::properties::{NodeContext, edit_node};
use self::tree_view::{MenuContext, TreeAction, TreeState, TreeView};
use crate::project::ProjectDimension;
use crate::style::ERROR_COLOR;
use crate::texts;

/// A scene open in the editor.
pub(crate) struct SceneDocument {
    /// Where it is inside the assets folder: `levels/one.scene`.
    path: String,
    /// Its file on disk.
    file: PathBuf,
    /// The kind of project it belongs to, which new nodes and parts follow.
    dimension: ProjectDimension,
    /// Its nodes, or why its file could not be read.
    content: Result<SceneOutline, String>,
    /// The position of the node selected in the tree.
    selected: Option<usize>,
    /// What can be undone and redone, which also tells whether it changed
    /// since it was saved; `None` if its file could not be read.
    history: Option<SceneHistory>,
    /// Why the last save failed, until one works.
    save_error: Option<String>,
    /// The nodes closed in the tree and the text searched in it.
    tree: TreeState,
    /// The classes of a script dropped on a node, to choose from.
    class_choice: Option<ClassChoice>,
}

impl SceneDocument {
    /// Opens the scene at `path` inside the `assets` folder of a project of
    /// `dimension`.
    pub(crate) fn open(assets: &Path, path: String, dimension: ProjectDimension) -> Self {
        let file = assets.join(&path);
        let content = fs::read_to_string(&file)
            .map_err(|error| error.to_string())
            .and_then(|text| SceneOutline::from_text(&text).map_err(|error| error.to_string()));
        let history = content.as_ref().ok().map(|outline| {
            SceneHistory::new(Snapshot {
                outline: outline.clone(),
                selected: None,
            })
        });
        Self {
            path,
            file,
            dimension,
            content,
            selected: None,
            history,
            save_error: None,
            tree: TreeState::default(),
            class_choice: None,
        }
    }

    /// Where it is inside the assets folder.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// The name of its file, for its tab, marked if it has unsaved changes.
    pub(crate) fn title(&self) -> String {
        let name = self.path.rsplit('/').next().unwrap_or(&self.path);
        if self.is_unsaved() {
            texts::unsaved_title(name)
        } else {
            name.to_owned()
        }
    }

    /// Whether it changed since it was opened or saved.
    pub(crate) fn is_unsaved(&self) -> bool {
        self.history
            .as_ref()
            .is_some_and(|history| !history.is_saved())
    }

    /// Writes it to its file if it has unsaved changes. Returns whether it
    /// is saved now; if not, the “Scene” panel shows why.
    pub(crate) fn save(&mut self) -> bool {
        if !self.is_unsaved() {
            return true;
        }
        let (Ok(outline), Some(history)) = (&self.content, &mut self.history) else {
            return true;
        };
        let written = outline
            .to_text()
            .map_err(|error| error.to_string())
            .and_then(|text| fs::write(&self.file, text).map_err(|error| error.to_string()));
        match written {
            Ok(()) => {
                history.mark_saved(Now {
                    outline,
                    selected: self.selected,
                });
                self.save_error = None;
                true
            }
            Err(error) => {
                self.save_error = Some(texts::current().scene_not_saved(&error));
                false
            }
        }
    }

    /// Adds to `frame` what its nodes show and the mark of the one
    /// selected, with the textures and meshes of `gpu`.
    pub(crate) fn draw(&self, gpu: &mut impl SceneGpu, marks: Marks, frame: &mut Frame) {
        if let Ok(outline) = &self.content {
            drawing::draw_scene(outline, self.selected, gpu, marks, frame);
        }
    }

    /// The camera whose view is previewed, with its node's name: the node
    /// selected if it is a camera, or else the scene's main camera.
    pub(crate) fn preview_camera(&self) -> Option<(&str, Camera)> {
        let outline = self.content.as_ref().ok()?;
        let (index, camera) = drawing::preview_camera(outline, self.selected)?;
        Some((outline.nodes().get(index)?.name.as_str(), camera))
    }

    /// Adds to `frame` what the game shows of the scene, before its code
    /// changes anything.
    pub(crate) fn draw_game_view(&self, gpu: &mut impl SceneGpu, frame: &mut Frame) {
        if let Ok(outline) = &self.content {
            drawing::draw_game_view(outline, gpu, frame);
        }
    }

    /// Selects the node `ray` meets first, or none if it meets nothing, as
    /// when the user clicks the view.
    pub(crate) fn select_at(&mut self, ray: Ray) {
        if let Ok(outline) = &self.content {
            self.selected = picking::node_at(outline, ray);
            self.tree.reveal_selected();
        }
    }

    /// Adds a node showing `image` under the root, where `ray` meets the
    /// scene, and selects it.
    pub(crate) fn drop_image_at(&mut self, image: &ImageDrop, ray: Ray) {
        let world = dropping::drop_point(ray, self.dimension);
        self.add_image(image, ROOT, Some(world));
    }

    /// Adds a node showing `image` under `parent`, at its parent's place
    /// (or at `world`, if given), and selects it.
    fn add_image(&mut self, image: &ImageDrop, parent: usize, world: Option<Vec3>) {
        let Ok(outline) = &mut self.content else {
            return;
        };
        let handed = drawing::handed_down(outline.nodes());
        let Some(&place) = handed.get(parent) else {
            return;
        };
        let world = world.unwrap_or_else(|| match self.dimension {
            ProjectDimension::TwoD => {
                let origin = place.0.translation();
                Vec3::new(origin.x, origin.y, 0.0)
            }
            ProjectDimension::ThreeD => place.1.translation(),
        });
        let name = unique_child_name(outline, parent, image.node_name());
        let Some(node) = image.node(name, self.dimension, world, place) else {
            return;
        };
        match outline.add_node(parent, node) {
            Ok(index) => {
                self.selected = Some(index);
                self.mark_changed();
                self.tree.reveal_selected();
            }
            Err(error) => log::warn!("{error}"),
        }
    }

    /// The node selected, for the handles of the view to move: in 3D if
    /// the project is 3D and the node has a 3D transform, in 2D if not.
    /// `None` if no node is selected or it has no transform.
    pub(crate) fn gizmo_target(&self) -> Option<Target> {
        let nodes = self.content.as_ref().ok()?.nodes();
        let node = nodes.get(self.selected?)?;
        let handed = drawing::handed_down(nodes);
        let (parent_flat, parent_solid) = drawing::parent_place(node, &handed);
        let flat = node.transform.map(|local| Target::Flat {
            parent: parent_flat,
            local,
        });
        let solid = node.transform_3d.map(|local| Target::Solid {
            parent: parent_solid,
            local,
        });
        match self.dimension {
            ProjectDimension::TwoD => flat.or(solid),
            ProjectDimension::ThreeD => solid.or(flat),
        }
    }

    /// Gives the node selected the transform the handles of the view
    /// left it with.
    pub(crate) fn set_gizmo_target(&mut self, target: Target) {
        let Some(node) = self.content.as_mut().ok().and_then(|outline| {
            self.selected
                .and_then(|selected| outline.node_mut(selected))
        }) else {
            return;
        };
        let changed = match target {
            Target::Flat { local, .. } => node.transform.replace(local) != Some(local),
            Target::Solid { local, .. } => node.transform_3d.replace(local) != Some(local),
        };
        if changed {
            self.mark_changed();
        }
    }

    /// The tree of its nodes, where the user selects one, with buttons to
    /// add a node and delete the one selected.
    pub(crate) fn show_tree(&mut self, ui: &mut egui::Ui, images: &[String]) {
        if let Some(error) = &self.save_error {
            ui.colored_label(ERROR_COLOR, error);
        }
        let outline = match &self.content {
            Ok(outline) => outline,
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts::current().scene_unreadable(error));
                return;
            }
        };
        let menu = MenuContext {
            dimension: self.dimension,
            images,
        };
        let outcome =
            TreeView::new(outline.nodes(), &mut self.tree, &mut self.selected, menu).show(ui);
        if let Some(action) = outcome.action.or_else(|| self.action_from_keys(ui)) {
            self.change_tree(action, images);
        }
        if let Some((parent, image)) = outcome.dropped {
            self.add_image(&image, parent, None);
        }
        if let Some((node, script, at)) = outcome.class_dropped {
            self.drop_class(node, &script, at);
        }
    }

    /// The node `ray` meets first, if any.
    pub(crate) fn node_at(&self, ray: Ray) -> Option<usize> {
        picking::node_at(self.content.as_ref().ok()?, ray)
    }

    /// Whether a node is selected.
    pub(crate) const fn has_selection(&self) -> bool {
        self.selected.is_some()
    }

    /// Gives the node `ray` meets first the class of `script`, dropped on
    /// the view at `at`; nothing if it meets no node.
    pub(crate) fn drop_class_at(&mut self, script: &ClassDrop, ray: Ray, at: egui::Pos2) {
        if let Some(node) = self.node_at(ray) {
            self.drop_class(node, script, at);
        }
    }

    /// Gives the node selected the class of `script`, dropped on the
    /// inspector at `at`.
    pub(crate) fn drop_class_on_selected(&mut self, script: &ClassDrop, at: egui::Pos2) {
        if let Some(node) = self.selected {
            self.drop_class(node, script, at);
        }
    }

    /// Gives the node at `node` the only class of `script`, or opens a menu
    /// at `at` that asks which one if it has several.
    fn drop_class(&mut self, node: usize, script: &ClassDrop, at: egui::Pos2) {
        match script.classes.as_slice() {
            [] => {}
            [class] => self.change_tree(
                TreeAction::SetClass {
                    node,
                    class: class.clone(),
                },
                &[],
            ),
            classes => {
                self.class_choice = Some(ClassChoice {
                    node,
                    classes: classes.to_vec(),
                    at,
                });
            }
        }
    }

    /// The menu of the classes of a script dropped on a node, while it is
    /// open.
    pub(crate) fn show_class_choice(&mut self, ctx: &egui::Context) {
        let Some(choice) = &self.class_choice else {
            return;
        };
        match choice.show(ctx) {
            ChoiceOutcome::Open => {}
            ChoiceOutcome::Chosen(class) => {
                let node = choice.node;
                self.class_choice = None;
                self.change_tree(TreeAction::SetClass { node, class }, &[]);
            }
            ChoiceOutcome::Dismissed => self.class_choice = None,
        }
    }

    /// `Delete` deletes the node selected and `Ctrl+D` duplicates it,
    /// while the pointer is over the tree and no text is being typed.
    fn action_from_keys(&self, ui: &egui::Ui) -> Option<TreeAction> {
        if !ui.ui_contains_pointer() || ui.ctx().egui_wants_keyboard_input() {
            return None;
        }
        let selected = self.selected.filter(|&selected| selected != ROOT)?;
        ui.input(|input| {
            if input.key_pressed(egui::Key::Delete) {
                Some(TreeAction::Delete(selected))
            } else if input.modifiers.command && input.key_pressed(egui::Key::D) {
                Some(TreeAction::Duplicate(selected))
            } else {
                None
            }
        })
    }

    /// Does what the user asked of the tree, making images of `images`
    /// available to new sprites.
    fn change_tree(&mut self, action: TreeAction, images: &[String]) {
        let Ok(outline) = &mut self.content else {
            return;
        };
        let mut editor = TreeEditor {
            outline,
            selected: &mut self.selected,
            dimension: self.dimension,
        };
        let changed = match action {
            TreeAction::Create { parent, template } => editor.add_child(parent, template, images),
            TreeAction::Duplicate(index) => editor.duplicate(index),
            TreeAction::Delete(index) => editor.delete(index),
            TreeAction::Move { node, parent } => editor.reparent(node, parent),
            TreeAction::SetClass { node, class } => editor.set_class(node, class),
        };
        if changed {
            self.mark_changed();
            self.tree.reveal_selected();
        }
    }

    /// The node selected, to edit, choosing images among `images`. Returns
    /// whether one is selected, so the inspector can show something else
    /// when none is.
    pub(crate) fn edit_selected_node(
        &mut self,
        ui: &mut egui::Ui,
        images: &[String],
        classes: Option<&ClassTable>,
    ) -> bool {
        let node = self.content.as_mut().ok().and_then(|outline| {
            self.selected
                .and_then(|selected| outline.node_mut(selected))
        });
        let Some(node) = node else {
            return false;
        };
        let context = NodeContext {
            dimension: self.dimension,
            images,
            classes,
        };
        let was_main = node.is_main_camera();
        if edit_node(ui, node, &context) {
            if !was_main && node.is_main_camera() {
                self.make_only_main_camera();
            }
            self.mark_changed();
        }
        true
    }

    /// The selected node has just become the main camera: no other node
    /// is it any more, as a scene is seen from one camera.
    fn make_only_main_camera(&mut self) {
        let (Ok(outline), Some(selected)) = (&mut self.content, self.selected) else {
            return;
        };
        for index in 0..outline.nodes().len() {
            if index != selected
                && let Some(node) = outline.node_mut(index)
            {
                stop_being_main(node);
            }
        }
    }

    /// The scene changed: the change can be undone.
    fn mark_changed(&mut self) {
        if let Some(history) = &mut self.history {
            history.mark_changed();
        }
    }

    /// Closes the change being made once the user lets go of the pointer
    /// and of the text being typed, so a whole drag or a whole number is
    /// undone at once. Called once per frame, after every change.
    pub(crate) fn settle(&mut self, context: &egui::Context) {
        let (Ok(outline), Some(history)) = (&self.content, &mut self.history) else {
            return;
        };
        history.follow_selection(self.selected);
        let busy =
            context.input(|input| input.pointer.any_down()) || context.egui_wants_keyboard_input();
        if !busy {
            history.close_step(Now {
                outline,
                selected: self.selected,
            });
        }
    }

    /// Whether there is a change to undo.
    pub(crate) fn can_undo(&self) -> bool {
        self.history.as_ref().is_some_and(SceneHistory::can_undo)
    }

    /// Whether there is a change undone to redo.
    pub(crate) fn can_redo(&self) -> bool {
        self.history.as_ref().is_some_and(SceneHistory::can_redo)
    }

    /// Takes the scene back to how it was before the last change.
    pub(crate) fn undo(&mut self) {
        self.travel(SceneHistory::undo);
    }

    /// Makes again the last change undone.
    pub(crate) fn redo(&mut self) {
        self.travel(SceneHistory::redo);
    }

    /// Shows the scene `step` takes the history to, if it takes it anywhere.
    fn travel(&mut self, step: fn(&mut SceneHistory, Now<'_>) -> Option<Snapshot>) {
        let (Ok(outline), Some(history)) = (&mut self.content, &mut self.history) else {
            return;
        };
        let now = Now {
            outline,
            selected: self.selected,
        };
        let Some(snapshot) = step(history, now) else {
            return;
        };
        *outline = snapshot.outline;
        self.selected = snapshot
            .selected
            .filter(|&selected| selected < outline.nodes().len());
        self.tree.reveal_selected();
    }
}

/// Every scene has its root first.
const ROOT: usize = 0;

/// Makes the camera of `node`, if it is one, wait to be chosen.
const fn stop_being_main(node: &mut NodeOutline) {
    if let Some(camera) = &mut node.camera {
        camera.main = false;
    }
    if let Some(camera) = &mut node.camera_3d {
        camera.main = false;
    }
}

/// What changes the tree: adding, moving and deleting nodes.
struct TreeEditor<'scene> {
    outline: &'scene mut SceneOutline,
    selected: &'scene mut Option<usize>,
    dimension: ProjectDimension,
}

impl TreeEditor<'_> {
    /// Adds a node of `template` under `parent` and selects it.
    fn add_child(&mut self, parent: usize, template: NodeTemplate, images: &[String]) -> bool {
        let name = unique_child_name(self.outline, parent, template.node_name());
        let mut node = template.build(name, self.dimension, images);
        if self.outline.main_camera().is_some() {
            stop_being_main(&mut node);
        }
        match self.outline.add_node(parent, node) {
            Ok(index) => {
                *self.selected = Some(index);
                true
            }
            Err(error) => {
                log::warn!("{error}");
                false
            }
        }
    }

    /// Copies the node at `index` with its descendants under the same
    /// parent, with a free name, and selects the copy.
    fn duplicate(&mut self, index: usize) -> bool {
        let nodes = self.outline.nodes();
        let Some(parent) = nodes.get(index).and_then(|node| node.parent) else {
            return false;
        };
        let originals = subtree(nodes, index);
        let copies: Vec<_> = originals
            .iter()
            .filter_map(|&original| nodes.get(original).cloned())
            .collect();
        // Where each original's copy went, to place the copies' children.
        let mut placed: Vec<(usize, usize)> = Vec::with_capacity(copies.len());
        for (original, mut copy) in originals.into_iter().zip(copies) {
            // A scene has one main camera: the original stays it.
            stop_being_main(&mut copy);
            let copy_parent = if original == index {
                copy.name = unique_child_name(self.outline, parent, &copy.name);
                Some(parent)
            } else {
                copy.parent.and_then(|old_parent| {
                    placed
                        .iter()
                        .find(|(old, _)| *old == old_parent)
                        .map(|(_, new)| *new)
                })
            };
            let Some(copy_parent) = copy_parent else {
                continue;
            };
            match self.outline.add_node(copy_parent, copy) {
                Ok(new) => placed.push((original, new)),
                Err(error) => log::warn!("{error}"),
            }
        }
        let Some(&(_, copy)) = placed.first() else {
            return false;
        };
        *self.selected = Some(copy);
        true
    }

    /// Moves the node at `index`, with its descendants, to be the last
    /// child of `parent`, where it stays in the same place of the world,
    /// with a name free among its new siblings, and selects it.
    fn reparent(&mut self, index: usize, parent: usize) -> bool {
        let nodes = self.outline.nodes();
        let Some(node) = nodes
            .get(index)
            .filter(|_| tree::can_move(nodes, index, parent))
        else {
            return false;
        };
        let handed = drawing::handed_down(nodes);
        let (from_flat, from_solid) = drawing::parent_place(node, &handed);
        let Some(&(to_flat, to_solid)) = handed.get(parent) else {
            return false;
        };
        let transform = node
            .transform
            .map(|local| reparenting::keep_in_place_2d(local, from_flat, to_flat));
        let transform_3d = node
            .transform_3d
            .map(|local| reparenting::keep_in_place_3d(local, from_solid, to_solid));
        let name = unique_child_name(self.outline, parent, &node.name);
        let moved = match self.outline.move_node(index, parent) {
            Ok(moved) => moved,
            Err(error) => {
                log::warn!("{error}");
                return false;
            }
        };
        if let Some(node) = self.outline.node_mut(moved) {
            node.name = name;
            node.transform = transform;
            node.transform_3d = transform_3d;
        }
        *self.selected = Some(moved);
        true
    }

    /// Builds the node at `index` as `class`, and selects it.
    fn set_class(&mut self, index: usize, class: String) -> bool {
        let Some(node) = self.outline.node_mut(index) else {
            return false;
        };
        *self.selected = Some(index);
        if node.class.as_deref() == Some(class.as_str()) {
            return false;
        }
        node.class = Some(class);
        true
    }

    /// Deletes the node at `index` with its descendants, and selects its
    /// parent.
    fn delete(&mut self, index: usize) -> bool {
        let parent = self.outline.nodes().get(index).and_then(|node| node.parent);
        match self.outline.remove_node(index) {
            Ok(()) => {
                // Nodes before the one removed keep their place.
                *self.selected = parent;
                true
            }
            Err(error) => {
                log::warn!("{error}");
                false
            }
        }
    }
}

/// The node at `index` and its descendants, parents first.
fn subtree(nodes: &[NodeOutline], index: usize) -> Vec<usize> {
    let mut inside = vec![index];
    for (position, node) in nodes.iter().enumerate().skip(index + 1) {
        if node.parent.is_some_and(|parent| inside.contains(&parent)) {
            inside.push(position);
        }
    }
    inside
}

/// `base`, or `base2`, `base3`… : the first that no child of `parent` has.
fn unique_child_name(outline: &SceneOutline, parent: usize, base: &str) -> String {
    let taken = |name: &str| {
        outline
            .nodes()
            .iter()
            .any(|node| node.parent == Some(parent) && node.name == name)
    };
    if !taken(base) {
        return base.to_owned();
    }
    // Among one more name than there are nodes, one is free.
    (2..=outline.nodes().len() + 1)
        .map(|number| format!("{base}{number}"))
        .find(|name| !taken(name))
        .unwrap_or_else(|| base.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;
    use star_engine::prelude::{Transform2D, Vec2};

    const LEVEL: &str = "format = 1\n[[node]]\nname = \"One\"";

    fn open_level(name: &str) -> (TestDirectory, SceneDocument) {
        let assets = TestDirectory::new(name);
        fs::create_dir_all(assets.path().join("levels")).unwrap();
        fs::write(assets.path().join("levels/one.scene"), LEVEL).unwrap();
        let scene = SceneDocument::open(
            assets.path(),
            "levels/one.scene".to_owned(),
            ProjectDimension::TwoD,
        );
        (assets, scene)
    }

    fn tree_editor(scene: &mut SceneDocument) -> TreeEditor<'_> {
        TreeEditor {
            outline: scene.content.as_mut().unwrap(),
            selected: &mut scene.selected,
            dimension: scene.dimension,
        }
    }

    #[test]
    fn scenes_are_read_from_the_assets_folder() {
        let (_assets, scene) = open_level("scene_document_read");

        assert_eq!(scene.title(), "one.scene");
        assert_eq!(scene.content.unwrap().nodes()[0].name, "One");
    }

    #[test]
    fn scenes_that_cannot_be_read_say_why() {
        let assets = TestDirectory::new("scene_document_broken");
        fs::create_dir_all(assets.path()).unwrap();
        fs::write(assets.path().join("bad.scene"), "format = 2").unwrap();

        let broken = SceneDocument::open(
            assets.path(),
            "bad.scene".to_owned(),
            ProjectDimension::TwoD,
        );
        let missing = SceneDocument::open(
            assets.path(),
            "missing.scene".to_owned(),
            ProjectDimension::TwoD,
        );

        assert!(broken.content.unwrap_err().contains("format"));
        assert!(missing.content.is_err());
    }

    #[test]
    fn added_nodes_get_free_names_and_are_selected() {
        let (_assets, mut scene) = open_level("scene_document_add");
        let mut editor = tree_editor(&mut scene);

        assert!(editor.add_child(ROOT, NodeTemplate::Empty, &[]));
        assert!(editor.add_child(ROOT, NodeTemplate::Empty, &[]));

        let names: Vec<_> = editor
            .outline
            .nodes()
            .iter()
            .map(|node| node.name.clone())
            .collect();
        assert_eq!(names, ["One", "Node", "Node2"]);
        assert_eq!(scene.selected, Some(2));
    }

    #[test]
    fn only_the_first_camera_created_or_copied_is_main() {
        let (_assets, mut scene) = open_level("scene_document_cameras");
        let mut editor = tree_editor(&mut scene);

        editor.add_child(ROOT, NodeTemplate::Camera2D, &[]);
        editor.add_child(ROOT, NodeTemplate::Camera3D, &[]);
        editor.duplicate(1);

        let mains: Vec<_> = editor.outline.nodes()[1..]
            .iter()
            .map(NodeOutline::is_main_camera)
            .collect();
        assert_eq!(mains, [true, false, false]);
    }

    #[test]
    fn a_new_main_camera_takes_the_place_of_the_old_one() {
        let (_assets, mut scene) = open_level("scene_document_new_main");
        let mut editor = tree_editor(&mut scene);
        editor.add_child(ROOT, NodeTemplate::Camera2D, &[]);
        editor.add_child(ROOT, NodeTemplate::Camera3D, &[]);
        let outline = scene.content.as_mut().unwrap();
        if let Some(camera) = &mut outline.node_mut(2).unwrap().camera_3d {
            camera.main = true;
        }

        scene.make_only_main_camera();

        assert_eq!(scene.content.unwrap().main_camera(), Some(2));
    }

    #[test]
    fn deleting_a_node_selects_its_parent() {
        let (_assets, mut scene) = open_level("scene_document_delete");
        let mut editor = tree_editor(&mut scene);
        editor.add_child(ROOT, NodeTemplate::Empty, &[]);

        assert!(editor.delete(1));

        assert_eq!(editor.outline.nodes().len(), 1);
        assert_eq!(scene.selected, Some(ROOT));
    }

    #[test]
    fn duplicates_copy_the_descendants_under_the_same_parent() {
        let (_assets, mut scene) = open_level("scene_document_duplicate");
        let mut editor = tree_editor(&mut scene);
        editor.add_child(ROOT, NodeTemplate::Empty, &[]);
        editor.add_child(1, NodeTemplate::Sprite, &["hero.png".to_owned()]);

        assert!(editor.duplicate(1));

        let nodes = editor.outline.nodes();
        let names: Vec<_> = nodes.iter().map(|node| node.name.as_str()).collect();
        assert_eq!(names, ["One", "Node", "Sprite", "Node2", "Sprite"]);
        assert_eq!(nodes[3].parent, Some(ROOT));
        assert_eq!(nodes[4].parent, Some(3));
        assert!(nodes[4].sprite.is_some());
        assert_eq!(scene.selected, Some(3));
    }

    #[test]
    fn moved_nodes_stay_in_place_with_a_free_name() {
        let (_assets, mut scene) = open_level("scene_document_move");
        let mut editor = tree_editor(&mut scene);
        editor.add_child(ROOT, NodeTemplate::Empty, &[]);
        editor.add_child(ROOT, NodeTemplate::Empty, &[]);
        editor.add_child(1, NodeTemplate::Empty, &[]);
        editor.outline.node_mut(1).unwrap().transform =
            Some(Transform2D::from_position(Vec2::new(5.0, 0.0)));
        editor.outline.node_mut(3).unwrap().name = "Node2".to_owned();
        editor.outline.node_mut(3).unwrap().transform =
            Some(Transform2D::from_position(Vec2::new(1.0, 0.0)));

        assert!(editor.reparent(3, ROOT));

        let nodes = editor.outline.nodes();
        let names: Vec<_> = nodes.iter().map(|node| node.name.as_str()).collect();
        assert_eq!(names, ["One", "Node", "Node2", "Node22"]);
        assert_eq!(nodes[3].parent, Some(ROOT));
        assert_eq!(nodes[3].transform.unwrap().position, Vec2::new(6.0, 0.0));
        assert_eq!(scene.selected, Some(3));
    }

    #[test]
    fn nodes_never_move_into_themselves() {
        let (_assets, mut scene) = open_level("scene_document_move_inside");
        let mut editor = tree_editor(&mut scene);
        editor.add_child(ROOT, NodeTemplate::Empty, &[]);
        editor.add_child(1, NodeTemplate::Empty, &[]);

        assert!(!editor.reparent(1, 2));
        assert!(!editor.reparent(ROOT, 1));
        assert_eq!(editor.outline.nodes()[2].parent, Some(1));
    }

    #[test]
    fn dropped_classes_are_given_to_the_node() {
        let (_assets, mut scene) = open_level("scene_document_class");
        let mut editor = tree_editor(&mut scene);

        assert!(editor.set_class(ROOT, "Level".to_owned()));
        assert!(!editor.set_class(ROOT, "Level".to_owned()));
        assert!(!editor.set_class(7, "Level".to_owned()));

        assert_eq!(editor.outline.nodes()[0].class.as_deref(), Some("Level"));
        assert_eq!(scene.selected, Some(ROOT));
    }

    fn script(classes: &[&str]) -> ClassDrop {
        ClassDrop {
            path: "level.star".to_owned(),
            classes: classes.iter().map(|&class| class.to_owned()).collect(),
            mistake: None,
        }
    }

    #[test]
    fn scripts_dropped_on_the_selected_node_give_it_their_only_class() {
        let (_assets, mut scene) = open_level("scene_document_inspector_drop");
        scene.drop_class_on_selected(&script(&["Level"]), egui::Pos2::ZERO);
        assert_eq!(scene.content.as_ref().unwrap().nodes()[0].class, None);

        scene.selected = Some(ROOT);
        scene.drop_class_on_selected(&script(&["Level"]), egui::Pos2::ZERO);

        assert_eq!(
            scene.content.as_ref().unwrap().nodes()[0].class.as_deref(),
            Some("Level")
        );
        assert!(scene.is_unsaved());
    }

    #[test]
    fn scripts_with_several_classes_ask_which_one() {
        let (_assets, mut scene) = open_level("scene_document_class_choice");
        scene.selected = Some(ROOT);

        scene.drop_class_on_selected(&script(&[]), egui::Pos2::ZERO);
        assert!(scene.class_choice.is_none());
        scene.drop_class_on_selected(&script(&["Level", "Boss"]), egui::Pos2::ZERO);

        let choice = scene.class_choice.as_ref().unwrap();
        assert_eq!(choice.node, ROOT);
        assert_eq!(choice.classes, ["Level", "Boss"]);
        assert!(!scene.is_unsaved());
    }

    #[test]
    fn the_root_is_never_duplicated() {
        let (_assets, mut scene) = open_level("scene_document_duplicate_root");

        assert!(!tree_editor(&mut scene).duplicate(ROOT));
    }

    #[test]
    fn changes_can_be_undone_and_redone() {
        let (_assets, mut scene) = open_level("scene_document_undo");
        tree_editor(&mut scene).add_child(ROOT, NodeTemplate::Empty, &[]);
        scene.mark_changed();
        scene.settle(&egui::Context::default());
        assert!(scene.can_undo());

        scene.undo();
        assert_eq!(scene.content.as_ref().unwrap().nodes().len(), 1);
        assert_eq!(scene.selected, None);
        assert!(!scene.is_unsaved());

        scene.redo();
        assert_eq!(scene.content.as_ref().unwrap().nodes().len(), 2);
        assert_eq!(scene.selected, Some(1));
        assert!(scene.is_unsaved());
    }

    #[test]
    fn saving_writes_the_changes_and_clears_the_mark() {
        let (assets, mut scene) = open_level("scene_document_save");
        tree_editor(&mut scene).add_child(ROOT, NodeTemplate::Empty, &[]);
        scene.mark_changed();

        assert!(scene.save());

        assert!(!scene.is_unsaved());
        assert_eq!(scene.title(), "one.scene");
        let text = fs::read_to_string(assets.path().join("levels/one.scene")).unwrap();
        let saved = SceneOutline::from_text(&text).unwrap();
        assert_eq!(saved.nodes()[1].name, "Node");
    }
}
