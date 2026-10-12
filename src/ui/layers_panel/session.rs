//! Live Session viewport controls UI.

use bevy_egui::egui;

use bevy::math::Vec2;

use crate::session::LiveSessionState;

/// Renders the Live Session controls section when a session is active. Returns true if
/// the user changed anything.
///
/// `move_while_frozen` is the setting that lets viewport moves reach players
/// while their view is frozen. `screen_width` is the player screen's physical width in
/// inches, if known (the caller saves it); `grid_size` is the map's.
pub fn render_session_controls(
    ui: &mut egui::Ui,
    session_state: &mut LiveSessionState,
    move_while_frozen: bool,
    screen_width: &mut Option<f32>,
    grid_size: f32,
) -> bool {
    if !session_state.is_active {
        return false;
    }
    let mut changed = false;

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Live Session").heading().size(18.0));
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    if let Some(ref monitor) = session_state.selected_monitor {
        ui.label(egui::RichText::new(format!("Monitor: {}", monitor.name)).size(13.0));
        ui.label(
            egui::RichText::new(format!(
                "{}x{}",
                monitor.physical_size.x, monitor.physical_size.y
            ))
            .size(13.0)
            .weak(),
        );
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        changed |= toggle_switch(ui, &mut session_state.blackout)
            .on_hover_text("Show players a black screen (Ctrl+B)")
            .changed();
        ui.label(egui::RichText::new("Blackout").size(14.0));
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        changed |= toggle_switch(ui, &mut session_state.frozen)
            .on_hover_text("Hold the player view while you edit the map")
            .changed();
        ui.label(egui::RichText::new("Freeze player view").size(14.0));
    });
    if session_state.frozen {
        let hint = if move_while_frozen {
            "Players see the map as it was when frozen. Viewport moves still reach them."
        } else {
            "Players see the map and view as they were when frozen."
        };
        ui.label(egui::RichText::new(hint).size(12.0).weak());
    }

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Position").size(14.0).strong());

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("X:").size(14.0));
        changed |= ui
            .add(
                egui::DragValue::new(&mut session_state.viewport_center.x)
                    .speed(5.0)
                    .suffix(" px"),
            )
            .changed();
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Y:").size(14.0));
        changed |= ui
            .add(
                egui::DragValue::new(&mut session_state.viewport_center.y)
                    .speed(5.0)
                    .suffix(" px"),
            )
            .changed();
    });

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Size").size(14.0).strong());

    // Width can be edited, height is locked to aspect ratio
    let aspect = session_state.monitor_aspect_ratio();
    let mut width = session_state.viewport_size.x;

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("W:").size(14.0));
        if ui
            .add(
                egui::DragValue::new(&mut width)
                    .speed(5.0)
                    .range(100.0..=10000.0)
                    .suffix(" px"),
            )
            .changed()
        {
            session_state.viewport_size.x = width;
            session_state.viewport_size.y = width / aspect;
            changed = true;
        }
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("H:").size(14.0));
        ui.label(
            egui::RichText::new(format!("{:.0} px", session_state.viewport_size.y)).size(14.0),
        );
    });

    // Table scale: one grid cell per inch on a screen laid flat, so miniatures fit
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Table scale").size(14.0).strong());
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Screen width:").size(13.0));
        let mut inches = screen_width.unwrap_or(0.0);
        if ui
            .add(
                egui::DragValue::new(&mut inches)
                    .range(0.0..=200.0)
                    .speed(0.1)
                    .suffix(" in"),
            )
            .on_hover_text("The visible width of the player screen, measured with a ruler")
            .changed()
        {
            *screen_width = (inches > 0.0).then_some(inches);
        }
    });
    if ui
        .add_enabled(screen_width.is_some(), egui::Button::new("1 cell = 1 inch"))
        .on_hover_text("Size the player view so each grid cell is one inch on that screen")
        .on_disabled_hover_text("Enter the screen's width first")
        .clicked()
        && let Some(inches) = *screen_width
    {
        session_state.viewport_size = table_scale_viewport(inches, grid_size, aspect);
        changed = true;
    }

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Rotation").size(14.0).strong());

    ui.horizontal(|ui| {
        if ui.add_sized([28.0, 26.0], egui::Button::new("↺")).clicked() {
            session_state.rotate_ccw();
            changed = true;
        }
        ui.label(egui::RichText::new(format!("{}°", session_state.rotation_degrees)).size(14.0));
        if ui.add_sized([28.0, 26.0], egui::Button::new("↻")).clicked() {
            session_state.rotate_cw();
            changed = true;
        }
    });

    ui.add_space(12.0);

    if ui
        .add_sized([140.0, 26.0], egui::Button::new("End Session"))
        .clicked()
    {
        session_state.is_active = false;
        changed = true;
    }
    changed
}

/// An on/off switch, like a checkbox without a label
fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        ui.painter().circle(
            egui::pos2(knob_x, rect.center().y),
            0.75 * radius,
            visuals.bg_fill,
            visuals.fg_stroke,
        );
    }

    response
}

/// The viewport (in the player screen's own frame) that shows one grid cell per inch on a
/// screen `inches` wide with `aspect` width/height
pub fn table_scale_viewport(inches: f32, grid_size: f32, aspect: f32) -> Vec2 {
    let width = inches * grid_size;
    Vec2::new(width, width / aspect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_scale_shows_one_cell_per_inch() {
        // A 42-inch-wide 16:9 TV shows 42 cells across
        let size = table_scale_viewport(42.0, 70.0, 16.0 / 9.0);
        assert_eq!(size.x / 70.0, 42.0);
        assert!((size.x / size.y - 16.0 / 9.0).abs() < 1e-4);
    }
}
