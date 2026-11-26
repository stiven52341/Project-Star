//! How the “Scene” panel draws a scene's nodes: one row each, across the
//! whole panel, with stripes so long lists can be followed, guides down
//! to the children of every node, an icon for what the node is and its
//! class on the right. A search field narrows the list to the nodes whose
//! name matches. Right clicks open menus to create, duplicate and delete
//! nodes, and rows dragged onto another node (or under the rows, for the
//! root) move there. Scripts and blueprints dropped on a node give it
//! their class, or a choice of them if they have several.

use std::collections::HashSet;
use std::f32::consts::FRAC_1_SQRT_2;
use std::sync::Arc;

use eframe::egui;
use star_engine::star_scene::NodeOutline;

use super::ROOT;
use super::class_drop::ClassDrop;
use super::components::{BodyTemplate, NodeTemplate};
use super::dropping::AssetDrop;
use super::tree::{can_move, children_of};
use crate::drag_drop::released;
use crate::project::ProjectDimension;
use crate::texts;

const ROW_HEIGHT: f32 = 22.0;
/// Space between a row's edges and what it shows.
const ROW_PADDING: f32 = 4.0;
/// How far children are moved right of their parent; also the width of
/// the arrow that opens and closes them.
const INDENT: f32 = 16.0;
const ICON_SIZE: f32 = 14.0;
const ICON_GAP: f32 = 6.0;
const ARROW_HALF: f32 = 3.5;
const GUIDE_WIDTH: f32 = 1.0;
const CLASS_TEXT_SIZE: f32 = 11.0;
/// The keys shown next to the menu entries they also do.
const DUPLICATE_SHORTCUT: &str = "Ctrl+D";
const DELETE_SHORTCUT: &str = "Del";
/// The width of the outline of a row an image or a node would be dropped
/// on.
const DROP_TARGET_WIDTH: f32 = 2.0;
/// How far from the pointer the name of a node dragged is shown.
const DRAGGED_OFFSET: f32 = 12.0;

const SPRITE_COLOR: egui::Color32 = egui::Color32::from_rgb(110, 190, 120);
const MESH_COLOR: egui::Color32 = egui::Color32::from_rgb(95, 155, 230);
const LIGHT_COLOR: egui::Color32 = egui::Color32::from_rgb(240, 200, 80);
const BODY_COLOR: egui::Color32 = egui::Color32::from_rgb(180, 120, 220);
const EMPTY_COLOR: egui::Color32 = egui::Color32::from_rgb(150, 150, 160);
const CAMERA_COLOR: egui::Color32 = egui::Color32::from_rgb(90, 200, 210);
/// The camera icon: a box, as wide as the icon less its lens and as high
/// as this part of its width, with a lens on its right.
const CAMERA_BOX_ASPECT: f32 = 0.6;
const CAMERA_LENS_WIDTH: f32 = 4.0;
const CAMERA_ROUNDING: f32 = 1.5;
const ICON_STROKE: f32 = 1.5;
const LIGHT_RADIUS: f32 = 3.0;
/// Where the rays of the light icon start and end, from its center.
const RAY_START: f32 = 4.5;
const RAY_END: f32 = 6.5;
/// The directions of the light icon's rays: across, up and the diagonals.
const RAYS: [(f32, f32); 8] = [
    (1.0, 0.0),
    (-1.0, 0.0),
    (0.0, 1.0),
    (0.0, -1.0),
    (FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (-FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
    (-FRAC_1_SQRT_2, -FRAC_1_SQRT_2),
];
/// Height of the sprite icon's picture frame, for its width of one.
const PICTURE_ASPECT: f32 = 0.8;
const PICTURE_ROUNDING: f32 = 2.0;
/// How far the mountain drawn in the sprite icon stays from the frame.
const PICTURE_INSET: f32 = 2.0;
/// Half the width of the mesh icon's hexagon, for a height of one:
/// the cosine of 30°.
const HEXAGON_HALF_WIDTH: f32 = 0.866;
const BODY_RING_WIDTH: f32 = 2.0;
const BODY_DOT_RADIUS: f32 = 2.0;

/// What the tree remembers between frames: the nodes closed, by path, so
/// they stay closed when nodes are added or deleted before them; the text
/// searched for; and whether to show the node selected, opening its
/// parents, after it was chosen elsewhere.
#[derive(Debug, Default)]
pub(super) struct TreeState {
    collapsed: HashSet<String>,
    search: String,
    reveal: bool,
}

impl TreeState {
    /// Opens the parents of the node selected and scrolls to it the next
    /// time the tree is drawn.
    pub(super) const fn reveal_selected(&mut self) {
        self.reveal = true;
    }
}

/// What a node is, by what it does that shows most: its icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Camera,
    Light,
    Mesh,
    Sprite,
    Body,
    Empty,
}

impl NodeKind {
    fn of(node: &NodeOutline) -> Self {
        if node.camera.is_some() || node.camera_3d.is_some() {
            Self::Camera
        } else if node.light.is_some() || node.light_2d.is_some() {
            Self::Light
        } else if node.mesh.is_some() {
            Self::Mesh
        } else if node.sprite.is_some() {
            Self::Sprite
        } else if node.body.is_some() || node.body_3d.is_some() {
            Self::Body
        } else {
            Self::Empty
        }
    }
}

/// What the user asked the tree to change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TreeAction {
    /// Add a node of `template` under `parent`.
    Create {
        parent: usize,
        template: NodeTemplate,
    },
    /// Copy the node, with its descendants, next to it.
    Duplicate(usize),
    /// Delete the node with its descendants.
    Delete(usize),
    /// Move `node`, with its descendants, to be the last child of `parent`.
    Move { node: usize, parent: usize },
    /// Build `node` as `class`.
    SetClass { node: usize, class: String },
}

/// What a row carries while it is dragged: the position of its node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NodeDrag(usize);

/// What happened in the tree this frame.
#[derive(Default)]
pub(super) struct TreeOutcome {
    /// An image dropped on a row, with that row's node.
    pub(super) dropped: Option<(usize, Arc<AssetDrop>)>,
    /// A script or blueprint dropped on a row, with that row's node and
    /// where it was dropped.
    pub(super) class_dropped: Option<(usize, Arc<ClassDrop>, egui::Pos2)>,
    pub(super) action: Option<TreeAction>,
}

/// What the menus offer: the templates of the project's kind, sprites
/// only if there are images.
#[derive(Clone, Copy)]
pub(super) struct MenuContext<'images> {
    pub(super) dimension: ProjectDimension,
    pub(super) images: &'images [String],
}

/// The tree of a scene's nodes for one frame.
pub(super) struct TreeView<'scene> {
    nodes: &'scene [NodeOutline],
    children: Vec<Vec<usize>>,
    /// The path of every node, `Root/Player/Sword`: names are unique
    /// among siblings, so paths name nodes while others come and go.
    paths: Vec<String>,
    state: &'scene mut TreeState,
    selected: &'scene mut Option<usize>,
    /// Rows drawn so far, to stripe every other one.
    rows: usize,
    menu: MenuContext<'scene>,
    outcome: TreeOutcome,
}

impl<'scene> TreeView<'scene> {
    pub(super) fn new(
        nodes: &'scene [NodeOutline],
        state: &'scene mut TreeState,
        selected: &'scene mut Option<usize>,
        menu: MenuContext<'scene>,
    ) -> Self {
        Self {
            nodes,
            children: children_of(nodes),
            paths: paths_of(nodes),
            state,
            selected,
            rows: 0,
            menu,
            outcome: TreeOutcome::default(),
        }
    }

    /// The search field and, under it, the tree, or the nodes that match.
    /// Returns what the user dropped on it or chose in its menus.
    pub(super) fn show(mut self, ui: &mut egui::Ui) -> TreeOutcome {
        let texts = texts::current();
        ui.add(
            egui::TextEdit::singleline(&mut self.state.search)
                .hint_text(texts.search_nodes)
                .desired_width(f32::INFINITY),
        );
        if self.state.reveal {
            self.open_parents_of_selected();
        }
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let search = self.state.search.trim().to_lowercase();
                if search.is_empty() {
                    self.show_branch(ui, ROOT, 0);
                } else {
                    let matches = matching(self.nodes, &search);
                    if matches.is_empty() {
                        ui.weak(texts.no_nodes_match);
                    }
                    for index in matches {
                        self.show_row(ui, index, 0, None);
                    }
                }
                self.show_background(ui);
            });
        self.state.reveal = false;
        self.show_dragged_node(ui);
        self.outcome
    }

    /// While a row is dragged, its node's name follows the pointer.
    fn show_dragged_node(&self, ui: &egui::Ui) {
        let ctx = ui.ctx();
        let Some(name) = egui::DragAndDrop::payload::<NodeDrag>(ctx)
            .and_then(|drag| self.nodes.get(drag.0))
            .map(|node| node.name.as_str())
        else {
            return;
        };
        let Some(cursor) = ctx.pointer_latest_pos() else {
            return;
        };
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::unique("dragged_scene_node"),
        ));
        painter.text(
            cursor + egui::Vec2::splat(DRAGGED_OFFSET),
            egui::Align2::LEFT_TOP,
            name,
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().strong_text_color(),
        );
    }

    /// Outlines `rect` while something that can land on the node at
    /// `target` is held over `response`, and takes a node released there
    /// as a move under that node.
    fn drop_target(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        target: usize,
        is_row: bool,
    ) {
        let nodes = self.nodes;
        let fits = |drag: &NodeDrag| can_move(nodes, drag.0, target);
        let node_lands = response
            .dnd_hover_payload::<NodeDrag>()
            .is_some_and(|drag| fits(&drag));
        let image_held = is_row && response.dnd_hover_payload::<AssetDrop>().is_some();
        let class_held = is_row
            && response
                .dnd_hover_payload::<ClassDrop>()
                .is_some_and(|script| !script.classes.is_empty());
        if node_lands || image_held || class_held {
            ui.painter().rect_stroke(
                response.rect,
                0.0,
                egui::Stroke::new(DROP_TARGET_WIDTH, ui.visuals().selection.stroke.color),
                egui::StrokeKind::Inside,
            );
        }
        if let Some(drag) = released::<NodeDrag>(response).filter(|drag| fits(drag)) {
            self.outcome.action = Some(TreeAction::Move {
                node: drag.0,
                parent: target,
            });
        }
        if let Some(script) = released::<ClassDrop>(response).filter(|_| is_row) {
            let at = response
                .ctx
                .pointer_latest_pos()
                .unwrap_or_else(|| response.rect.center());
            self.outcome.class_dropped = Some((target, script, at));
        }
    }

    /// The space under the rows: a click there selects nothing, and its
    /// menu creates nodes under the root.
    fn show_background(&mut self, ui: &mut egui::Ui) {
        let space = ui.available_size().max(egui::vec2(0.0, ROW_HEIGHT));
        let background = ui.allocate_response(space, egui::Sense::click());
        if background.clicked() {
            *self.selected = None;
        }
        let mut action = None;
        background.context_menu(|ui| create_menu(ui, ROOT, self.menu, &mut action));
        if self.outcome.action.is_none() {
            self.outcome.action = action;
        }
        self.drop_target(ui, &background, ROOT, false);
    }

    fn open_parents_of_selected(&mut self) {
        let mut parent = self
            .selected
            .and_then(|selected| self.nodes.get(selected))
            .and_then(|node| node.parent);
        while let Some(index) = parent {
            if let Some(path) = self.paths.get(index) {
                self.state.collapsed.remove(path);
            }
            parent = self.nodes.get(index).and_then(|node| node.parent);
        }
    }

    /// The node at `index`, `depth` levels down, and its children if it
    /// is open.
    fn show_branch(&mut self, ui: &mut egui::Ui, index: usize, depth: usize) {
        let Some(open) = self
            .paths
            .get(index)
            .map(|path| !self.state.collapsed.contains(path))
        else {
            return;
        };
        let has_children = self
            .children
            .get(index)
            .is_some_and(|list| !list.is_empty());
        let toggled = self.show_row(ui, index, depth, has_children.then_some(open));
        if let Some(path) = self.paths.get(index).filter(|_| toggled) {
            if open {
                self.state.collapsed.insert(path.clone());
            } else {
                self.state.collapsed.remove(path);
            }
        }
        if has_children && open != toggled {
            let count = self.children.get(index).map_or(0, Vec::len);
            for position in 0..count {
                if let Some(&child) = self.children.get(index).and_then(|list| list.get(position)) {
                    self.show_branch(ui, child, depth + 1);
                }
            }
        }
    }

    /// One row: the node at `index`, `depth` levels down, with an arrow if
    /// it has children (`Some(open)`). A click selects it. Returns whether
    /// its arrow was clicked.
    fn show_row(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        depth: usize,
        open: Option<bool>,
    ) -> bool {
        let Some(node) = self.nodes.get(index) else {
            return false;
        };
        // The root stays first: it cannot be dragged.
        let sense = if index == ROOT {
            egui::Sense::click()
        } else {
            egui::Sense::click_and_drag()
        };
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), sense);
        let selected = *self.selected == Some(index);
        let painter = ui.painter_at(rect);
        paint_background(ui, &painter, rect, &response, selected, self.rows);
        self.rows += 1;

        let left = guides(ui, &painter, rect, depth);
        let arrow_rect =
            egui::Rect::from_min_size(egui::pos2(left, rect.top()), egui::vec2(INDENT, ROW_HEIGHT));
        let mut toggled = false;
        if let Some(open) = open {
            let arrow = ui.interact(
                arrow_rect,
                ui.make_persistent_id(("scene_tree_arrow", index)),
                egui::Sense::click(),
            );
            toggled = arrow.clicked();
            let color = if arrow.hovered() {
                ui.visuals().strong_text_color()
            } else {
                ui.visuals().weak_text_color()
            };
            paint_arrow(&painter, arrow_rect.center(), open, color);
        }

        let icon_center = egui::pos2(left + INDENT + ICON_SIZE / 2.0, rect.center().y);
        paint_icon(&painter, icon_center, NodeKind::of(node));
        let text_color = if selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        };
        painter.text(
            egui::pos2(left + INDENT + ICON_SIZE + ICON_GAP, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &node.name,
            egui::TextStyle::Body.resolve(ui.style()),
            text_color,
        );
        if let Some(class) = &node.class {
            painter.text(
                egui::pos2(rect.right() - ROW_PADDING, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                class,
                egui::FontId::proportional(CLASS_TEXT_SIZE),
                ui.visuals().weak_text_color(),
            );
        }

        if (response.clicked() || response.secondary_clicked()) && !toggled {
            *self.selected = Some(index);
        }
        let mut action = None;
        response.context_menu(|ui| node_menu(ui, index, self.menu, &mut action));
        if self.outcome.action.is_none() {
            self.outcome.action = action;
        }
        if index != ROOT {
            response.dnd_set_drag_payload(NodeDrag(index));
        }
        self.drop_target(ui, &response, index, true);
        if let Some(image) = released::<AssetDrop>(&response) {
            self.outcome.dropped = Some((index, image));
        }
        if selected && self.state.reveal {
            response.scroll_to_me(Some(egui::Align::Center));
        }
        toggled
    }
}

/// The menu of a node's row: create a child, duplicate it or delete it.
/// The root can be neither duplicated nor deleted.
fn node_menu(
    ui: &mut egui::Ui,
    index: usize,
    menu: MenuContext<'_>,
    action: &mut Option<TreeAction>,
) {
    let texts = texts::current();
    create_menu(ui, index, menu, action);
    ui.separator();
    let is_root = index == ROOT;
    let duplicate = egui::Button::new(texts.duplicate).shortcut_text(DUPLICATE_SHORTCUT);
    if ui
        .add_enabled(!is_root, duplicate)
        .on_disabled_hover_text(texts.root_cannot_be_duplicated)
        .clicked()
    {
        *action = Some(TreeAction::Duplicate(index));
    }
    let delete = egui::Button::new(texts.delete).shortcut_text(DELETE_SHORTCUT);
    if ui
        .add_enabled(!is_root, delete)
        .on_disabled_hover_text(texts.root_cannot_be_deleted)
        .clicked()
    {
        *action = Some(TreeAction::Delete(index));
    }
}

/// “Create”, with the templates of the project's kind, to add under
/// `parent`.
fn create_menu(
    ui: &mut egui::Ui,
    parent: usize,
    menu: MenuContext<'_>,
    action: &mut Option<TreeAction>,
) {
    let texts = texts::current();
    ui.menu_button(texts.create, |ui| {
        for &template in NodeTemplate::of(menu.dimension) {
            let available = template.is_available(menu.images);
            if ui
                .add_enabled(available, egui::Button::new(template.label()))
                .on_disabled_hover_text(texts.no_images_hint)
                .clicked()
            {
                *action = Some(TreeAction::Create { parent, template });
            }
        }
        ui.menu_button(texts.physics, |ui| {
            for body in BodyTemplate::ALL {
                let template = NodeTemplate::Body(body);
                if ui.button(template.label()).clicked() {
                    *action = Some(TreeAction::Create { parent, template });
                }
            }
        });
    });
}

/// Fills a row: the selection color if it is selected, a lighter one
/// under the pointer, and a faint stripe on every other row.
fn paint_background(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    response: &egui::Response,
    selected: bool,
    row: usize,
) {
    let visuals = ui.visuals();
    let fill = if selected {
        visuals.selection.bg_fill
    } else if response.hovered() {
        visuals.widgets.hovered.weak_bg_fill
    } else if row % 2 == 1 {
        visuals.faint_bg_color
    } else {
        return;
    };
    painter.rect_filled(rect, 0.0, fill);
}

/// Draws a faint line down from each of the `depth` parents' arrows and
/// returns where the row's own arrow starts.
fn guides(ui: &egui::Ui, painter: &egui::Painter, rect: egui::Rect, depth: usize) -> f32 {
    let first = rect.left() + ROW_PADDING;
    let stroke = egui::Stroke::new(
        GUIDE_WIDTH,
        ui.visuals().widgets.noninteractive.bg_stroke.color,
    );
    let mut left = first;
    for _ in 0..depth {
        let x = left + INDENT / 2.0;
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            stroke,
        );
        left += INDENT;
    }
    left
}

/// A small triangle pointing right when closed and down when open.
fn paint_arrow(painter: &egui::Painter, center: egui::Pos2, open: bool, color: egui::Color32) {
    let points = if open {
        vec![
            center + egui::vec2(-ARROW_HALF, -ARROW_HALF / 2.0),
            center + egui::vec2(ARROW_HALF, -ARROW_HALF / 2.0),
            center + egui::vec2(0.0, ARROW_HALF),
        ]
    } else {
        vec![
            center + egui::vec2(-ARROW_HALF / 2.0, -ARROW_HALF),
            center + egui::vec2(ARROW_HALF, 0.0),
            center + egui::vec2(-ARROW_HALF / 2.0, ARROW_HALF),
        ]
    };
    painter.add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));
}

/// The icon of a node of `kind`: a picture for sprites, a cube for meshes,
/// a sun for lights, a box with a lens for cameras, a ring for bodies and
/// a hollow dot for the rest.
fn paint_icon(painter: &egui::Painter, center: egui::Pos2, kind: NodeKind) {
    let half = ICON_SIZE / 2.0;
    match kind {
        NodeKind::Sprite => {
            let frame = egui::Rect::from_center_size(
                center,
                egui::vec2(ICON_SIZE, ICON_SIZE * PICTURE_ASPECT),
            );
            painter.rect_stroke(
                frame,
                PICTURE_ROUNDING,
                egui::Stroke::new(ICON_STROKE, SPRITE_COLOR),
                egui::StrokeKind::Inside,
            );
            let mountain = vec![
                frame.left_bottom() + egui::vec2(PICTURE_INSET, -PICTURE_INSET),
                frame.center(),
                frame.right_bottom() + egui::vec2(-PICTURE_INSET, -PICTURE_INSET),
            ];
            painter.add(egui::Shape::convex_polygon(
                mountain,
                SPRITE_COLOR,
                egui::Stroke::NONE,
            ));
        }
        NodeKind::Mesh => {
            let corner = |x: f32, y: f32| center + egui::vec2(x * half, y * half);
            let outline = vec![
                corner(0.0, -1.0),
                corner(HEXAGON_HALF_WIDTH, -0.5),
                corner(HEXAGON_HALF_WIDTH, 0.5),
                corner(0.0, 1.0),
                corner(-HEXAGON_HALF_WIDTH, 0.5),
                corner(-HEXAGON_HALF_WIDTH, -0.5),
            ];
            let stroke = egui::Stroke::new(ICON_STROKE, MESH_COLOR);
            painter.add(egui::Shape::closed_line(outline, stroke));
            for end in [
                corner(HEXAGON_HALF_WIDTH, -0.5),
                corner(-HEXAGON_HALF_WIDTH, -0.5),
                corner(0.0, 1.0),
            ] {
                painter.line_segment([center, end], stroke);
            }
        }
        NodeKind::Light => {
            painter.circle_filled(center, LIGHT_RADIUS, LIGHT_COLOR);
            let stroke = egui::Stroke::new(ICON_STROKE, LIGHT_COLOR);
            for (x, y) in RAYS {
                let direction = egui::vec2(x, y);
                painter.line_segment(
                    [center + direction * RAY_START, center + direction * RAY_END],
                    stroke,
                );
            }
        }
        NodeKind::Camera => {
            let body = egui::Rect::from_min_size(
                center - egui::vec2(half, half * CAMERA_BOX_ASPECT),
                egui::vec2(ICON_SIZE - CAMERA_LENS_WIDTH, ICON_SIZE * CAMERA_BOX_ASPECT),
            );
            painter.rect_filled(body, CAMERA_ROUNDING, CAMERA_COLOR);
            let lens = vec![
                body.right_center(),
                egui::pos2(center.x + half, body.top()),
                egui::pos2(center.x + half, body.bottom()),
            ];
            painter.add(egui::Shape::convex_polygon(
                lens,
                CAMERA_COLOR,
                egui::Stroke::NONE,
            ));
        }
        NodeKind::Body => {
            painter.circle_stroke(
                center,
                half - BODY_RING_WIDTH / 2.0,
                egui::Stroke::new(BODY_RING_WIDTH, BODY_COLOR),
            );
            painter.circle_filled(center, BODY_DOT_RADIUS, BODY_COLOR);
        }
        NodeKind::Empty => {
            painter.circle_stroke(
                center,
                half / 2.0,
                egui::Stroke::new(ICON_STROKE, EMPTY_COLOR),
            );
        }
    }
}

/// The path of every node, from the root's name down to its own.
fn paths_of(nodes: &[NodeOutline]) -> Vec<String> {
    let mut paths: Vec<String> = Vec::with_capacity(nodes.len());
    for node in nodes {
        let path = match node.parent.and_then(|parent| paths.get(parent)) {
            Some(parent) => format!("{parent}/{}", node.name),
            None => node.name.clone(),
        };
        paths.push(path);
    }
    paths
}

/// The nodes whose name contains `search`, already in lower case, in
/// file order.
fn matching(nodes: &[NodeOutline], search: &str) -> Vec<usize> {
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.name.to_lowercase().contains(search))
        .map(|(index, _)| index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use star_engine::prelude::SceneOutline;

    fn nodes() -> Vec<NodeOutline> {
        SceneOutline::from_text(
            "format = 1\n\
             [[node]]\nname = \"Level\"\n\
             [[node]]\nname = \"Player\"\nparent = 0\nposition = [0.0, 0.0]\n\
             [node.sprite]\ntexture = \"hero.png\"\nsize = [1.0, 1.0]\n\
             [[node]]\nname = \"Lamp\"\nparent = 1\n[node.transform_3d]\n\
             [node.light]\nkind = \"point\"\nrange = 5.0\n\
             [[node]]\nname = \"Enemy player\"\nparent = 0\n\
             [[node]]\nname = \"Eye\"\nparent = 0\n[node.camera]\n",
        )
        .unwrap()
        .into_nodes()
    }

    #[test]
    fn paths_join_the_names_from_the_root() {
        assert_eq!(
            paths_of(&nodes()),
            [
                "Level",
                "Level/Player",
                "Level/Player/Lamp",
                "Level/Enemy player",
                "Level/Eye"
            ]
        );
    }

    #[test]
    fn searches_ignore_case() {
        assert_eq!(matching(&nodes(), "player"), [1, 3]);
        assert_eq!(matching(&nodes(), "boss"), Vec::<usize>::new());
    }

    #[test]
    fn icons_follow_what_shows_most() {
        let kinds: Vec<_> = nodes().iter().map(NodeKind::of).collect();

        assert_eq!(
            kinds,
            [
                NodeKind::Empty,
                NodeKind::Sprite,
                NodeKind::Light,
                NodeKind::Empty,
                NodeKind::Camera
            ]
        );
    }

    #[test]
    fn revealing_opens_the_parents_of_the_node_selected() {
        let nodes = nodes();
        let mut state = TreeState::default();
        state.collapsed.insert("Level".to_owned());
        state.collapsed.insert("Level/Player".to_owned());
        let mut selected = Some(2);

        TreeView::new(
            &nodes,
            &mut state,
            &mut selected,
            MenuContext {
                dimension: ProjectDimension::TwoD,
                images: &[],
            },
        )
        .open_parents_of_selected();

        assert!(state.collapsed.is_empty());
    }
}
