const DWM_COLOR_DEFAULT: u32 = 0xffff_ffff;

#[tauri::command]
pub fn set_window_caption_color(
    window: tauri::Window,
    color: Option<[u8; 3]>,
) -> Result<(), String> {
    set_window_caption_color_impl(&window, color)
}

fn caption_color_value(color: Option<[u8; 3]>) -> u32 {
    color.map_or(DWM_COLOR_DEFAULT, |[red, green, blue]| {
        u32::from(red) | (u32::from(green) << 8) | (u32::from(blue) << 16)
    })
}

#[cfg(windows)]
fn set_window_caption_color_impl(
    window: &tauri::Window,
    color: Option<[u8; 3]>,
) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CAPTION_COLOR};

    let hwnd = window
        .hwnd()
        .map_err(|error| format!("Could not access the Cloudburst window: {error}"))?;
    let color = caption_color_value(color);
    let result = unsafe {
        DwmSetWindowAttribute(
            hwnd.0,
            DWMWA_CAPTION_COLOR as u32,
            (&color as *const u32).cast(),
            size_of::<u32>() as u32,
        )
    };

    if result < 0 {
        Err(format!(
            "Could not set the Windows caption color (HRESULT 0x{:08X})",
            result as u32
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn set_window_caption_color_impl(
    _window: &tauri::Window,
    _color: Option<[u8; 3]>,
) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_rgb_to_a_windows_colorref() {
        assert_eq!(caption_color_value(Some([16, 16, 18])), 0x0012_1010);
        assert_eq!(caption_color_value(Some([255, 255, 255])), 0x00ff_ffff);
    }

    #[test]
    fn resets_material_modes_to_the_system_caption_color() {
        assert_eq!(caption_color_value(None), DWM_COLOR_DEFAULT);
    }
}
