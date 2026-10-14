use std::collections::HashMap;
use std::path::Path;
use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
use windows::Win32::System::StationsAndDesktops::EnumDesktopWindows;
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use super::desktop::ensure_user_desktop;
use crate::types::{ListAppsApp, Window, WindowRect};

/// Helper to get the process name or full path from a process ID.
pub fn get_process_name(pid: u32) -> String {
    if pid == 0 {
        return "System".to_string();
    }
    unsafe {
        if let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buf = [0u16; 1024];
            let mut size = buf.len() as u32;
            if QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut size).is_ok() && size > 0 {
                let full_path = String::from_utf16_lossy(&buf[..size as usize]);
                if let Some(file_name) = Path::new(&full_path).file_name().and_then(|f| f.to_str()) {
                    return file_name.to_lowercase();
                }
                return full_path;
            }
            // Fallback to GetModuleFileNameExW
            let len = GetModuleFileNameExW(Some(process), None, &mut buf);
            if len > 0 {
                let full_path = String::from_utf16_lossy(&buf[..len as usize]);
                if let Some(file_name) = Path::new(&full_path).file_name().and_then(|f| f.to_str()) {
                    return file_name.to_lowercase();
                }
                return full_path;
            }
        }
    }
    format!("process_{}", pid)
}

/// Check if a window is cloaked by Windows Desktop Window Manager.
pub fn is_window_cloaked(hwnd: HWND) -> bool {
    unsafe {
        let mut cloaked: u32 = 0;
        let res = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
        );
        res.is_ok() && cloaked != 0
    }
}

/// Enumerate all open top-level windows.
pub fn list_windows() -> Result<Vec<Window>, Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;

    let mut windows: Vec<Window> = Vec::new();

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL::from(true);
            }

            if is_window_cloaked(hwnd) {
                return BOOL::from(true);
            }

            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_err() {
                return BOOL::from(true);
            }

            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            if width <= 0 || height <= 0 {
                return BOOL::from(true);
            }

            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let title = if len > 0 {
                Some(String::from_utf16_lossy(&title_buf[..len as usize]))
            } else {
                None
            };

            // Skip windows with no title and no meaningful size or tooltips
            let mut class_buf = [0u16; 256];
            let class_len = GetClassNameW(hwnd, &mut class_buf);
            let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);

            // Filter standard background system windows
            if class_name == "Progman" || class_name == "Shell_TrayWnd" || class_name == "Windows.UI.Core.CoreWindow" && title.is_none() {
                return BOOL::from(true);
            }

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let app_name = get_process_name(pid);

            let win_list = &mut *(lparam.0 as *mut Vec<Window>);
            win_list.push(Window {
                id: hwnd.0 as usize,
                app: app_name,
                title,
                rect: Some(WindowRect {
                    x: rect.left,
                    y: rect.top,
                    width,
                    height,
                }),
            });

            BOOL::from(true)
        }
    }

    unsafe {
        let _ = EnumDesktopWindows(
            None,
            Some(enum_proc),
            LPARAM(&mut windows as *mut _ as isize),
        );
    }

    // Sort by id / title
    windows.sort_by(|a, b| a.app.cmp(&b.app).then(a.id.cmp(&b.id)));
    Ok(windows)
}

/// Enumerate apps and group their open windows.
pub fn list_apps() -> Result<Vec<ListAppsApp>, Box<dyn std::error::Error + Send + Sync>> {
    let windows = list_windows()?;
    let mut app_map: HashMap<String, Vec<Window>> = HashMap::new();

    for win in windows {
        app_map.entry(win.app.clone()).or_default().push(win);
    }

    let mut apps: Vec<ListAppsApp> = app_map
        .into_iter()
        .map(|(app_id, wins)| {
            let display_name = wins.iter().find_map(|w| w.title.clone()).or_else(|| Some(app_id.clone()));
            ListAppsApp {
                id: app_id,
                display_name,
                is_running: Some(true),
                windows: wins,
            }
        })
        .collect();

    apps.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(apps)
}

/// Rehydrate or retrieve a specific window by its HWND id.
pub fn get_window(id: usize) -> Result<Window, Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;

    let hwnd = HWND(id as *mut _);
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err(format!("Window handle {} is not a valid window", id).into());
        }

        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title_buf);
        let title = if len > 0 {
            Some(String::from_utf16_lossy(&title_buf[..len as usize]))
        } else {
            None
        };

        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let app = get_process_name(pid);

        Ok(Window {
            id,
            app,
            title,
            rect: Some(WindowRect {
                x: rect.left,
                y: rect.top,
                width,
                height,
            }),
        })
    }
}

/// Activate a window: restore if minimized, bring to top and set foreground.
pub fn activate_window(hwnd: HWND) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

        // Attach input threads to bypass Windows focus-stealing restrictions
        let cur_thread = GetCurrentThreadId();
        let fg_hwnd = GetForegroundWindow();
        let fg_thread = if !fg_hwnd.is_invalid() {
            GetWindowThreadProcessId(fg_hwnd, None)
        } else {
            0
        };

        if fg_thread != 0 && fg_thread != cur_thread {
            let _ = AttachThreadInput(cur_thread, fg_thread, true);
        }

        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(hwnd));

        if fg_thread != 0 && fg_thread != cur_thread {
            let _ = AttachThreadInput(cur_thread, fg_thread, false);
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
        Ok(())
    }
}

/// Launch an app by path, name, or protocol.
pub fn launch_app(app: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_user_desktop()?;

    // If it's a URI or protocol (e.g. calc:, notepad:) or executable
    if app.contains("://") || app.ends_with(':') {
        let uri = windows::core::HSTRING::from(app);
        unsafe {
            windows::Win32::UI::Shell::ShellExecuteW(
                None,
                windows::core::w!("open"),
                &uri,
                None,
                None,
                SW_SHOWNORMAL,
            );
        }
        return Ok(());
    }

    // Spawn process targeting the interactive user desktop ("WinSta0\Default")
    unsafe {
        let mut desk_utf16: Vec<u16> = "WinSta0\\Default\0".encode_utf16().collect();
        let mut si = windows::Win32::System::Threading::STARTUPINFOW::default();
        si.cb = std::mem::size_of::<windows::Win32::System::Threading::STARTUPINFOW>() as u32;
        si.lpDesktop = PWSTR(desk_utf16.as_mut_ptr());

        let mut pi = windows::Win32::System::Threading::PROCESS_INFORMATION::default();
        let mut cmd_utf16: Vec<u16> = format!("{}\0", app).encode_utf16().collect();

        let res = windows::Win32::System::Threading::CreateProcessW(
            None,
            Some(PWSTR(cmd_utf16.as_mut_ptr())),
            None,
            None,
            false,
            windows::Win32::System::Threading::PROCESS_CREATION_FLAGS(0),
            None,
            None,
            &si,
            &mut pi,
        );

        if res.is_ok() {
            let _ = windows::Win32::Foundation::CloseHandle(pi.hProcess);
            let _ = windows::Win32::Foundation::CloseHandle(pi.hThread);
            return Ok(());
        }

        // Fallback to ShellExecuteW
        let app_hstr = windows::core::HSTRING::from(app);
        windows::Win32::UI::Shell::ShellExecuteW(
            None,
            windows::core::w!("open"),
            &app_hstr,
            None,
            None,
            SW_SHOWNORMAL,
        );
        Ok(())
    }
}
