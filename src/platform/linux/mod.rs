use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::session::get_element;
use crate::types::{AccessibilityState, IndexedElement, ListAppsApp, Screenshot, Window, WindowRect};

/// Helper to resolve process name from PID using /proc.
fn get_proc_name(pid: u32) -> String {
    if pid == 0 {
        return "system".to_string();
    }
    let comm_path = format!("/proc/{}/comm", pid);
    if let Ok(comm) = fs::read_to_string(&comm_path) {
        let trimmed = comm.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let cmdline_path = format!("/proc/{}/cmdline", pid);
    if let Ok(cmdline) = fs::read(&cmdline_path) {
        if let Some(first_null) = cmdline.iter().position(|&b| b == 0) {
            if let Ok(first_arg) = std::str::from_utf8(&cmdline[..first_null]) {
                if let Some(filename) = Path::new(first_arg).file_name() {
                    return filename.to_string_lossy().to_string();
                }
            }
        }
    }
    format!("pid_{}", pid)
}

/// List open desktop windows on Linux.
pub fn list_windows() -> Result<Vec<Window>, Box<dyn std::error::Error + Send + Sync>> {
    // Try wmctrl first
    let wmctrl_out = Command::new("wmctrl").args(["-l", "-p", "-G"]).output();
    if let Ok(output) = wmctrl_out {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut list = Vec::new();
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 8 {
                    let id_str = parts[0];
                    let id = if id_str.starts_with("0x") || id_str.starts_with("0X") {
                        usize::from_str_radix(&id_str[2..], 16).unwrap_or(0)
                    } else {
                        id_str.parse::<usize>().unwrap_or(0)
                    };

                    let pid: u32 = parts[2].parse().unwrap_or(0);
                    let x: i32 = parts[3].parse().unwrap_or(0);
                    let y: i32 = parts[4].parse().unwrap_or(0);
                    let width: i32 = parts[5].parse().unwrap_or(0);
                    let height: i32 = parts[6].parse().unwrap_or(0);
                    let title = parts[8..].join(" ");
                    let app = get_proc_name(pid);

                    list.push(Window {
                        id,
                        app,
                        title: Some(title),
                        rect: Some(WindowRect {
                            x,
                            y,
                            width,
                            height,
                        }),
                    });
                }
            }
            return Ok(list);
        }
    }

    // Fallback: try xdotool search
    let xdotool_out = Command::new("xdotool")
        .args(["search", "--onlyvisible", ""])
        .output();
    if let Ok(output) = xdotool_out {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut list = Vec::new();
            for line in stdout.lines() {
                let trimmed = line.trim();
                if let Ok(id) = trimmed.parse::<usize>() {
                    let name_out = Command::new("xdotool")
                        .args(["getwindowname", trimmed])
                        .output();
                    let title = name_out
                        .ok()
                        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                        .filter(|t| !t.is_empty());

                    let pid_out = Command::new("xdotool")
                        .args(["getwindowpid", trimmed])
                        .output();
                    let pid: u32 = pid_out
                        .ok()
                        .and_then(|o| {
                            String::from_utf8_lossy(&o.stdout)
                                .trim()
                                .parse::<u32>()
                                .ok()
                        })
                        .unwrap_or(0);
                    let app = get_proc_name(pid);

                    list.push(Window {
                        id,
                        app,
                        title,
                        rect: None,
                    });
                }
            }
            return Ok(list);
        }
    }

    Err("Neither 'wmctrl' nor 'xdotool' found. Please install wmctrl or xdotool (e.g. 'sudo apt-get install wmctrl xdotool').".into())
}

/// List running apps grouped with their open windows.
pub fn list_apps() -> Result<Vec<ListAppsApp>, Box<dyn std::error::Error + Send + Sync>> {
    let windows = list_windows()?;
    let mut app_map: HashMap<String, Vec<Window>> = HashMap::new();

    for w in windows {
        app_map.entry(w.app.clone()).or_default().push(w);
    }

    let mut apps: Vec<ListAppsApp> = app_map
        .into_iter()
        .map(|(app_name, wins)| ListAppsApp {
            id: app_name.clone(),
            display_name: Some(app_name),
            is_running: Some(true),
            windows: wins,
        })
        .collect();

    apps.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(apps)
}

/// Get details of a specific window by its ID.
pub fn get_window(id: usize) -> Result<Window, Box<dyn std::error::Error + Send + Sync>> {
    let windows = list_windows()?;
    windows
        .into_iter()
        .find(|w| w.id == id)
        .ok_or_else(|| format!("Window ID {} not found", id).into())
}

/// Bring a window to foreground (restoring if minimized).
pub fn activate_window(id: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Try wmctrl
    let status = Command::new("wmctrl")
        .args(["-i", "-a", &format!("0x{:x}", id)])
        .status();
    if let Ok(s) = status {
        if s.success() {
            return Ok(());
        }
    }

    // Try xdotool
    let status = Command::new("xdotool")
        .args(["windowactivate", &id.to_string()])
        .status();
    if let Ok(s) = status {
        if s.success() {
            return Ok(());
        }
    }

    Err(format!("Failed to activate window 0x{:x}", id).into())
}

/// Launch an application.
pub fn launch_app(app: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(s) = Command::new("gtk-launch").arg(app).status() {
        if s.success() {
            return Ok(());
        }
    }
    if let Ok(s) = Command::new("xdg-open").arg(app).status() {
        if s.success() {
            return Ok(());
        }
    }
    Command::new("sh")
        .args(["-c", &format!("{} &", app)])
        .spawn()?;
    Ok(())
}

/// Capture a screenshot of a specific window.
pub fn capture_window(
    id: usize,
    shot_id: &str,
) -> Result<Screenshot, Box<dyn std::error::Error + Send + Sync>> {
    let temp_path = std::env::temp_dir().join(format!(
        "window_{}_{}.png",
        id,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    let path_str = temp_path.to_str().ok_or("Invalid temporary path")?;

    let (w, h) = capture_window_to_file(id, path_str)?;
    let bytes = fs::read(&temp_path)?;
    let _ = fs::remove_file(&temp_path);

    let b64 = format!("data:image/png;base64,{}", BASE64_STANDARD.encode(&bytes));
    let win = get_window(id).ok();

    Ok(Screenshot {
        id: shot_id.to_string(),
        url: b64,
        width: Some(w),
        height: Some(h),
        origin_x: win.as_ref().and_then(|w| w.rect.as_ref().map(|r| r.x)),
        origin_y: win.as_ref().and_then(|w| w.rect.as_ref().map(|r| r.y)),
        z_index: 0,
    })
}

/// Capture a screenshot of a window directly to a file on disk.
pub fn capture_window_to_file(
    id: usize,
    path: &str,
) -> Result<(u32, u32), Box<dyn std::error::Error + Send + Sync>> {
    let hex_id = format!("0x{:x}", id);

    // Try ImageMagick import
    if let Ok(s) = Command::new("import")
        .args(["-window", &hex_id, path])
        .status()
    {
        if s.success() {
            let img = image::open(path)?;
            return Ok((img.width(), img.height()));
        }
    }

    // Try maim
    if let Ok(s) = Command::new("maim")
        .args(["-i", &hex_id, path])
        .status()
    {
        if s.success() {
            let img = image::open(path)?;
            return Ok((img.width(), img.height()));
        }
    }

    // Try xwd + convert
    let xwd_out = Command::new("xwd")
        .args(["-id", &hex_id, "-silent"])
        .output();
    if let Ok(xwd) = xwd_out {
        if xwd.status.success() {
            let mut convert = Command::new("convert")
                .args(["xwd:-", &format!("png:{}", path)])
                .stdin(std::process::Stdio::piped())
                .spawn()?;
            if let Some(mut stdin) = convert.stdin.take() {
                use std::io::Write;
                stdin.write_all(&xwd.stdout)?;
            }
            if convert.wait()?.success() {
                let img = image::open(path)?;
                return Ok((img.width(), img.height()));
            }
        }
    }

    Err("Failed to capture screenshot. Please install 'imagemagick' ('sudo apt install imagemagick') or 'maim'.".into())
}

/// Inspect accessibility / structural elements for a window.
pub fn inspect_accessibility(
    id: usize,
) -> Result<(AccessibilityState, Vec<IndexedElement>), Box<dyn std::error::Error + Send + Sync>> {
    let win = get_window(id)?;
    let hex_id = format!("0x{:x}", id);

    let mut elements: Vec<IndexedElement> = Vec::new();
    let mut tree_lines: Vec<String> = Vec::new();

    // Use xwininfo -tree to enumerate child windows
    let xwininfo = Command::new("xwininfo")
        .args(["-tree", "-id", &hex_id])
        .output();

    if let Ok(output) = xwininfo {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut idx = 1;
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("0x") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let name_part = if trimmed.contains('"') {
                            trimmed
                                .split('"')
                                .nth(1)
                                .unwrap_or("<unnamed>")
                                .to_string()
                        } else {
                            "<unnamed>".to_string()
                        };

                        let formatted = format!("[{}] window '{}' ({})", idx, name_part, parts[0]);
                        tree_lines.push(formatted);
                        elements.push(IndexedElement {
                            index: idx,
                            name: name_part,
                            control_type: "Window".to_string(),
                            class_name: "X11Window".to_string(),
                            rect: win.rect.clone().unwrap_or(WindowRect {
                                x: 0,
                                y: 0,
                                width: 800,
                                height: 600,
                            }),
                            is_focused: false,
                            is_enabled: true,
                            value: None,
                        });
                        idx += 1;
                    }
                }
            }
        }
    }

    if tree_lines.is_empty() {
        tree_lines.push(format!(
            "[1] Window '{}' (bounds: {:?})",
            win.title.unwrap_or_default(),
            win.rect
        ));
    }

    let state = AccessibilityState {
        tree: tree_lines.join("\n"),
        focused_element: None,
        selected_text: None,
        selected_elements: None,
        document_text: None,
    };

    Ok((state, elements))
}

/// Click inside a window at window-relative coordinates.
pub fn click_at(
    id: usize,
    x: i32,
    y: i32,
    button: &str,
    count: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    activate_window(id)?;
    let btn_code = match button {
        "right" => "3",
        "middle" => "2",
        _ => "1",
    };

    let status = Command::new("xdotool")
        .args([
            "mousemove",
            "--window",
            &id.to_string(),
            &x.to_string(),
            &y.to_string(),
            "click",
            "--repeat",
            &count.to_string(),
            btn_code,
        ])
        .status()?;

    if !status.success() {
        return Err(format!("xdotool click failed with status: {}", status).into());
    }
    Ok(())
}

/// Click an indexed element.
pub fn click_element(
    id: usize,
    element_idx: usize,
    button: &str,
    count: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let elem = get_element(id, element_idx)
        .ok_or_else(|| format!("Element [{}] not found for window {}", element_idx, id))?;

    let center_x = elem.rect.x + elem.rect.width / 2;
    let center_y = elem.rect.y + elem.rect.height / 2;

    let btn_code = match button {
        "right" => "3",
        "middle" => "2",
        _ => "1",
    };

    let status = Command::new("xdotool")
        .args([
            "mousemove",
            &center_x.to_string(),
            &center_y.to_string(),
            "click",
            "--repeat",
            &count.to_string(),
            btn_code,
        ])
        .status()?;

    if !status.success() {
        return Err(format!("xdotool click failed with status: {}", status).into());
    }
    Ok(())
}

/// Type text into the active window.
pub fn type_text(
    id: usize,
    text: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    activate_window(id)?;
    let status = Command::new("xdotool")
        .args([
            "type",
            "--window",
            &id.to_string(),
            "--delay",
            "12",
            "--",
            text,
        ])
        .status()?;

    if !status.success() {
        return Err(format!("xdotool type failed with status: {}", status).into());
    }
    Ok(())
}

/// Press a key chord (e.g. "Control_L+a", "Ctrl+C", "Return", "Tab", "Escape").
pub fn press_key(
    id: usize,
    chord: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    activate_window(id)?;
    // Normalize chord for xdotool
    let mut normalized_parts: Vec<String> = Vec::new();
    for part in chord.split('+').map(|s| s.trim()) {
        let lower = part.to_lowercase();
        let x_key = match lower.as_str() {
            "ctrl" | "control" | "control_l" => "ctrl".to_string(),
            "shift" | "shift_l" => "shift".to_string(),
            "alt" | "alt_l" => "alt".to_string(),
            "super" | "super_l" | "win" => "super".to_string(),
            "return" | "enter" => "Return".to_string(),
            "escape" | "esc" => "Escape".to_string(),
            "tab" => "Tab".to_string(),
            "backspace" | "back" => "BackSpace".to_string(),
            "delete" | "del" => "Delete".to_string(),
            _ => part.to_string(),
        };
        normalized_parts.push(x_key);
    }
    let x_chord = normalized_parts.join("+");

    let status = Command::new("xdotool")
        .args(["key", "--window", &id.to_string(), &x_chord])
        .status()?;

    if !status.success() {
        return Err(format!("xdotool key failed with status: {}", status).into());
    }
    Ok(())
}

/// Replace the value of an editable element.
pub fn set_value(
    id: usize,
    element_idx: usize,
    value: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    click_element(id, element_idx, "left", 1)?;
    press_key(id, "ctrl+a")?;
    press_key(id, "BackSpace")?;
    type_text(id, value)?;
    Ok(())
}

/// Scroll with mouse wheel.
pub fn scroll(
    id: usize,
    _x: i32,
    _y: i32,
    dx: i32,
    dy: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // In X11: button 4 = up, 5 = down, 6 = left, 7 = right
    if dy > 0 {
        Command::new("xdotool")
            .args(["click", "--window", &id.to_string(), "--repeat", &dy.to_string(), "4"])
            .status()?;
    } else if dy < 0 {
        let count = dy.abs();
        Command::new("xdotool")
            .args(["click", "--window", &id.to_string(), "--repeat", &count.to_string(), "5"])
            .status()?;
    }

    if dx > 0 {
        Command::new("xdotool")
            .args(["click", "--window", &id.to_string(), "--repeat", &dx.to_string(), "7"])
            .status()?;
    } else if dx < 0 {
        let count = dx.abs();
        Command::new("xdotool")
            .args(["click", "--window", &id.to_string(), "--repeat", &count.to_string(), "6"])
            .status()?;
    }

    Ok(())
}

/// Drag mouse from one point to another.
pub fn drag(
    id: usize,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let status = Command::new("xdotool")
        .args([
            "mousemove",
            "--window",
            &id.to_string(),
            &from_x.to_string(),
            &from_y.to_string(),
            "mousedown",
            "1",
            "mousemove",
            "--window",
            &id.to_string(),
            &to_x.to_string(),
            &to_y.to_string(),
            "mouseup",
            "1",
        ])
        .status()?;

    if !status.success() {
        return Err(format!("xdotool drag failed with status: {}", status).into());
    }
    Ok(())
}

/// Perform secondary accessibility action on an element.
pub fn perform_secondary_action(
    id: usize,
    element_idx: usize,
    action: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match action.to_lowercase().as_str() {
        "toggle" | "expand" | "collapse" => {
            click_element(id, element_idx, "left", 1)?;
            press_key(id, "space")?;
        }
        _ => {
            click_element(id, element_idx, "left", 1)?;
        }
    }
    Ok(())
}
