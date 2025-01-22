use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    Camera,
};
use chrono::Local;
use anyhow::{Result, Context};
use image::ImageBuffer;
use log::info;
use crate::img::imgutil::SecureFolder;

pub fn capturecam(secure_folder: &SecureFolder) -> Result<()> {
    // Initialize camera with default settings
    let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
    let mut camera = Camera::new(
        CameraIndex::Index(0), // Use first available camera
        requested,
    )?;

    // Get camera resolution
    let resolution = camera.resolution();
    info!("Camera resolution: {}x{}", resolution.width(), resolution.height());

    // Open camera stream
    camera.open_stream()?;
    info!("Camera opened successfully!");

    // Capture image and decode to RGB
    let frame = camera.frame()?;
    let decoded = frame.decode_image::<RgbFormat>()?;

    // Convert to image::RgbImage
    let img_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>> = ImageBuffer::from_raw(
        resolution.width() as u32,
        resolution.height() as u32,
        decoded.to_vec(),
    ).ok_or(anyhow::anyhow!("Failed to create image buffer"))?;

    // Generate timestamp for filename
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("capture_{}.png", timestamp);

    // Convert image to bytes
    let mut png_bytes: Vec<u8> = Vec::new();
    img_buffer.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageOutputFormat::Png)?;

    // Store in secure folder
    secure_folder.store_file(&filename, &png_bytes)
        .context("Failed to store image in secure folder")?;

    info!("Image saved securely as: {}", filename);

    // Close camera stream
    camera.stop_stream()?;
    info!("Camera stream closed.");

    Ok(())
}

