#[cfg(target_os = "linux")]
extern crate winres;
#[cfg(target_os = "windows")]
extern crate winres;

#[cfg(target_os = "linux")]
use std::{env, io};

#[cfg(target_os = "linux")]
fn main() -> io::Result<()> {
    let target_family = env::var("CARGO_CFG_TARGET_FAMILY").unwrap();
    if target_family == "windows" {
        winres::WindowsResource::new()
            .set_toolkit_path("/usr/bin")
            .set_windres_path("windres")
            .set_ar_path("ar")
            .set_icon("resources/icons/voidrat.ico")
            .compile()?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon_with_id("resources/icons/voidrat.ico", "32512");
    res.compile().unwrap();
}
