use std::thread::sleep;
use std::time::Duration;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::desktop::ensure_user_desktop;
use crate::session::get_element;
use super::window::activate_window;

/// Parse a key chord name into a Windows VIRTUAL_KEY.
pub fn parse_virtual_key(name: &str) -> Option<VIRTUAL_KEY> {
    let normalized = name.trim().to_uppercase();
    match normalized.as_str() {
        "CONTROL" | "CTRL" | "CONTROL_L" | "CONTROL_R" | "LCONTROL" | "RCONTROL" => Some(VK_CONTROL),
        "ALT" | "ALT_L" | "ALT_R" | "MENU" | "LMENU" | "RMENU" => Some(VK_MENU),
        "SHIFT" | "SHIFT_L" | "SHIFT_R" | "LSHIFT" | "RSHIFT" => Some(VK_SHIFT),
        "SUPER" | "SUPER_L" | "SUPER_R" | "WIN" | "WINDOWS" | "LWIN" | "RWIN" => Some(VK_LWIN),
        "RETURN" | "ENTER" => Some(VK_RETURN),
        "TAB" => Some(VK_TAB),
        "SPACE" => Some(VK_SPACE),
        "ESCAPE" | "ESC" => Some(VK_ESCAPE),
        "BACKSPACE" | "BACK" => Some(VK_BACK),
        "DELETE" | "DEL" => Some(VK_DELETE),
        "INSERT" | "INS" => Some(VK_INSERT),
        "HOME" => Some(VK_HOME),
        "END" => Some(VK_END),
        "PAGE_UP" | "PAGEUP" | "PRIOR" => Some(VK_PRIOR),
        "PAGE_DOWN" | "PAGEDOWN" | "NEXT" => Some(VK_NEXT),
        "UP" | "ARROWUP" => Some(VK_UP),
        "DOWN" | "ARROWDOWN" => Some(VK_DOWN),
        "LEFT" | "ARROWLEFT" => Some(VK_LEFT),
        "RIGHT" | "ARROWRIGHT" => Some(VK_RIGHT),
        "CAPITAL" | "CAPSLOCK" => Some(VK_CAPITAL),
        "PRINTSCREEN" | "PRTSC" | "SNAPSHOT" => Some(VK_SNAPSHOT),
        "F1" => Some(VK_F1),
        "F2" => Some(VK_F2),
        "F3" => Some(VK_F3),
        "F4" => Some(VK_F4),
        "F5" => Some(VK_F5),
        "F6" => Some(VK_F6),
        "F7" => Some(VK_F7),
        "F8" => Some(VK_F8),
        "F9" => Some(VK_F9),
        "F10" => Some(VK_F10),
        "F11" => Some(VK_F11),
        "F12" => Some(VK_F12),
        "0" => Some(VK_0),
        "1" => Some(VK_1),
        "2" => Some(VK_2),
        "3" => Some(VK_3),
        "4" => Some(VK_4),
        "5" => Some(VK_5),
        "6" => Some(VK_6),
        "7" => Some(VK_7),
        "8" => Some(VK_8),
        "9" => Some(VK_9),
        "A" => Some(VK_A),
        "B" => Some(VK_B),
        "C" => Some(VK_C),
        "D" => Some(VK_D),
        "E" => Some(VK_E),
        "F" => Some(VK_F),
        "G" => Some(VK_G),
        "H" => Some(VK_H),
        "I" => Some(VK_I),
        "J" => Some(VK_J),
        "K" => Some(VK_K),
        "L" => Some(VK_L),
        "M" => Some(VK_M),
        "N" => Some(VK_N),
        "O" => Some(VK_O),
        "P" => Some(VK_P),
        "Q" => Some(VK_Q),
        "R" => Some(VK_R),
        "S" => Some(VK_S),
        "T" => Some(VK_T),
        "U" => Some(VK_U),
        "V" => Some(VK_V),
        "W" => Some(VK_W),
        "X" => Some(VK_X),
        "Y" => Some(VK_Y),
        "Z" => Some(VK_Z),
        "PERIOD" | "." => Some(VK_OEM_PERIOD),
        "COMMA" | "," => Some(VK_OEM_COMMA),
        "SLASH" | "/" => Some(VK_OEM_2),
        "SEMICOLON" | ";" => Some(VK_OEM_1),
        "MINUS" | "-" => Some(VK_OEM_MINUS),
        "EQUAL" | "=" => Some(VK_OEM_PLUS),
        "KP_0" | "NUMPAD_0" => Some(VK_NUMPAD0),
        "KP_1" | "NUMPAD_1" => Some(VK_NUMPAD1),
        "KP_2" | "NUMPAD_2" => Some(VK_NUMPAD2),
        "KP_3" | "NUMPAD_3" => Some(VK_NUMPAD3),
        "KP_4" | "NUMPAD_4" => Some(VK_NUMPAD4),
        "KP_5" | "NUMPAD_5" => Some(VK_NUMPAD5),
        "KP_6" | "NUMPAD_6" => Some(VK_NUMPAD6),
        "KP_7" | "NUMPAD_7" => Some(VK_NUMPAD7),
        "KP_8" | "NUMPAD_8" => Some(VK_NUMPAD8),
        "KP_9" | "NUMPAD_9" => Some(VK_NUMPAD9),
        _ => None,
    }
}

/// Send mouse cursor movement to screen coordinates.
pub fn move_cursor(x: i32, y: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    unsafe {
        let cx = GetSystemMetrics(SM_CXSCREEN);
        let cy = GetSystemMetrics(SM_CYSCREEN);
        if cx <= 0 || cy <= 0 {
            return Err("Failed to query screen size".into());
        }

        let abs_x = (x as f64 / cx as f64 * 65535.0).round() as i32;
        let abs_y = (y as f64 / cy as f64 * 65535.0).round() as i32;

        let input = [INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: abs_x,
                    dy: abs_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }];
        SendInput(&input, std::mem::size_of::<INPUT>() as i32);
        Ok(())
    }
}

/// Click at absolute screen coordinates.
pub fn click_screen(
    x: i32,
    y: i32,
    button: &str,
    click_count: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;
    move_cursor(x, y)?;
    sleep(Duration::from_millis(50));

    let (down_flag, up_flag) = match button.to_lowercase().as_str() {
        "right" | "r" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        "middle" | "m" => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
        _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
    };

    unsafe {
        for i in 0..click_count.max(1) {
            let down = [INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: down_flag,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }];
            SendInput(&down, std::mem::size_of::<INPUT>() as i32);
            sleep(Duration::from_millis(30));

            let up = [INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: up_flag,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }];
            SendInput(&up, std::mem::size_of::<INPUT>() as i32);

            if i + 1 < click_count {
                sleep(Duration::from_millis(60));
            }
        }
    }

    Ok(())
}

/// Click mouse at window-relative coordinates.
pub fn click_at(
    hwnd: HWND,
    x: i32,
    y: i32,
    button: &str,
    click_count: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;
    activate_window(hwnd)?;

    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect)?;
        click_screen(rect.left + x, rect.top + y, button, click_count)
    }
}

/// Click a UI automation element by its index from the latest observation.
pub fn click_element(
    hwnd: HWND,
    element_index: usize,
    button: &str,
    click_count: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;
    activate_window(hwnd)?;

    let elem = get_element(hwnd.0 as usize, element_index).ok_or_else(|| {
        format!(
            "Element index [{}] not found for window 0x{:x}. Please observe (state) before acting.",
            element_index, hwnd.0 as usize
        )
    })?;

    // Calculate center point of the element
    let center_x = elem.rect.x + (elem.rect.width / 2);
    let center_y = elem.rect.y + (elem.rect.height / 2);

    click_screen(center_x, center_y, button, click_count)
}

/// Type text into the focused window/element using Unicode SendInput.
pub fn type_text(hwnd: HWND, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;
    activate_window(hwnd)?;

    unsafe {
        for ch in text.chars() {
            let mut utf16_buf = [0u16; 2];
            let encoded = ch.encode_utf16(&mut utf16_buf);

            for &mut code in encoded {
                let down = [INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: code,
                            dwFlags: KEYEVENTF_UNICODE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                }];
                SendInput(&down, std::mem::size_of::<INPUT>() as i32);

                let up = [INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: code,
                            dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                }];
                SendInput(&up, std::mem::size_of::<INPUT>() as i32);
            }
            sleep(Duration::from_millis(10));
        }
        Ok(())
    }
}

/// Press a `+`-separated key chord (e.g. `Control_L+a`, `Alt+F4`, `Return`).
pub fn press_key(hwnd: HWND, chord: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;
    activate_window(hwnd)?;

    let parts: Vec<&str> = chord.split('+').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return Err("Key chord cannot be empty".into());
    }

    let mut keys: Vec<VIRTUAL_KEY> = Vec::new();
    for part in parts {
        let vk = parse_virtual_key(part)
            .ok_or_else(|| format!("Unknown key: '{}' in chord '{}'", part, chord))?;
        keys.push(vk);
    }

    unsafe {
        // Press all keys down in forward order
        for &key in &keys {
            let down = [INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: key,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }];
            SendInput(&down, std::mem::size_of::<INPUT>() as i32);
            sleep(Duration::from_millis(15));
        }

        sleep(Duration::from_millis(40));

        // Release all keys in reverse order
        for &key in keys.iter().rev() {
            let up = [INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: key,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }];
            SendInput(&up, std::mem::size_of::<INPUT>() as i32);
            sleep(Duration::from_millis(15));
        }

        Ok(())
    }
}

/// Replace the value of an editable element.
pub fn set_value(
    hwnd: HWND,
    element_index: usize,
    value: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Click element to focus it
    click_element(hwnd, element_index, "left", 1)?;
    sleep(Duration::from_millis(50));

    // Select all (Ctrl+A)
    press_key(hwnd, "Control_L+a")?;
    sleep(Duration::from_millis(50));

    // Type the replacement value
    type_text(hwnd, value)
}

/// Mouse scroll by delta at window coordinates.
pub fn scroll(
    hwnd: HWND,
    x: i32,
    y: i32,
    scroll_x: i32,
    scroll_y: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;
    activate_window(hwnd)?;

    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect)?;

        let screen_x = rect.left + x;
        let screen_y = rect.top + y;
        move_cursor(screen_x, screen_y)?;
        sleep(Duration::from_millis(30));

        // Vertical scroll
        if scroll_y != 0 {
            let input = [INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: (scroll_y as u32) & 0xFFFF,
                        dwFlags: MOUSEEVENTF_WHEEL,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }];
            SendInput(&input, std::mem::size_of::<INPUT>() as i32);
        }

        // Horizontal scroll
        if scroll_x != 0 {
            let input = [INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: (scroll_x as u32) & 0xFFFF,
                        dwFlags: MOUSEEVENTF_HWHEEL,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }];
            SendInput(&input, std::mem::size_of::<INPUT>() as i32);
        }

        Ok(())
    }
}

/// Mouse drag between coordinates.
pub fn drag(
    hwnd: HWND,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;
    activate_window(hwnd)?;

    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect)?;

        let start_x = rect.left + from_x;
        let start_y = rect.top + from_y;
        let end_x = rect.left + to_x;
        let end_y = rect.top + to_y;

        move_cursor(start_x, start_y)?;
        sleep(Duration::from_millis(50));

        // Mouse down
        let down = [INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }];
        SendInput(&down, std::mem::size_of::<INPUT>() as i32);
        sleep(Duration::from_millis(50));

        // Smooth interpolate to end position
        let steps = 15;
        for i in 1..=steps {
            let cur_x = start_x + ((end_x - start_x) * i) / steps;
            let cur_y = start_y + ((end_y - start_y) * i) / steps;
            move_cursor(cur_x, cur_y)?;
            sleep(Duration::from_millis(15));
        }

        // Mouse up
        let up = [INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }];
        SendInput(&up, std::mem::size_of::<INPUT>() as i32);
        Ok(())
    }
}

/// Perform secondary accessibility action on an element (Expand, Collapse, etc.).
pub fn perform_secondary_action(
    hwnd: HWND,
    element_index: usize,
    action: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // As a robust baseline, click the element or double click depending on action
    match action.to_lowercase().as_str() {
        "expand" | "collapse" | "toggle" => click_element(hwnd, element_index, "left", 1),
        _ => click_element(hwnd, element_index, "left", 1),
    }
}
