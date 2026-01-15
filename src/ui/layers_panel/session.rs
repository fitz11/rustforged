//! Live Session viewport controls UI.

use bevy_egui::egui;

use crate::session::LiveSessionState;

/// Renders the Live Session controls section when a session is active.
pub fn render_session_controls(ui: &mut egui::Ui, session_state: &mut LiveSessionState) {
    if !session_state.is_active {
        return;
    }

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
    ui.label(egui::RichText::new("Position").size(14.0).strong());

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("X:").size(14.0));
        ui.add(
            egui::DragValue::new(&mut session_state.viewport_center.x)
                .speed(5.0)
                .suffix(" px"),
        );
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Y:").size(14.0));
        ui.add(
            egui::DragValue::new(&mut session_state.viewport_center.y)
                .speed(5.0)
                .suffix(" px"),
        );
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
        }
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("H:").size(14.0));
        ui.label(
            egui::RichText::new(format!("{:.0} px", session_state.viewport_size.y)).size(14.0),
        );
    });

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Rotation").size(14.0).strong());

    ui.horizontal(|ui| {
        if ui.add_sized([28.0, 26.0], egui::Button::new("↺")).clicked() {
            session_state.rotate_ccw();
        }
        ui.label(
            egui::RichText::new(format!("{}°", session_state.rotation_degrees)).size(14.0),
        );
        if ui.add_sized([28.0, 26.0], egui::Button::new("↻")).clicked() {
            session_state.rotate_cw();
        }
    });

    ui.add_space(12.0);

    if ui
        .add_sized([140.0, 26.0], egui::Button::new("End Session"))
        .clicked()
    {
        session_state.is_active = false;
    }
}
