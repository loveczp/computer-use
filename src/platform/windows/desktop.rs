use windows::core::PCSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::StationsAndDesktops::*;
use windows::Win32::System::Threading::GetCurrentThreadId;

/// Ensures the current thread is attached to the interactive user desktop ("Default").
/// This is necessary because processes launched in sandboxed or background agent sessions
/// might initially run in an isolated desktop.
pub fn ensure_user_desktop() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    unsafe {
        // Check current desktop name
        if let Ok(cur_desk) = GetThreadDesktop(GetCurrentThreadId()) {
            let mut name_buf = [0u8; 256];
            let mut len = 0u32;
            let res = GetUserObjectInformationA(
                HANDLE(cur_desk.0),
                UOI_NAME,
                Some(name_buf.as_mut_ptr() as _),
                name_buf.len() as u32,
                Some(&mut len),
            );
            if res.is_ok() && len > 0 {
                let name = String::from_utf8_lossy(&name_buf[..len as usize]);
                let trimmed = name.trim_matches(char::from(0)).trim();
                if trimmed.eq_ignore_ascii_case("default") {
                    return Ok(());
                }
            }
        }

        // Try opening "Default" desktop
        let default_desk = OpenDesktopA(
            PCSTR(b"Default\0".as_ptr()),
            DESKTOP_CONTROL_FLAGS(0),
            true,
            0x01ff, // DESKTOP_ALL_ACCESS
        )?;

        SetThreadDesktop(default_desk)?;
        Ok(())
    }
}
