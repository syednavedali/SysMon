use winapi::um::wingdi;
use std::ptr::null_mut;
use winapi::um::winuser::SetThreadDesktop;
use winapi::um::winuser::CloseDesktop;
use winapi::um::winuser::DESKTOP_READOBJECTS;
use winapi::um::winuser::DESKTOP_CREATEWINDOW;
use winapi::um::winuser::DESKTOP_WRITEOBJECTS;
use winapi::um::winuser::DESKTOP_SWITCHDESKTOP;
use winapi::um::winuser::DESKTOP_ENUMERATE;
use winapi::um::winuser::OpenInputDesktop;
use std::io;
use image::{ImageBuffer, Rgba};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use std::mem::size_of;
use log::debug;

pub fn capture_screenshot(file_path: &str) -> io::Result<()> {
    // Start by trying to switch to interactive desktop
    if let Err(e) = switch_to_interactive_desktop() {
        eprintln!("Warning: Failed to switch to interactive desktop: {}. Will try capture anyway.", e);
    }

    // Attempt primary capture method first
    match capture_screen_gdi(file_path) {
        Ok(_) => {
            debug!("Successfully captured screen using GDI method");
            Ok(())
        }
        Err(primary_error) => {
            eprintln!("Primary capture method failed: {}", primary_error);
            // Fall back to secondary method
            match capture_screen_dc(file_path) {
                Ok(_) => {
                    debug!("Successfully captured screen using DC method");
                    Ok(())
                }
                Err(secondary_error) => {
                    eprintln!("Secondary capture method also failed: {}", secondary_error);
                    Err(io::Error::new(io::ErrorKind::Other,
                                       format!("All capture methods failed. Errors: Primary: {}, Secondary: {}",
                                               primary_error, secondary_error)))
                }
            }
        }
    }
}

fn switch_to_interactive_desktop() -> io::Result<()> {
    unsafe {
        // Open the input desktop with all necessary rights
        let desktop = OpenInputDesktop(
            0,
            0,
            DESKTOP_CREATEWINDOW | DESKTOP_READOBJECTS | DESKTOP_WRITEOBJECTS |
                DESKTOP_SWITCHDESKTOP | DESKTOP_ENUMERATE
        );

        if desktop == null_mut() {
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to open input desktop"));
        }

        // Ensure we clean up the desktop handle
        let _desktop_guard = scopeguard::guard(desktop, |h| {
            CloseDesktop(h);
        });

        // Try to switch to the desktop
        if !SetThreadDesktop(desktop) == 0 {
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to switch to input desktop"));
        }

        Ok(())
    }
}

fn capture_screen_gdi(file_path: &str) -> io::Result<()> {
    unsafe {
        // Get primary display dimensions
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);

        if screen_width <= 0 || screen_height <= 0 {
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to get screen dimensions"));
        }

        // Create device contexts
        let screen_dc = GetDC(HWND(0));
        if screen_dc.is_invalid() {
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to get screen DC"));
        }

        let memory_dc = CreateCompatibleDC(screen_dc);
        if memory_dc.is_invalid() {
            ReleaseDC(HWND(0), screen_dc);
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to create compatible DC"));
        }

        // Create compatible bitmap
        let bitmap = CreateCompatibleBitmap(screen_dc, screen_width, screen_height);
        if bitmap.is_invalid() {
            DeleteDC(memory_dc);
            ReleaseDC(HWND(0), screen_dc);
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to create compatible bitmap"));
        }

        // Select bitmap into memory DC
        let old_bitmap = SelectObject(memory_dc, bitmap);

        // Copy screen content
        if !BitBlt(
            memory_dc,
            0, 0,
            screen_width, screen_height,
            screen_dc,
            0, 0,
            SRCCOPY,
        ).as_bool() {
            SelectObject(memory_dc, old_bitmap);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(HWND(0), screen_dc);
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to copy screen content"));
        }

        // Get bitmap information
        let mut bitmap_info: BITMAPINFO = std::mem::zeroed();
        bitmap_info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bitmap_info.bmiHeader.biWidth = screen_width;
        bitmap_info.bmiHeader.biHeight = -screen_height; // Top-down
        bitmap_info.bmiHeader.biPlanes = 1;
        bitmap_info.bmiHeader.biBitCount = 32;
        bitmap_info.bmiHeader.biCompression = wingdi::BI_RGB as u32;

        // Allocate memory for pixel data
        let mut pixels = vec![0u8; (screen_width * screen_height * 4) as usize];

        // Get bitmap bits
        if GetDIBits(
            memory_dc,
            bitmap,
            0,
            screen_height as u32,
            Some(pixels.as_mut_ptr() as *mut std::ffi::c_void),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        ) == 0 {
            SelectObject(memory_dc, old_bitmap);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(HWND(0), screen_dc);
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to get bitmap data"));
        }

        // Create image buffer
        let mut image_buffer = ImageBuffer::new(screen_width as u32, screen_height as u32);

        // Copy pixels to image buffer
        for y in 0..screen_height {
            for x in 0..screen_width {
                let pos = ((y * screen_width + x) * 4) as usize;
                let pixel = Rgba([
                    pixels[pos + 2], // Red
                    pixels[pos + 1], // Green
                    pixels[pos + 0], // Blue
                    pixels[pos + 3], // Alpha
                ]);
                image_buffer.put_pixel(x as u32, y as u32, pixel);
            }
        }

        // Clean up GDI resources
        SelectObject(memory_dc, old_bitmap);
        DeleteObject(bitmap);
        DeleteDC(memory_dc);
        ReleaseDC(HWND(0), screen_dc);

        // Save image
        image_buffer.save(file_path).map_err(|e| {
            io::Error::new(io::ErrorKind::Other,
                           format!("Failed to save image: {}", e))
        })?;

        Ok(())
    }
}

fn capture_screen_dc(file_path: &str) -> io::Result<()> {
    unsafe {
        // Get desktop window handle
        let hwnd = GetDesktopWindow();

        // Get window dimensions
        let mut rect = RECT::default();
        if !GetWindowRect(hwnd, &mut rect).as_bool() {
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to get window dimensions"));
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        // Get device contexts
        let window_dc = GetWindowDC(hwnd);
        if window_dc.is_invalid() {
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to get window DC"));
        }

        let memory_dc = CreateCompatibleDC(window_dc);
        if memory_dc.is_invalid() {
            ReleaseDC(hwnd, window_dc);
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to create compatible DC"));
        }

        let bitmap = CreateCompatibleBitmap(window_dc, width, height);
        if bitmap.is_invalid() {
            DeleteDC(memory_dc);
            ReleaseDC(hwnd, window_dc);
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to create compatible bitmap"));
        }

        let old_bitmap = SelectObject(memory_dc, bitmap);

        // Copy screen content
        if !BitBlt(
            memory_dc,
            0, 0,
            width, height,
            window_dc,
            0, 0,
            SRCCOPY,
        ).as_bool() {
            SelectObject(memory_dc, old_bitmap);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(hwnd, window_dc);
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to copy screen content"));
        }

        // Set up bitmap info
        let mut bitmap_info: BITMAPINFO = std::mem::zeroed();
        bitmap_info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bitmap_info.bmiHeader.biWidth = width;
        bitmap_info.bmiHeader.biHeight = -height; // Top-down
        bitmap_info.bmiHeader.biPlanes = 1;
        bitmap_info.bmiHeader.biBitCount = 32;
        bitmap_info.bmiHeader.biCompression = wingdi::BI_RGB as u32;

        // Allocate memory for pixel data
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        // Get bitmap bits
        if GetDIBits(
            memory_dc,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut std::ffi::c_void),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        ) == 0 {
            SelectObject(memory_dc, old_bitmap);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(hwnd, window_dc);
            return Err(io::Error::new(io::ErrorKind::Other,
                                      "Failed to get bitmap data"));
        }

        // Create image buffer
        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        // Copy pixels to image buffer
        for y in 0..height {
            for x in 0..width {
                let pos = ((y * width + x) * 4) as usize;
                let pixel = Rgba([
                    pixels[pos + 2], // Red
                    pixels[pos + 1], // Green
                    pixels[pos + 0], // Blue
                    pixels[pos + 3], // Alpha
                ]);
                image_buffer.put_pixel(x as u32, y as u32, pixel);
            }
        }

        // Clean up
        SelectObject(memory_dc, old_bitmap);
        DeleteObject(bitmap);
        DeleteDC(memory_dc);
        ReleaseDC(hwnd, window_dc);

        // Save image
        image_buffer.save(file_path).map_err(|e| {
            io::Error::new(io::ErrorKind::Other,
                           format!("Failed to save image: {}", e))
        })?;

        Ok(())
    }
}