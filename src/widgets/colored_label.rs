// use eframe::egui;
// use eframe::egui::{Color32, Label, Stroke};
//
// pub enum LedStatus {
//     White,
//     Red,
//     Yellow,
//     Green,
//     Black,
// }
//
// impl LedStatus {
//     fn color(&self) -> Color32 {
//         match self {
//             LedStatus::White => Color32::WHITE,
//             LedStatus::Red => Color32::LIGHT_RED,
//             LedStatus::Yellow => Color32::YELLOW,
//             LedStatus::Green => Color32::LIGHT_GREEN,
//             LedStatus::Black => Color32::BLACK,
//         }
//     }
// }
//
// impl Label {
//     pub fn colored_layout_in_ui(
//         &self,
//         ui: &mut egui::Ui,
//         status: &mut LedStatus,
//     ) -> egui::Response {
//         // Widget code can be broken up in four steps:
//         //  1. Decide a size for the widget
//         //  2. Allocate space for it
//         //  3. Handle interactions with the widget (if any)
//         //  4. Paint the widget
//
//         // 1. Deciding widget size:
//         // You can query the `ui` how much space is available,
//         // but in this example we have a fixed size widget based on the height of a standard button:
//         let desired_size = ui.spacing().interact_size.y * egui::vec2(1.0, 1.0);
//
//         // 2. Allocating space:
//         // This is where we get a region of the screen assigned.
//         // We also tell the Ui to sense clicks in the allocated region.
//         let (rect, response) =
//             ui.allocate_exact_size(desired_size, egui::Sense::focusable_noninteractive());
//
//         // 4. Paint!
//         // Make sure we need to paint:
//         if ui.is_rect_visible(rect) {
//             let visuals = ui.style().noninteractive();
//             // All coordinates are in absolute screen coordinates so we use `rect` to place the elements.
//             let rect = rect.expand(visuals.expansion);
//             let radius = 0.5 * rect.height();
//             // Paint the circle.
//             let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), 1.0);
//             let center = egui::pos2(circle_x, rect.center().y);
//             ui.painter().circle(
//                 center,
//                 0.5 * radius,
//                 status.color(),
//                 Stroke::new(0.5, Color32::BLACK),
//             );
//         }
//
//         response
//     }
// }
