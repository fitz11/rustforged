//! Docked panel layout shared across egui UI systems.
//!
//! Since egui 0.34, panels must be shown inside a parent [`egui::Ui`] rather than directly
//! on the [`egui::Context`], and bevy_egui does not expose the root `Ui` of each pass. Our
//! panels are drawn by separate Bevy systems, so each one is given a parent `Ui` covering
//! the space left over by the panels shown before it in the current pass. That leftover
//! space is the map canvas.
//!
//! egui's own [`egui::Context::is_pointer_over_egui`] only knows about panels placed in its
//! root `Ui`, so it reports `false` over our panels. Use [`is_pointer_over_ui`] instead to
//! gate editor input.

use bevy_egui::egui;

/// Space left for the map canvas after the panels shown so far in a pass.
#[derive(Clone, Copy)]
struct PanelLayout {
    pass_nr: u64,
    panels_shown: usize,
    canvas: egui::Rect,
}

fn layout_id() -> egui::Id {
    egui::Id::new("rustforged_panel_layout")
}

/// Extension for showing an [`egui::Panel`] docked against the panels shown before it.
pub trait DockedPanel {
    /// Show the panel at the edge of the space not yet taken by other panels this pass.
    fn show_docked<R>(
        self,
        ctx: &egui::Context,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R>;
}

impl DockedPanel for egui::Panel {
    fn show_docked<R>(
        self,
        ctx: &egui::Context,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        let pass_nr = ctx.cumulative_pass_nr();
        let layout = ctx
            .data(|data| data.get_temp::<PanelLayout>(layout_id()))
            .filter(|layout| layout.pass_nr == pass_nr)
            .unwrap_or(PanelLayout {
                pass_nr,
                panels_shown: 0,
                canvas: ctx.content_rect(),
            });

        let mut parent = egui::Ui::new(
            ctx.clone(),
            layout_id().with(layout.panels_shown),
            egui::UiBuilder::new()
                .layer_id(egui::LayerId::background())
                .max_rect(layout.canvas),
        );
        let response = self.show(&mut parent, add_contents);

        ctx.data_mut(|data| {
            data.insert_temp(
                layout_id(),
                PanelLayout {
                    pass_nr,
                    panels_shown: layout.panels_shown + 1,
                    canvas: parent.available_rect_before_wrap(),
                },
            )
        });
        response
    }
}

/// The map canvas: the part of the window the panels left in the last pass, in egui
/// points (the window's logical pixels).
pub fn canvas_rect(ctx: &egui::Context) -> Option<egui::Rect> {
    ctx.data(|data| data.get_temp::<PanelLayout>(layout_id()))
        .map(|layout| layout.canvas)
}

/// Returns true if the pointer is over any egui UI: a docked panel, window, or area.
pub fn is_pointer_over_ui(ctx: &egui::Context) -> bool {
    if ctx.is_pointer_over_egui() {
        return true;
    }
    let Some(pointer) = ctx.pointer_interact_pos() else {
        return false;
    };
    ctx.data(|data| data.get_temp::<PanelLayout>(layout_id()))
        .is_some_and(|layout| !layout.canvas.contains(pointer))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs one egui pass the way bevy_egui does: the root `Ui` from `run_ui` is left
    /// unused and each panel is shown by separate code.
    fn run_pass(ctx: &egui::Context, pointer: egui::Pos2) -> (egui::Rect, egui::Rect) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 800.0),
            )),
            events: vec![egui::Event::PointerMoved(pointer)],
            ..Default::default()
        };
        let mut rects = (egui::Rect::NOTHING, egui::Rect::NOTHING);
        let mut output = ctx.run_ui(input, |_| {
            let left = egui::Panel::left("left").show_docked(ctx, |ui| ui.label("left"));
            let top = egui::Panel::top("top").show_docked(ctx, |ui| ui.label("top"));
            rects = (left.response.rect, top.response.rect);
        });
        output.textures_delta.clear();
        rects
    }

    #[test]
    fn test_panels_dock_against_each_other() {
        let ctx = egui::Context::default();
        let (left, top) = run_pass(&ctx, egui::pos2(500.0, 400.0));
        assert_eq!(left.min, egui::Pos2::ZERO);
        assert_eq!(top.min, egui::pos2(left.max.x, 0.0));
        assert_eq!(top.max.x, 1000.0);
    }

    #[test]
    fn test_pointer_over_docked_panel_is_over_ui() {
        let ctx = egui::Context::default();
        let (left, top) = run_pass(&ctx, egui::Pos2::ZERO);
        for pointer in [left.center(), top.center()] {
            run_pass(&ctx, pointer);
            assert!(is_pointer_over_ui(&ctx), "pointer at {pointer:?}");
        }
    }

    #[test]
    fn test_pointer_over_canvas_is_not_over_ui() {
        let ctx = egui::Context::default();
        run_pass(&ctx, egui::pos2(600.0, 500.0));
        assert!(!is_pointer_over_ui(&ctx));
    }
}
