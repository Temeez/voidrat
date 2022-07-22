use eframe::egui::{
    Color32, NumExt, Response, Sense, Stroke, TextStyle, Ui, Widget, WidgetInfo, WidgetText,
    WidgetType,
};

/// Button/Label that has different style when `selected` is true or false.
pub struct ToggledButton {
    selected: bool,
    text: WidgetText,
}

impl ToggledButton {
    pub fn new(selected: bool, text: impl Into<WidgetText>) -> Self {
        Self {
            selected,
            text: text.into(),
        }
    }
}

impl Widget for ToggledButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { selected, text } = self;

        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding;

        let wrap_width = ui.available_width() - total_extra.x;
        let text = text.into_galley(ui, None, wrap_width, TextStyle::Button);

        let mut desired_size = total_extra + text.size();
        desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click());
        response.widget_info(|| WidgetInfo::selected(WidgetType::Button, selected, text.text()));

        if ui.is_rect_visible(response.rect) {
            let text_pos = ui
                .layout()
                .align_size_within_rect(text.size(), rect.shrink2(button_padding))
                .min;

            let mut visuals = *ui.style().visuals.widgets.style(&response);
            if selected {
                visuals.bg_fill = ui.style().visuals.selection.bg_fill;
                visuals.bg_stroke = ui.style().visuals.selection.stroke;
                visuals.fg_stroke = ui.style().visuals.selection.stroke;
            }

            if selected || response.hovered() || response.has_focus() {
                let rect = rect.expand(visuals.expansion);

                ui.painter()
                    .rect(rect, visuals.rounding, visuals.bg_fill, visuals.bg_stroke);
            } else {
                ui.painter().rect(
                    rect,
                    visuals.rounding,
                    Color32::LIGHT_GRAY,
                    Stroke {
                        width: 1.0,
                        color: Color32::GRAY,
                    },
                );
            }

            text.paint_with_visuals(ui.painter(), text_pos, &visuals);
        }

        response
    }
}
