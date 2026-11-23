//! How the “Scene” panel draws a scene's nodes: one row each, across the
//! whole panel, with stripes so long lists can be followed, guides down
//! to the children of every node, an icon for what the node is and its
//! class on the right. A search field narrows the list to the nodes whose
//! name matches.

use std::collections::HashSet;
use std::f32::consts::FRAC_1_SQRT_2;
use std::sync::Arc;

use eframe::egui;
use star_engine::star_scene::NodeOutline;

use super::ROOT;
use super::dropping::ImageDrop;
use super::tree::children_of;
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
/// The width of the outline of a row an image would be dropped on.
const DROP_TARGET_WIDTH: f32 = 2.0;

const SPRITE_COLOR: egui::Color32 = egui::Color32::from_rgb(110, 190, 120);
const MESH_COLOR: egui::Color32 = egui::Color32::from_rgb(95, 155, 230);
const LIGHT_COLOR: egui::Color32 = egui::Color32::from_rgb(240, 200, 80);
const BODY_COLOR: egui::Color32 = egui::Color32::from_rgb(180, 120, 220);
const EMPTY_COLOR: egui::Color32 = egui::Color32::from_rgb(150, 150, 160);
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
    Light,
    Mesh,
    Sprite,
    Body,
    Empty,
}

impl NodeKind {
    fn of(node: &NodeOutline) -> Self {
        if node.light.is_some() {
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
    /// An image dropped on a row this frame, with that row's node.
    dropped: Option<(usize, Arc<ImageDrop>)>,
}

impl<'scene> TreeView<'scene> {
    pub(super) fn new(
        nodes: &'scene [NodeOutline],
        state: &'scene mut TreeState,
        selected: &'scene mut Option<usize>,
    ) -> Self {
        Self {
            nodes,
            children: children_of(nodes),
            paths: paths_of(nodes),
            state,
            selected,
            rows: 0,
            dropped: None,
        }
    }

    /// The search field and, under it, the tree, or the nodes that match.
    /// Returns the image dropped on a row, if one was, with its node.
    pub(super) fn show(mut self, ui: &mut egui::Ui) -> Option<(usize, Arc<ImageDrop>)> {
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
            });
        self.state.reveal = false;
        self.dropped
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
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), ROW_HEIGHT),
            egui::Sense::click(),
        );
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

        if response.clicked() && !toggled {
            *self.selected = Some(index);
        }
        if response.dnd_hover_payload::<ImageDrop>().is_some() {
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(DROP_TARGET_WIDTH, ui.visuals().selection.stroke.color),
                egui::StrokeKind::Inside,
            );
        }
        if let Some(image) = response.dnd_release_payload::<ImageDrop>() {
            self.dropped = Some((index, image));
        }
        if selected && self.state.reveal {
            response.scroll_to_me(Some(egui::Align::Center));
        }
        toggled
    }
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
/// a sun for lights, a ring for bodies and a hollow dot for the rest.
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
             [[node]]\nname = \"Enemy player\"\nparent = 0\n",
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
                "Level/Enemy player"
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
                NodeKind::Empty
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

        TreeView::new(&nodes, &mut state, &mut selected).open_parents_of_selected();

        assert!(state.collapsed.is_empty());
    }
}
