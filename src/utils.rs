use std::{fs::File, io::Read};

use image::{ImageBuffer, Rgba};
use winit::window::Icon;

pub fn load_icon(path: &str) -> Icon {
    let img = image::open(path).expect("error opening image").to_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();
    Icon::from_rgba(rgba, width, height).expect("error convert image to rgba")
}

// Color correction. needed for web browser
pub fn linear_to_srgb(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

pub fn read_file_to_bytes(path: &str) -> Vec<u8> {
    //let img = image::open(path).expect("error opening image").to_rgba8();
    //img.into_raw()

    let mut file = File::open(path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");
    buffer
}
