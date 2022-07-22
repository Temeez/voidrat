use crate::widgets::toggle_button::ToggledButton;
use eframe::egui::style::Margin;
use eframe::egui::{
    Color32, Frame, InnerResponse, Label, Response, RichText, Rounding, Stroke, Ui, Vec2, Widget,
    WidgetText,
};

pub mod colored_label;
mod toggle_button;

pub trait UiExt {
    /// Show text on a colored background, like a badge.
    ///
    /// Shortcut for `ui.label(RichText::new(text).color(color))`
    fn badge_label(
        &mut self,
        color: impl Into<Color32>,
        bg_color: impl Into<Color32>,
        text: impl Into<RichText>,
    ) -> Response;

    fn grid_badge_frame<R>(
        &mut self,
        fill_color: impl Into<Color32>,
        border_color: impl Into<Color32>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R>;

    fn badge_frame<R>(
        &mut self,
        fill_color: impl Into<Color32>,
        border_color: impl Into<Color32>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R>;

    fn toggled_button<Value: PartialEq>(
        &mut self,
        current_value: &mut Value,
        selected_value: Value,
        text: impl Into<WidgetText>,
    ) -> Response;
}

///
///
///
impl UiExt for eframe::egui::Ui {
    fn badge_label(
        &mut self,
        color: impl Into<Color32>,
        bg_color: impl Into<Color32>,
        text: impl Into<RichText>,
    ) -> Response {
        Label::new(text.into().color(color).background_color(bg_color)).ui(self)
    }

    fn grid_badge_frame<R>(
        &mut self,
        fill_color: impl Into<Color32>,
        border_color: impl Into<Color32>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        Frame::none()
            .fill(fill_color.into())
            .outer_margin(Margin::from(Vec2::new(0.0, 24.0)))
            .inner_margin(Margin::from(Vec2::new(8.0, 6.0)))
            .rounding(Rounding::from(3.0))
            .stroke(Stroke {
                width: 1.0,
                color: border_color.into(),
            })
            .show(self, add_contents)
    }

    fn badge_frame<R>(
        &mut self,
        fill_color: impl Into<Color32>,
        border_color: impl Into<Color32>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        Frame::none()
            .fill(fill_color.into())
            .outer_margin(Margin::from(Vec2::new(0.0, 0.0)))
            .inner_margin(Margin::from(Vec2::new(8.0, 6.0)))
            .rounding(Rounding::from(3.0))
            .stroke(Stroke {
                width: 1.0,
                color: border_color.into(),
            })
            .show(self, add_contents)
    }

    fn toggled_button<Value: PartialEq>(
        &mut self,
        current_value: &mut Value,
        selected_value: Value,
        text: impl Into<WidgetText>,
    ) -> Response {
        let mut response = ToggledButton::new(*current_value == selected_value, text).ui(self);
        if response.clicked() {
            *current_value = selected_value;
            response.mark_changed();
        }
        response
    }
}
