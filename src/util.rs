use chrono::Duration;
use eframe::egui::Color32;
use egui_extras::RetainedImage;
use log::warn;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "resources/"]
pub struct Resources;

/// Returns a `RetainedImage` from the data sourced from a local file.
pub fn get_retained_image(file_name: &str) -> RetainedImage {
    let resource = Resources::get(&format!("images/{}", file_name))
        .unwrap_or_else(|| {
            warn!("Missing image: {}", file_name);
            Resources::get("images/MissingImg.webp").unwrap()
        })
        .data;

    RetainedImage::from_image_bytes(file_name, resource.as_ref())
        .unwrap_or_else(|_| panic!("Cannot load image {}", file_name))
}

/// Returns human readable time of `Duration` supplied.
/// Times are "zero padded".
pub fn duration_to_string(dur: &Duration) -> String {
    let seconds = dur.num_seconds() % 60;
    let minutes = (dur.num_seconds() / 60) % 60;
    let hours = (dur.num_seconds() / 60) / 60;

    if hours <= 0 && minutes <= 0 {
        format!("{:0>2}s", seconds)
    } else if hours <= 0 {
        format!("{:0>2}m {:0>2}s", minutes, seconds)
    } else {
        format!("{:0>2}h {:0>2}m {:0>2}s", hours, minutes, seconds)
    }
}

/// Return background and border color based on the duration left.
///
/// https://yeun.github.io/open-color/ingredients.html
pub fn time_left_color(dur: &Duration) -> (Color32, Color32) {
    // let seconds = dur.num_seconds() % 60;
    let minutes = (dur.num_seconds() / 60) % 60;
    let hours = (dur.num_seconds() / 60) / 60;

    if hours == 0 && minutes < 10 {
        (
            Color32::from_rgb(255, 227, 227), // Red 1
            Color32::from_rgb(255, 168, 168), // Red 3
        )
    } else if hours == 0 && minutes < 20 {
        (
            Color32::from_rgb(255, 243, 191), // Yellow 1
            Color32::from_rgb(255, 224, 102), // Yellow 3
        )
    } else if hours == 0 && minutes < 40 {
        (
            Color32::from_rgb(211, 249, 216), // Green 1
            Color32::from_rgb(140, 233, 154), // Green 3
        )
    } else {
        (
            Color32::from_rgb(208, 235, 255), // Blue 1
            Color32::from_rgb(116, 192, 252), // Blue 3
        )
    }
}

pub fn split_pascal_case(value: &str) -> String {
    let mut idxs = vec![];
    let mut copy = value.to_string();
    // Find the positions of uppercase characters,
    // excluding the zero position.
    value.chars().enumerate().for_each(|(i, c)| {
        if i > 0 && c.is_ascii_uppercase() {
            idxs.push(i);
        }
    });
    // Add spaces to correct positions.
    idxs.iter().enumerate().for_each(|(i, k)| {
        copy.insert(*k + i, ' ');
    });
    // Return the finished string.
    copy
}
