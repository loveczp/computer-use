pub mod accessibility;
pub mod capture;
pub mod desktop;
pub mod input;
pub mod window;

use windows::Win32::Foundation::HWND;
use crate::types::{AccessibilityState, IndexedElement, ListAppsApp, Screenshot, Window};

pub fn list_windows() -> Result<Vec<Window>, Box<dyn std::error::Error + Send + Sync>> {
    window::list_windows()
}

pub fn list_apps() -> Result<Vec<ListAppsApp>, Box<dyn std::error::Error + Send + Sync>> {
    window::list_apps()
}

pub fn get_window(id: usize) -> Result<Window, Box<dyn std::error::Error + Send + Sync>> {
    window::get_window(id)
}

pub fn activate_window(id: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    window::activate_window(HWND(id as *mut _))
}

pub fn launch_app(app: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    window::launch_app(app)
}

pub fn capture_window(id: usize, shot_id: &str) -> Result<Screenshot, Box<dyn std::error::Error + Send + Sync>> {
    capture::capture_window(HWND(id as *mut _), shot_id)
}

pub fn capture_window_to_file(id: usize, path: &str) -> Result<(u32, u32), Box<dyn std::error::Error + Send + Sync>> {
    capture::capture_window_to_file(HWND(id as *mut _), path)
}

pub fn inspect_accessibility(id: usize) -> Result<(AccessibilityState, Vec<IndexedElement>), Box<dyn std::error::Error + Send + Sync>> {
    accessibility::inspect_accessibility(HWND(id as *mut _))
}

pub fn click_element(id: usize, element_idx: usize, button: &str, count: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    input::click_element(HWND(id as *mut _), element_idx, button, count)
}

pub fn click_at(id: usize, x: i32, y: i32, button: &str, count: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    input::click_at(HWND(id as *mut _), x, y, button, count)
}

pub fn type_text(id: usize, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    input::type_text(HWND(id as *mut _), text)
}

pub fn press_key(id: usize, chord: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    input::press_key(HWND(id as *mut _), chord)
}

pub fn set_value(id: usize, element_idx: usize, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    input::set_value(HWND(id as *mut _), element_idx, value)
}

pub fn scroll(id: usize, x: i32, y: i32, dx: i32, dy: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    input::scroll(HWND(id as *mut _), x, y, dx, dy)
}

pub fn drag(id: usize, from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    input::drag(HWND(id as *mut _), from_x, from_y, to_x, to_y)
}

pub fn perform_secondary_action(id: usize, element_idx: usize, action: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    input::perform_secondary_action(HWND(id as *mut _), element_idx, action)
}
