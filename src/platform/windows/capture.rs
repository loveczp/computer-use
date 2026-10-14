use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use std::io::Cursor;
use windows::core::{link, BOOL};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::desktop::ensure_user_desktop;
use crate::types::Screenshot;

link!("user32.dll" "system" fn PrintWindow(hwnd: HWND, hdc: HDC, nflags: u32) -> BOOL);

/// Capture a screenshot of a window.
pub fn capture_window(
    hwnd: HWND,
    screenshot_id: &str,
) -> Result<Screenshot, Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;

    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err("Invalid window handle".into());
        }

        // Restore if minimized
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            std::thread::sleep(std::time::Duration::from_millis(150));
        }

        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect)?;
        let width = (rect.right - rect.left).max(1);
        let height = (rect.bottom - rect.top).max(1);

        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let hbm = CreateCompatibleBitmap(hdc_screen, width, height);
        let old_hbm = SelectObject(hdc_mem, hbm.into());

        // Attempt 1: PrintWindow with PW_RENDERFULLCONTENT (flag = 2)
        let pw_res = PrintWindow(hwnd, hdc_mem, 2);

        // Attempt 2: If PrintWindow fails, fallback to BitBlt from screen DC
        if !pw_res.as_bool() {
            let _ = BitBlt(
                hdc_mem,
                0,
                0,
                width,
                height,
                Some(hdc_screen),
                rect.left,
                rect.top,
                SRCCOPY | CAPTUREBLT,
            );
        }

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down bitmap
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        GetDIBits(
            hdc_mem,
            hbm,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Clean up GDI resources
        SelectObject(hdc_mem, old_hbm);
        let _ = DeleteObject(hbm.into());
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);

        // Swap B and R for RGBA format
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
            chunk[3] = 255; // Full opacity
        }

        let img = image::RgbaImage::from_raw(width as u32, height as u32, pixels)
            .ok_or("Failed to construct image buffer from captured pixels")?;

        let mut png_bytes = Cursor::new(Vec::new());
        img.write_to(&mut png_bytes, image::ImageFormat::Png)?;

        let base64_str = BASE64_STANDARD.encode(png_bytes.get_ref());
        let data_url = format!("data:image/png;base64,{}", base64_str);

        Ok(Screenshot {
            id: screenshot_id.to_string(),
            url: data_url,
            width: Some(width as u32),
            height: Some(height as u32),
            origin_x: Some(rect.left),
            origin_y: Some(rect.top),
            z_index: 0,
        })
    }
}

/// Capture and save window screenshot directly to a file.
pub fn capture_window_to_file(
    hwnd: HWND,
    output_path: &str,
) -> Result<(u32, u32), Box<dyn std::error::Error + Send + Sync>> {
    let shot = capture_window(hwnd, "screenshot-file")?;
    // Strip "data:image/png;base64," prefix
    if let Some(pos) = shot.url.find(',') {
        let base64_data = &shot.url[pos + 1..];
        let bytes = BASE64_STANDARD.decode(base64_data)?;
        std::fs::write(output_path, bytes)?;
    }
    Ok((shot.width.unwrap_or(0), shot.height.unwrap_or(0)))
}
