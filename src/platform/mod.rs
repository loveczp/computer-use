#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::*;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub mod fallback {
    use crate::types::*;
    pub fn list_windows() -> Result<Vec<Window>, Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn list_apps() -> Result<Vec<ListAppsApp>, Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn get_window(_id: usize) -> Result<Window, Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn activate_window(_id: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn launch_app(_app: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn capture_window(_id: usize, _shot_id: &str) -> Result<Screenshot, Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn capture_window_to_file(_id: usize, _path: &str) -> Result<(u32, u32), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn inspect_accessibility(_id: usize) -> Result<(AccessibilityState, Vec<IndexedElement>), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn click_element(_id: usize, _element_idx: usize, _button: &str, _count: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn click_at(_id: usize, _x: i32, _y: i32, _button: &str, _count: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn type_text(_id: usize, _text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn press_key(_id: usize, _chord: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn set_value(_id: usize, _element_idx: usize, _value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn scroll(_id: usize, _x: i32, _y: i32, _dx: i32, _dy: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn drag(_id: usize, _from_x: i32, _from_y: i32, _to_x: i32, _to_y: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
    pub fn perform_secondary_action(_id: usize, _element_idx: usize, _action: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Unsupported operating system".into())
    }
}
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub use self::fallback::*;
