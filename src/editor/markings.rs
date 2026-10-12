//! Marks for running the table, which players see too: a ruler, spell area templates, and
//! pings.
//!
//! The Measure tool drags out a ruler or a template from where the mouse goes down. A
//! finished one stays up until the next one, Escape, or a click without a drag. Alt+click
//! anywhere pings: rings spread out from that spot for a moment. None of this is saved
//! with the map. It isn't part of the map either, so players see it while their view is
//! frozen too (but not during a blackout).
//!
//! Distances follow the 5e grid rules: each cell is 5 feet, and a ruler counts cells the
//! way a creature moves, with a diagonal step counting as one.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::{Anchor, Text2dShadow};
use bevy::text::{FontSize, TextColor, TextFont};
use bevy_egui::EguiContexts;

use crate::input::{alt_held, command_held};
use crate::map::{MapData, render_layer};
use crate::ui::DialogState;

use super::camera::{CameraZoom, EditorCamera};
use super::params::{CameraParams, is_cursor_over_ui};

pub const FEET_PER_CELL: f32 = 5.0;
/// How long a ping lasts, in seconds
const PING_SECONDS: f32 = 1.6;
const MARKING_COLOR: Color = Color::srgb(1.0, 0.85, 0.2);
/// Editor label size on screen, in pixels
const EDITOR_LABEL_SIZE: f32 = 16.0;
/// Player label size, in grid cells
const PLAYER_LABEL_CELLS: f32 = 0.4;
/// Above every map layer
const LABEL_Z: f32 = 900.0;

/// What the Measure tool drags out
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeasureShape {
    #[default]
    Ruler,
    /// A sphere or cylinder: the drag is its radius
    Circle,
    /// The drag is its length; it's as wide as it is long at its far end
    Cone,
    /// A cube: the drag sets its side, toward the mouse
    Square,
    /// 5 feet wide; the drag is its length
    Line,
}

impl MeasureShape {
    pub const ALL: [Self; 5] = [
        Self::Ruler,
        Self::Circle,
        Self::Cone,
        Self::Square,
        Self::Line,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Ruler => "Ruler",
            Self::Circle => "Circle",
            Self::Cone => "Cone",
            Self::Square => "Square",
            Self::Line => "Line",
        }
    }

    /// Whether its start snaps to a cell's middle (otherwise to a grid corner)
    fn starts_in_a_cell(self) -> bool {
        matches!(self, Self::Ruler | Self::Line)
    }
}

/// A ruler or template, from where the drag started to where it is now
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Marking {
    pub shape: MeasureShape,
    pub start: Vec2,
    pub end: Vec2,
}

impl Marking {
    /// Its size in whole cells (at least one): the cells moved for a ruler, a cube's side,
    /// otherwise the length of the drag
    pub fn cells(&self, grid_size: f32) -> f32 {
        let delta = (self.end - self.start).abs() / grid_size;
        let cells = match self.shape {
            MeasureShape::Ruler | MeasureShape::Square => delta.max_element(),
            MeasureShape::Circle | MeasureShape::Cone | MeasureShape::Line => delta.length(),
        };
        cells.round().max(1.0)
    }

    pub fn label(&self, grid_size: f32) -> String {
        let feet = self.cells(grid_size) * FEET_PER_CELL;
        match self.shape {
            MeasureShape::Ruler => format!("{feet} ft"),
            MeasureShape::Circle => format!("{feet} ft radius"),
            shape => format!("{feet} ft {}", shape.name().to_lowercase()),
        }
    }

    /// Where its label goes, and which side of the text is there: past the far end of a
    /// cone or line (the text running away from it), above a circle or square, above the
    /// end of a ruler
    pub fn label_place(&self, grid_size: f32) -> (Vec2, Anchor) {
        let size = self.cells(grid_size) * grid_size;
        let gap = grid_size * 0.3;
        match self.shape {
            MeasureShape::Ruler => (self.end + Vec2::new(0.0, gap), Anchor::BOTTOM_CENTER),
            MeasureShape::Circle => (
                self.start + Vec2::new(0.0, size + gap),
                Anchor::BOTTOM_CENTER,
            ),
            MeasureShape::Cone | MeasureShape::Line => {
                let along = self.direction();
                let anchor = if along.x > 0.5 {
                    Anchor::CENTER_LEFT
                } else if along.x < -0.5 {
                    Anchor::CENTER_RIGHT
                } else if along.y > 0.0 {
                    Anchor::BOTTOM_CENTER
                } else {
                    Anchor::TOP_CENTER
                };
                (self.start + along * (size + gap), anchor)
            }
            MeasureShape::Square => {
                let corners = self.corners(grid_size).unwrap_or_default();
                let top = corners.iter().map(|c| c.y).fold(f32::MIN, f32::max);
                let middle = corners.iter().map(|c| c.x).sum::<f32>() / corners.len().max(1) as f32;
                (Vec2::new(middle, top + gap), Anchor::BOTTOM_CENTER)
            }
        }
    }

    /// Which way the drag goes (up if it hasn't moved)
    fn direction(&self) -> Vec2 {
        (self.end - self.start).try_normalize().unwrap_or(Vec2::Y)
    }

    /// The template's corners, in order, for the shapes with corners
    pub fn corners(&self, grid_size: f32) -> Option<Vec<Vec2>> {
        let size = self.cells(grid_size) * grid_size;
        let along = self.direction();
        let across = along.perp();
        let start = self.start;
        match self.shape {
            MeasureShape::Ruler | MeasureShape::Circle => None,
            MeasureShape::Cone => Some(vec![
                start,
                start + along * size + across * size / 2.0,
                start + along * size - across * size / 2.0,
            ]),
            MeasureShape::Square => {
                // Toward the mouse on both axes
                let toward = Vec2::new(
                    if self.end.x < start.x { -1.0 } else { 1.0 },
                    if self.end.y < start.y { -1.0 } else { 1.0 },
                ) * size;
                Some(vec![
                    start,
                    start + Vec2::new(toward.x, 0.0),
                    start + toward,
                    start + Vec2::new(0.0, toward.y),
                ])
            }
            MeasureShape::Line => {
                let half_width = across * grid_size / 2.0;
                let end = start + along * size;
                Some(vec![
                    start + half_width,
                    end + half_width,
                    end - half_width,
                    start - half_width,
                ])
            }
        }
    }
}

/// The Measure tool's settings, and the ruler or template being shown
#[derive(Resource)]
pub struct MeasureState {
    pub shape: MeasureShape,
    /// Players see the ruler or template too
    pub show_players: bool,
    pub marking: Option<Marking>,
    dragging: bool,
}

impl Default for MeasureState {
    fn default() -> Self {
        Self {
            shape: MeasureShape::default(),
            show_players: true,
            marking: None,
            dragging: false,
        }
    }
}

/// Pings fading out, and whether the mouse button that made the last one is still down
#[derive(Resource, Default)]
pub struct Pings {
    /// Where, and when (in seconds since startup)
    pings: Vec<(Vec2, f32)>,
    /// The left button went down to ping and hasn't come up yet: the tools mustn't see it
    holding: bool,
}

/// Run condition: the left mouse button belongs to the tools, not to a ping.
pub fn not_pinging(pings: Res<Pings>) -> bool {
    !pings.holding
}

/// Ruler, template and ping lines, which players see too
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct MarkingGizmoGroup;

pub fn configure_marking_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<MarkingGizmoGroup>();
    // The editor's layer and the player camera's, whether or not the view is frozen
    config.render_layers = RenderLayers::from_layers(&[render_layer::SHARED, render_layer::FROZEN]);
    config.line.width = 3.0;
}

/// Who a label is drawn for: the GM's at a constant size on screen, the players' at a size
/// on the map
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum MarkingLabel {
    Editor,
    Player,
}

pub fn spawn_marking_labels(mut commands: Commands) {
    for (label, layer) in [
        (MarkingLabel::Editor, render_layer::EDITOR),
        (MarkingLabel::Player, render_layer::PLAYER),
    ] {
        commands.spawn((
            label,
            Text2d::default(),
            TextFont {
                font_size: FontSize::Px(EDITOR_LABEL_SIZE),
                ..default()
            },
            TextColor(MARKING_COLOR),
            Text2dShadow {
                offset: Vec2::new(1.0, -1.0),
                color: Color::BLACK,
            },
            Anchor::CENTER,
            Transform::from_xyz(0.0, 0.0, LABEL_Z),
            Visibility::Hidden,
            RenderLayers::layer(layer),
        ));
    }
}

/// Where a drag starting at `position` starts: a cell's middle or a grid corner, or right
/// there with Ctrl
fn snapped_start(shape: MeasureShape, position: Vec2, grid_size: f32, snap: bool) -> Vec2 {
    if !snap {
        position
    } else if shape.starts_in_a_cell() {
        super::snap_to_grid(position, grid_size, true)
    } else {
        (position / grid_size).round() * grid_size
    }
}

/// Measure tool: drag out a ruler or template; a click without a drag clears it.
pub fn handle_measure(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    camera: CameraParams,
    map_data: Res<MapData>,
    mut state: ResMut<MeasureState>,
    mut contexts: EguiContexts,
) {
    let grid_size = map_data.grid_size;
    let snap = !command_held(&keyboard);
    if mouse_button.just_pressed(MouseButton::Left) && !is_cursor_over_ui(&mut contexts) {
        let Some(position) = camera.cursor_world_pos() else {
            return;
        };
        let start = snapped_start(state.shape, position, grid_size, snap);
        state.marking = Some(Marking {
            shape: state.shape,
            start,
            end: start,
        });
        state.dragging = true;
        return;
    }
    if !state.dragging {
        return;
    }
    if mouse_button.pressed(MouseButton::Left) {
        if let (Some(position), Some(marking)) = (camera.cursor_world_pos(), &mut state.marking) {
            marking.end = if marking.shape == MeasureShape::Ruler && snap {
                super::snap_to_grid(position, grid_size, true)
            } else {
                position
            };
        }
        return;
    }
    // Released
    state.dragging = false;
    if state
        .marking
        .is_some_and(|m| m.start.distance(m.end) < grid_size / 2.0)
    {
        state.marking = None;
    }
}

/// Escape clears the ruler or template.
pub fn clear_marking(mut state: ResMut<MeasureState>) {
    state.marking = None;
    state.dragging = false;
}

/// A new map starts without the old one's marks.
pub fn clear_marks_on_map_change(mut state: ResMut<MeasureState>, mut pings: ResMut<Pings>) {
    state.marking = None;
    state.dragging = false;
    pings.pings.clear();
}

/// Alt+click pings, with any tool. The click is the ping's alone: the tools don't see it.
pub fn start_ping(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    dialog_state: Res<DialogState>,
    time: Res<Time>,
    camera: CameraParams,
    mut pings: ResMut<Pings>,
    mut contexts: EguiContexts,
) {
    if pings.holding && !mouse_button.pressed(MouseButton::Left) {
        pings.holding = false;
    }
    if !mouse_button.just_pressed(MouseButton::Left)
        || !alt_held(&keyboard)
        || dialog_state.any_modal_open
        || is_cursor_over_ui(&mut contexts)
    {
        return;
    }
    if let Some(position) = camera.cursor_world_pos() {
        let now = time.elapsed_secs();
        pings.pings.push((position, now));
        pings.holding = true;
    }
}

/// Draws the ruler or template, for players too unless the GM turned that off.
pub fn draw_marking(
    state: Res<MeasureState>,
    map_data: Res<MapData>,
    mut shared: Gizmos<MarkingGizmoGroup>,
    mut editor_only: Gizmos,
) {
    let Some(marking) = state.marking else {
        return;
    };
    let gizmos: &mut dyn MarkingGizmos = if state.show_players {
        &mut shared
    } else {
        &mut editor_only
    };
    let grid_size = map_data.grid_size;
    match marking.shape {
        MeasureShape::Ruler => {
            gizmos.line(marking.start, marking.end);
            gizmos.circle(marking.start, grid_size * 0.1);
            gizmos.circle(marking.end, grid_size * 0.1);
        }
        MeasureShape::Circle => {
            gizmos.circle(marking.start, marking.cells(grid_size) * grid_size);
            gizmos.circle(marking.start, grid_size * 0.08);
        }
        _ => {
            if let Some(corners) = marking.corners(grid_size) {
                gizmos.polygon(&corners);
            }
        }
    }
}

/// The two gizmo groups the marking can be drawn with
trait MarkingGizmos {
    fn line(&mut self, from: Vec2, to: Vec2);
    fn circle(&mut self, center: Vec2, radius: f32);
    fn polygon(&mut self, corners: &[Vec2]);
}

impl<Config: GizmoConfigGroup> MarkingGizmos for Gizmos<'_, '_, Config> {
    fn line(&mut self, from: Vec2, to: Vec2) {
        self.line_2d(from, to, MARKING_COLOR);
    }

    fn circle(&mut self, center: Vec2, radius: f32) {
        self.circle_2d(Isometry2d::from_translation(center), radius, MARKING_COLOR)
            .resolution(64);
    }

    fn polygon(&mut self, corners: &[Vec2]) {
        let closed = corners.iter().chain(corners.first()).copied();
        self.linestrip_2d(closed, MARKING_COLOR);
    }
}

/// Spreading rings for each ping, fading out.
pub fn draw_pings(
    time: Res<Time>,
    map_data: Res<MapData>,
    mut pings: ResMut<Pings>,
    mut gizmos: Gizmos<MarkingGizmoGroup>,
) {
    let now = time.elapsed_secs();
    pings
        .pings
        .retain(|(_, started)| now - started < PING_SECONDS);
    let grid_size = map_data.grid_size;
    for (position, started) in &pings.pings {
        let age = (now - started) / PING_SECONDS;
        // Two rings, one after the other
        for delay in [0.0, 0.25] {
            let t = ((age - delay) / (1.0 - delay)).clamp(0.0, 1.0);
            if t <= 0.0 {
                continue;
            }
            let radius = grid_size * (0.2 + 1.3 * t);
            gizmos
                .circle_2d(
                    Isometry2d::from_translation(*position),
                    radius,
                    MARKING_COLOR.with_alpha(1.0 - t),
                )
                .resolution(48);
        }
    }
}

/// Shows the ruler's or template's size next to the mouse end of it: at a constant size in
/// the editor, and at a size on the map for players.
pub fn update_marking_labels(
    state: Res<MeasureState>,
    map_data: Res<MapData>,
    camera: Query<&CameraZoom, With<EditorCamera>>,
    mut labels: Query<(
        &MarkingLabel,
        &mut Text2d,
        &mut TextFont,
        &mut Transform,
        &mut Anchor,
        &mut Visibility,
    )>,
) {
    let grid_size = map_data.grid_size;
    let zoom = camera.single().map_or(1.0, |zoom| zoom.scale);
    for (label, mut text, mut font, mut transform, mut anchor, mut visibility) in &mut labels {
        let shown = state
            .marking
            .filter(|_| *label == MarkingLabel::Editor || state.show_players);
        let Some(marking) = shown else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        visibility.set_if_neq(Visibility::Inherited);
        let content = marking.label(grid_size);
        if text.0 != content {
            text.0 = content;
        }
        let (size, scale) = match label {
            MarkingLabel::Editor => (EDITOR_LABEL_SIZE, zoom),
            MarkingLabel::Player => (grid_size * PLAYER_LABEL_CELLS, 1.0),
        };
        if font.font_size != FontSize::Px(size) {
            font.font_size = FontSize::Px(size);
        }
        let (position, new_anchor) = marking.label_place(grid_size);
        anchor.set_if_neq(new_anchor);
        let new_transform = Transform::from_translation(position.extend(LABEL_Z))
            .with_scale(Vec3::new(scale, scale, 1.0));
        transform.set_if_neq(new_transform);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    const GRID: f32 = 70.0;

    fn marking(shape: MeasureShape, end_cells: Vec2) -> Marking {
        Marking {
            shape,
            start: Vec2::ZERO,
            end: end_cells * GRID,
        }
    }

    #[test]
    fn test_ruler_counts_a_diagonal_step_as_one_cell() {
        let ruler = marking(MeasureShape::Ruler, Vec2::new(4.0, 3.0));
        assert_eq!(ruler.cells(GRID), 4.0);
        assert_eq!(ruler.label(GRID), "20 ft");
    }

    #[test]
    fn test_template_sizes_round_to_whole_cells() {
        // 3-4-5 triangle: a 25-foot radius
        let circle = marking(MeasureShape::Circle, Vec2::new(3.1, 3.9));
        assert_eq!(circle.cells(GRID), 5.0);
        assert_eq!(circle.label(GRID), "25 ft radius");
        // Never smaller than a cell
        assert_eq!(
            marking(MeasureShape::Cone, Vec2::new(0.1, 0.0)).cells(GRID),
            1.0
        );
    }

    #[test]
    fn test_cone_is_as_wide_as_it_is_long() {
        let cone = marking(MeasureShape::Cone, Vec2::new(3.0, 0.0));
        let corners = cone.corners(GRID).unwrap();
        assert_eq!(corners[0], Vec2::ZERO);
        assert_eq!(corners[1].x, 3.0 * GRID);
        assert_eq!(corners[1].distance(corners[2]), 3.0 * GRID);
        assert_eq!(cone.label(GRID), "15 ft cone");
    }

    #[test]
    fn test_square_extends_toward_the_mouse() {
        let square = marking(MeasureShape::Square, Vec2::new(-2.0, 1.2));
        let corners = square.corners(GRID).unwrap();
        assert_eq!(corners[2], Vec2::new(-2.0, 2.0) * GRID);
    }

    #[test]
    fn test_labels_sit_outside_the_shape() {
        // Past the far end of a cone pointing right, running further right
        let cone = marking(MeasureShape::Cone, Vec2::new(3.0, 0.0));
        let (position, anchor) = cone.label_place(GRID);
        assert!(position.x > 3.0 * GRID);
        assert_eq!(anchor, Anchor::CENTER_LEFT);
        // Above a square that goes down from its start
        let square = marking(MeasureShape::Square, Vec2::new(2.0, -2.0));
        let (position, anchor) = square.label_place(GRID);
        assert_eq!(position.x, GRID);
        assert!(position.y > 0.0);
        assert_eq!(anchor, Anchor::BOTTOM_CENTER);
    }

    #[test]
    fn test_line_is_one_cell_wide() {
        let line = marking(MeasureShape::Line, Vec2::new(0.0, 6.0));
        let corners = line.corners(GRID).unwrap();
        assert_eq!(corners[0].distance(corners[3]), GRID);
        assert_eq!(corners[0].distance(corners[1]), 6.0 * GRID);
    }

    #[test]
    fn test_players_see_marks_whether_or_not_their_view_is_frozen() {
        let mut app = App::new();
        app.init_resource::<GizmoConfigStore>()
            .init_gizmo_group::<MarkingGizmoGroup>();
        app.world_mut()
            .run_system_once(configure_marking_gizmos)
            .unwrap();
        let store = app.world().resource::<GizmoConfigStore>();
        let (config, _) = store.config::<MarkingGizmoGroup>();
        for frozen in [false, true] {
            let player = crate::session::freeze::player_camera_layers(frozen);
            assert!(config.render_layers.intersects(&player));
        }
    }

    #[test]
    fn test_starts_snap_to_cells_or_corners() {
        let at = Vec2::new(80.0, 130.0);
        assert_eq!(
            snapped_start(MeasureShape::Ruler, at, GRID, true),
            Vec2::new(105.0, 105.0)
        );
        assert_eq!(
            snapped_start(MeasureShape::Circle, at, GRID, true),
            Vec2::new(70.0, 140.0)
        );
        assert_eq!(snapped_start(MeasureShape::Circle, at, GRID, false), at);
    }
}
