//! Taking what was dropped on a widget, by kind.
//!
//! egui's `Response::dnd_release_payload` takes whatever is carried and
//! throws it away when it is not of the kind asked for, so a widget that
//! asks for one kind and then another loses the second. This asks only
//! for what is really carried.

use std::any::Any;
use std::sync::Arc;

use eframe::egui;

/// What of kind `Payload` was released over `response` this frame, if
/// that is what was carried; anything else stays for the next question.
pub(crate) fn released<Payload: Any + Send + Sync>(
    response: &egui::Response,
) -> Option<Arc<Payload>> {
    if egui::DragAndDrop::has_payload_of_type::<Payload>(&response.ctx) {
        response.dnd_release_payload::<Payload>()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Image;
    #[derive(Debug, PartialEq)]
    struct Script;

    const SPOT: egui::Pos2 = egui::pos2(20.0, 20.0);

    fn target(ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_exact_size(egui::vec2(100.0, 100.0), egui::Sense::hover())
            .1
    }

    /// One frame where the pointer is released at `SPOT`, over a widget
    /// that asks first for an image and then for a script.
    fn drop_script() -> (Option<Arc<Image>>, Option<Arc<Script>>) {
        let ctx = egui::Context::default();
        let press = egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(SPOT),
                egui::Event::PointerButton {
                    pos: SPOT,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        // egui finds the widget under the pointer among those of the
        // frame before, so it is laid out in both. Nothing is drawn, so
        // the font textures are never uploaded.
        ctx.run_ui(press, |ui| {
            target(ui);
        })
        .textures_delta
        .clear();
        egui::DragAndDrop::set_payload(&ctx, Script);
        let release = egui::RawInput {
            events: vec![egui::Event::PointerButton {
                pos: SPOT,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut taken = (None, None);
        ctx.run_ui(release, |ui| {
            let response = target(ui);
            taken = (released::<Image>(&response), released::<Script>(&response));
        })
        .textures_delta
        .clear();
        taken
    }

    #[test]
    fn asking_for_another_kind_first_keeps_what_was_dropped() {
        let (image, script) = drop_script();

        assert_eq!(image, None);
        assert_eq!(script.as_deref(), Some(&Script));
    }
}
