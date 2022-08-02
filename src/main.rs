#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::ui::UI;
use crate::util::Resources;
use crate::voidrat::VoidRat;
use eframe::egui::Vec2;
use eframe::{IconData, NativeOptions};

mod parsers;
pub mod ui;
mod util;
pub mod voidrat;
pub mod widgets;

fn main() {
    setup_logging().expect("failed to initialize logging.");

    let icon = Resources::get("icons/voidrat.ico").unwrap().data;
    let options = NativeOptions {
        initial_window_size: Some(Vec2::new(510.0, 540.0)),
        min_window_size: Some(Vec2::new(510.0, 160.0)),
        max_window_size: Some(Vec2::new(510.0, 2000.0)),
        icon_data: Some(IconData {
            rgba: image::load_from_memory(icon.as_ref())
                .unwrap()
                .to_rgba8()
                .into_raw(),
            width: 256,
            height: 256,
        }),
        ..NativeOptions::default()
    };

    eframe::run_native("Voidrat", options, Box::new(|cc| Box::new(UI::new(cc))));
}

pub fn setup_logging() -> Result<(), fern::InitError> {
    // Use debug for when in debug mode, otherwise set info as minimum log level
    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;
    #[cfg(not(debug_assertions))]
    let log_level = log::LevelFilter::Info;

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .level_for("voidrat", log_level)
        .chain(std::io::stdout())
        .chain(fern::log_file("voidrat.log")?)
        .apply()?;

    Ok(())
}
