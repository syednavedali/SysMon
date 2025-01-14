use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    Camera,
};
use chrono::Local;
use anyhow::Result;
use std::path::PathBuf;
use image::ImageBuffer;
use log::info;

pub fn capturecam_old(imagepath: &str) -> Result<()> {
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
    
    let path = PathBuf::from(&imagepath);

    // Save as PNG instead of JPEG
    img_buffer.save(path)?;
    info!("Image saved as: {}", imagepath);

    // Close camera stream
    camera.stop_stream()?;
    info!("Camera stream closed.");

    Ok(())
}