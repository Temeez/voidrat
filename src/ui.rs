use crate::parsers::FissureTier;
use crate::util::{get_retained_image, time_left_color, time_left_text};
use crate::widgets::UiExt;
use crate::VoidRat;
use eframe::egui::style::WidgetVisuals;

use eframe::egui::{
    CentralPanel, Color32, ColorImage, Context, RichText, Rounding, ScrollArea, Separator, Stroke,
    TextStyle, Vec2, Widget,
};
use egui_extras::{RetainedImage, Size, TableBuilder};

use chrono::Local;
use eframe::CreationContext;
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread;

const LOADING_FRAMES: [&str; 4] = ["Loading", "Loading.", "Loading..", "Loading..."];

/// Crude animated text thing that shows one "frame" every 250ms.
struct AnimatedText {
    frames: Vec<String>,
    current_frame: usize,
    time: i64,
}

impl AnimatedText {
    pub fn new(frames: &[&str]) -> Self {
        AnimatedText {
            frames: frames
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
            current_frame: 0,
            time: Local::now().timestamp_millis(),
        }
    }

    /// Returns the correct frame that should be rendered.
    pub fn animate(&mut self) -> &str {
        // Set the current frame to 0 if it is too large.
        if self.current_frame >= self.frames.len() {
            self.current_frame = 0;
        }

        // Get the correct text that should be returned.
        let text = self.frames.get(self.current_frame).unwrap().as_ref();

        // Increment current frame and update the time value.
        if Local::now().timestamp_millis() >= self.time + 250 {
            self.current_frame += 1;
            self.time = Local::now().timestamp_millis();
        }

        // Return text.
        text
    }
}

/// Images for the UI.
struct Images {
    lith: RetainedImage,
    meso: RetainedImage,
    neo: RetainedImage,
    axi: RetainedImage,
    requiem: RetainedImage,
}

impl Default for Images {
    fn default() -> Self {
        Self::new()
    }
}

impl Images {
    fn new() -> Self {
        Images {
            lith: Self::dummy(),
            meso: Self::dummy(),
            neo: Self::dummy(),
            axi: Self::dummy(),
            requiem: Self::dummy(),
        }
    }

    /// Return 1x1 px dummy `RetainedImage`.
    fn dummy() -> RetainedImage {
        RetainedImage::from_color_image("dummy", ColorImage::new([1, 1], Color32::BLACK))
    }
}

pub struct UI {
    /// The actual app where UI get its data from.
    app: VoidRat,
    /// True after the data has been loaded.
    initialized: bool,
    /// Loading text to show before initializing is done.
    loading_text: AnimatedText,
    /// All the images UI uses.
    images: Arc<RwLock<Images>>,
    /// False -> Fissures, True -> Void Storms.
    show_storm: bool,
}

impl UI {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Dummo images for now.
        let images = Arc::new(RwLock::new(Images::default()));

        // Load fissure images in a separate thread. Dummy images begone.
        let images_clone = images.clone();
        thread::spawn(move || {
            images_clone.write().lith = get_retained_image("VoidProjectionsIronD.webp");
            images_clone.write().meso = get_retained_image("VoidProjectionsBronzeD.webp");
            images_clone.write().neo = get_retained_image("VoidProjectionsSilverD.webp");
            images_clone.write().axi = get_retained_image("VoidProjectionsGoldD.webp");
            images_clone.write().requiem = get_retained_image("RequiemR0.webp");
        });

        let app = VoidRat::new();

        ui_style(cc);

        UI {
            app,
            initialized: false,
            loading_text: AnimatedText::new(&LOADING_FRAMES),
            images,
            show_storm: false,
        }
    }

    /// Render the list of fissures or void storms, depending on `show_storm` boolean.
    fn render_fissures(&mut self, ctx: &Context, ui: &mut eframe::egui::Ui, show_storm: bool) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                TableBuilder::new(ui)
                    .column(Size::exact(90.0))
                    .column(Size::initial(160.0).at_least(160.0))
                    .column(Size::initial(110.0).at_least(110.0))
                    .body(|mut body| {
                        ctx.request_repaint();

                        for fissure in &self.app.data.read().fissures {
                            // Skip storms or normal fissures.
                            if fissure.is_storm != show_storm {
                                continue;
                            }

                            body.row(80.0, |mut row| {
                                // Show fissure images.
                                // 1st column.
                                row.col(|ui| {
                                    let size_modifier = 0.2;
                                    match fissure.tier {
                                        FissureTier::Lith => ui.image(
                                            self.images.read().lith.texture_id(ctx),
                                            self.images.read().lith.size_vec2() * size_modifier,
                                        ),
                                        FissureTier::Meso => ui.image(
                                            self.images.read().meso.texture_id(ctx),
                                            self.images.read().meso.size_vec2() * size_modifier,
                                        ),
                                        FissureTier::Neo => ui.image(
                                            self.images.read().neo.texture_id(ctx),
                                            self.images.read().neo.size_vec2() * size_modifier,
                                        ),
                                        FissureTier::Axi => ui.image(
                                            self.images.read().axi.texture_id(ctx),
                                            self.images.read().axi.size_vec2() * size_modifier,
                                        ),
                                        FissureTier::Requiem => ui.image(
                                            self.images.read().requiem.texture_id(ctx),
                                            self.images.read().requiem.size_vec2() * size_modifier,
                                        ),
                                        _ => ui.label("Unknown"),
                                    };
                                });

                                // Basic fissure data.
                                // 2nd column.
                                row.col(|ui| {
                                    let text_color_override = if fissure.has_expired() {
                                        Some(Color32::GRAY)
                                    } else {
                                        None
                                    };

                                    // Override text color for expired fissures.
                                    ui.style_mut().visuals.override_text_color =
                                        text_color_override;

                                    ui.vertical(|ui| {
                                        ui.add_space(8.0);
                                        ui.heading(&fissure.tier.to_string());
                                        ui.label(&fissure.mission);
                                        ui.label(&fissure.node.value);
                                    });
                                });

                                // Countdowns
                                // 3rd column.
                                row.col(|ui| {
                                    if fissure.has_expired() {
                                        ui.grid_badge_frame(
                                            Color32::from_rgb(42, 42, 42),
                                            Color32::BLACK,
                                            |ui| {
                                                ui.colored_label(
                                                    Color32::from_rgb(250, 250, 250),
                                                    RichText::new("Expired")
                                                        .text_style(TextStyle::Monospace),
                                                );
                                            },
                                        );
                                    } else {
                                        // Figure out the correct badge background color.
                                        // For Void Capture missions only show violet.
                                        let (bg_color, border_color) = if fissure.node.value
                                            == *"Hepit (Void)"
                                            || fissure.node.value == *"Ukko (Void)"
                                        {
                                            (
                                                Color32::from_rgb(229, 219, 255), // Violet 1
                                                Color32::from_rgb(177, 151, 252), // Violet 3
                                            )
                                        } else {
                                            time_left_color(&fissure.till_expired())
                                        };
                                        // Time left in human readable format
                                        let text = time_left_text(&fissure.till_expired());

                                        ui.grid_badge_frame(bg_color, border_color, |ui| {
                                            ui.colored_label(
                                                Color32::BLACK,
                                                RichText::new(text)
                                                    .text_style(TextStyle::Monospace),
                                            );
                                        });
                                    }
                                });
                            });
                        }
                    });
            });
    }

    /// Render the top menu which has the buttons for displaying either fissures or void storms
    /// and shows the current day/night cycle of Cetus.
    fn render_top_menu(&mut self, _ctx: &Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.toggled_button(&mut self.show_storm, false, "Fissures");
            ui.toggled_button(&mut self.show_storm, true, "Void Storms");

            ui.add_space(50.0);

            let cetus_text = if self.app.data.read().cetus_cycle.cetus_is_day() {
                "Cetus ☀" // Day
            } else {
                "Cetus 🌙" // Night
            };

            ui.heading(cetus_text);

            // Duration of the current cycle.
            let cetus_cycle = &self.app.data.read().cetus_cycle.cetus_till_cycle();
            // Badge text.
            let text = time_left_text(cetus_cycle);
            // Badge fill and border color.
            let (bg_color, border_color) = time_left_color(cetus_cycle);

            if cetus_cycle.num_seconds() > 0 {
                // Current cycle is ongoing.
                ui.badge_frame(bg_color, border_color, |ui| {
                    ui.colored_label(
                        Color32::BLACK,
                        RichText::new(text).text_style(TextStyle::Monospace),
                    );
                });
            } else {
                // Current cycle has expired.
                ui.badge_frame(Color32::from_rgb(42, 42, 42), Color32::BLACK, |ui| {
                    ui.colored_label(
                        Color32::from_rgb(250, 250, 250),
                        RichText::new("Expired").text_style(TextStyle::Monospace),
                    );
                });
            }
        });
    }
}

impl eframe::App for UI {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Not sure if this is less taxing down the line..
        if !self.initialized && self.app.data.read().initialized {
            self.initialized = true;
        }

        // instead of using `self.app.data.read()..` every update, forever.
        if self.initialized {
            CentralPanel::default().show(ctx, |ui| {
                ui.add_space(8.0);

                self.render_top_menu(ctx, ui);

                Separator::default().spacing(24.0).horizontal().ui(ui);

                self.render_fissures(ctx, ui, self.show_storm);
            });
        } else {
            // Loading text which should be visible for very short duration
            // during app startup.
            CentralPanel::default().show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(150.0);

                    ctx.request_repaint();

                    let loading = RichText::from(self.loading_text.animate()).size(32.0);

                    ui.heading(loading);
                });
            });
        }
    }
}

/// Custom styles for the UI.
fn ui_style(cc: &CreationContext) {
    let mut style = (*cc.egui_ctx.style()).clone();

    let base = WidgetVisuals {
        bg_fill: Color32::WHITE,
        bg_stroke: Stroke {
            width: 1.0,
            color: Color32::BLACK,
        },
        rounding: Rounding::none(),
        expansion: 0.0,
        fg_stroke: Stroke {
            width: 0.0,
            color: Color32::BLACK,
        },
    };

    style.visuals.widgets.noninteractive = WidgetVisuals { ..base };

    // Styles that the toggle button uses.
    style.visuals.selection.bg_fill = Color32::LIGHT_GREEN;
    style.visuals.selection.stroke = Stroke {
        width: 1.0,
        color: Color32::DARK_GREEN,
    };

    // Scrollbar bg color.
    style.visuals.extreme_bg_color = Color32::from_rgb(244, 244, 244);

    // Padding for the buttons.
    style.spacing.button_padding = Vec2::new(12.0, 8.0);

    // Monospace = badge text
    style
        .text_styles
        .get_mut(&TextStyle::Monospace)
        .unwrap()
        .size = 16.0;

    // Save the new styles.
    cc.egui_ctx.set_style(style);
}
