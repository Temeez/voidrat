use crate::parsers::FissureTier;
use crate::util::{duration_to_string, get_retained_image, time_left_color};
use crate::widgets::UiExt;
use crate::VoidRat;
use eframe::egui::style::WidgetVisuals;
use std::collections::HashMap;

use eframe::egui::{
    Align, CentralPanel, Color32, ColorImage, Context, Direction, Layout, Pos2, RichText, Rounding,
    ScrollArea, Separator, Stroke, TextStyle, Vec2, Widget, Window,
};
use egui_extras::{RetainedImage, Size, TableBuilder};

use crate::voidrat::play_notification_sound;
use chrono::Local;
use eframe::CreationContext;
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread;

const LOADING_FRAMES: [&str; 4] = ["Loading", "Loading.", "Loading..", "Loading..."];

#[derive(PartialEq, Clone)]
enum ActiveView {
    Fissure,
    VoidStorm,
    Invasion,
}

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
    missing: RetainedImage,
    lith: RetainedImage,
    meso: RetainedImage,
    neo: RetainedImage,
    axi: RetainedImage,
    requiem: RetainedImage,

    invasion: HashMap<String, RetainedImage>,
}

impl Default for Images {
    fn default() -> Self {
        Self::new()
    }
}

impl Images {
    fn new() -> Self {
        Images {
            missing: Self::dummy(),
            lith: Self::dummy(),
            meso: Self::dummy(),
            neo: Self::dummy(),
            axi: Self::dummy(),
            requiem: Self::dummy(),

            invasion: HashMap::new(),
        }
    }

    /// Return 1x1 px dummy `RetainedImage`.
    fn dummy() -> RetainedImage {
        RetainedImage::from_color_image("dummy", ColorImage::new([1, 1], Color32::BLACK))
    }

    pub fn get_invasion_img(&self, key: &str) -> &RetainedImage {
        return if let Some(img) = self.invasion.get(key) {
            img
        } else {
            self.invasion
                .iter()
                .find_map(|(k, v)| if key.contains(k) { Some(v) } else { None })
                .unwrap_or(&self.missing)
        };
    }
}

const INVASION_REWARDS: [&str; 15] = [
    "Fieldron",
    "Detonite Injector",
    "Mutagen Mass",
    "Infested Alad V Nav Coordinate",
    "Karak Wraith",
    "Latron Wraith",
    "Strun Wraith",
    "Twin Vipers Wraith",
    "Sheev",
    "Dera Vandal",
    "Snipetron Vandal",
    "Orokin Catalyst Blueprint",
    "Orokin Reactor Blueprint",
    "Forma Blueprint",
    "Exilus Warframe Adapter Blueprint",
];

pub struct UI {
    /// The actual app where UI get its data from.
    app: VoidRat,
    /// True after the data has been loaded.
    initialized: bool,
    /// Loading text to show before initializing is done.
    loading_text: AnimatedText,
    /// All the images UI uses.
    images: Arc<RwLock<Images>>,
    /// Currently active view.
    active_view: ActiveView,
    /// Render the notification window when true.
    show_notifications: bool,
    /// For checkbox state
    noti_fissure_void_capture: bool,
    /// For checkbox state
    noti_invasion_epic: bool,
}

impl UI {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Dummo images for now.
        let images = Arc::new(RwLock::new(Images::default()));

        // Load fissure images in a separate thread. Dummy images begone.
        let images_clone = images.clone();
        thread::spawn(move || {
            images_clone.write().missing = get_retained_image("MissingImg.webp");
            images_clone.write().lith = get_retained_image("VoidProjectionsIronD.webp");
            images_clone.write().meso = get_retained_image("VoidProjectionsBronzeD.webp");
            images_clone.write().neo = get_retained_image("VoidProjectionsSilverD.webp");
            images_clone.write().axi = get_retained_image("VoidProjectionsGoldD.webp");
            images_clone.write().requiem = get_retained_image("RequiemR0.webp");

            let mut invasion_imgs: HashMap<String, RetainedImage> = HashMap::new();
            for ir in INVASION_REWARDS {
                let img_name = format!("{}.webp", ir.replace(' ', ""));
                invasion_imgs.insert(ir.to_owned(), get_retained_image(&img_name));
            }

            images_clone.write().invasion = invasion_imgs;
        });

        let app = VoidRat::new();

        let data_clone = app.data.read().clone();

        ui_style(cc);

        UI {
            app,
            initialized: false,
            loading_text: AnimatedText::new(&LOADING_FRAMES),
            images,
            active_view: ActiveView::Fissure,
            show_notifications: false,
            noti_fissure_void_capture: data_clone.storage.noti_fissure_void_capture,
            noti_invasion_epic: data_clone.storage.noti_invasion_epic,
        }
    }

    /// Render all incomplete invasions.
    fn render_invasions(&mut self, ctx: &Context, ui: &mut eframe::egui::Ui) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                TableBuilder::new(ui)
                    .column(Size::exact(120.0))
                    .column(Size::exact(30.0))
                    .column(Size::exact(120.0))
                    .column(Size::exact(200.0))
                    .body(|mut body| {
                        for invasion in &self.app.data.read().invasions {
                            body.row(120.0, |mut row| {
                                ctx.request_repaint();

                                // 1st column.
                                row.col(|ui| {
                                    ui.horizontal(|ui| {
                                        for reward in &invasion.rewards.defender {
                                            ui.with_layout(
                                                Layout::from_main_dir_and_cross_align(
                                                    Direction::TopDown,
                                                    Align::Center,
                                                ),
                                                |ui| {
                                                    ui.image(
                                                        self.images
                                                            .read()
                                                            .get_invasion_img(&reward.item)
                                                            .texture_id(ctx),
                                                        self.images
                                                            .read()
                                                            .get_invasion_img(&reward.item)
                                                            .size_vec2()
                                                            * 0.5,
                                                    );
                                                    ui.label(&reward.to_string());
                                                },
                                            );
                                        }
                                    });
                                });
                                // 2nd column.
                                row.col(|ui| {
                                    if !invasion.rewards.attacker.is_empty()
                                        && !invasion.rewards.defender.is_empty()
                                    {
                                        ui.with_layout(
                                            Layout::top_down_justified(Align::Center),
                                            |ui| {
                                                ui.add_space(30.0);
                                                ui.separator();
                                            },
                                        );
                                        // ui.horizontal(|ui| {
                                        //     ui.separator();
                                        // });
                                    }
                                });
                                // 3rd column.
                                row.col(|ui| {
                                    for reward in &invasion.rewards.attacker {
                                        ui.horizontal(|ui| {
                                            ui.with_layout(
                                                Layout::from_main_dir_and_cross_align(
                                                    Direction::TopDown,
                                                    Align::Center,
                                                ),
                                                |ui| {
                                                    ui.image(
                                                        self.images
                                                            .read()
                                                            .get_invasion_img(&reward.item)
                                                            .texture_id(ctx),
                                                        self.images
                                                            .read()
                                                            .get_invasion_img(&reward.item)
                                                            .size_vec2()
                                                            * 0.5,
                                                    );
                                                    ui.label(&reward.to_string());
                                                },
                                            );
                                        });
                                    }
                                });
                                // 4nd column.
                                row.col(|ui| {
                                    ui.add_space(4.0);
                                    ui.label(&invasion.node.value);
                                    ui.add_space(4.0);
                                    ui.badge_frame(
                                        Color32::from_rgb(240, 240, 240),
                                        Color32::from_rgb(200, 200, 200),
                                        |ui| {
                                            ui.colored_label(
                                                Color32::BLACK,
                                                RichText::new(&duration_to_string(
                                                    &invasion.active_duration(),
                                                ))
                                                .text_style(TextStyle::Monospace),
                                            );
                                        },
                                    );
                                });
                            });
                        }
                    });
            });
    }

    /// Render the list of fissures or void storms, depending on `show_storm` boolean.
    fn render_fissures(&mut self, ctx: &Context, ui: &mut eframe::egui::Ui, show_storm: bool) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                TableBuilder::new(ui)
                    .column(Size::exact(90.0))
                    .column(Size::exact(160.0))
                    .column(Size::exact(220.0))
                    .body(|mut body| {
                        ctx.request_repaint();

                        for fissure in &self.app.data.read().fissures {
                            // Skip expired fissures.
                            // Skip storms or normal fissures.
                            if fissure.has_expired() || show_storm != fissure.is_storm {
                                continue;
                            }

                            body.row(80.0, |mut row| {
                                // Show fissure images.
                                // 1st column.
                                row.col(|ui| {
                                    let size_modifier = 0.75;
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
                                        let text = duration_to_string(&fissure.till_expired());

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
    fn render_top_menu(&mut self, ctx: &Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.toggled_button(&mut self.active_view, ActiveView::Fissure, "Fissures");
            ui.toggled_button(&mut self.active_view, ActiveView::VoidStorm, "Void Storms");
            ui.toggled_button(&mut self.active_view, ActiveView::Invasion, "Invasions");

            if ui.button("🔔").clicked() {
                self.show_notifications = !self.show_notifications;
            }

            ui.add_space(10.0);

            ctx.request_repaint();

            let cetus_text = if self.app.data.read().cetus_cycle.cetus_is_day() {
                "Cetus ☀" // Day
            } else {
                "Cetus 🌙" // Night
            };

            ui.heading(cetus_text);

            // Duration of the current cycle.
            let cetus_cycle = &self.app.data.read().cetus_cycle.cetus_till_cycle();
            // Badge text.
            let text = duration_to_string(cetus_cycle);
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

    fn render_notification_window(&mut self, ctx: &Context) {
        Window::new("Notifications")
            .default_width(330.0)
            .min_width(330.0)
            .fixed_pos(Pos2::new(60.0, 100.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Play audio notification");
                ui.add_space(8.0);
                ui.style_mut()
                    .text_styles
                    .get_mut(&TextStyle::Button)
                    .unwrap()
                    .size = 16.0;
                ui.checkbox(
                    &mut self.noti_fissure_void_capture,
                    "Fissure Void Capture spotted",
                );
                ui.checkbox(
                    &mut self.noti_invasion_epic,
                    "Invasion epic reward (Forma / Orokin x) spotted",
                );
                ui.add_space(8.0);
                if ui.button("▶ Test").clicked() {
                    thread::spawn(play_notification_sound);
                }
                ui.with_layout(
                    Layout::from_main_dir_and_cross_align(Direction::RightToLeft, Align::RIGHT),
                    |ui| {
                        if ui.button("Close").clicked() {
                            self.show_notifications = false;
                        }
                        if ui.button("Save").clicked() {
                            self.app.data.write().storage.save_notification(
                                self.noti_fissure_void_capture,
                                self.noti_invasion_epic,
                            );
                            self.show_notifications = false;
                        }
                    },
                )
            });
    }
}

impl eframe::App for UI {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if self.show_notifications {
            self.render_notification_window(ctx);
        }

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

                match self.active_view {
                    ActiveView::Fissure => self.render_fissures(ctx, ui, false),
                    ActiveView::VoidStorm => self.render_fissures(ctx, ui, true),
                    ActiveView::Invasion => self.render_invasions(ctx, ui),
                }
            });
        } else {
            // Loading text which should be visible for very short duration
            // during app startup.
            CentralPanel::default().show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(180.0);

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

    // style.text_styles.get_mut(&TextStyle::Body).unwrap().size = 16.0;

    // Save the new styles.
    cc.egui_ctx.set_style(style);
}
