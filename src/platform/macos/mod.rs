use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use std::collections::HashMap;
use std::process::Command;

use crate::session::get_element;
use crate::types::{AccessibilityState, IndexedElement, ListAppsApp, Screenshot, Window, WindowRect};

/// Run an AppleScript command via osascript.
fn run_osascript(script: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("osascript failed: {}", err.trim()).into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run a JavaScript for Automation (JXA) script via osascript.
fn run_jxa(script: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let output = Command::new("osascript")
        .arg("-l")
        .arg("JavaScript")
        .arg("-e")
        .arg(script)
        .output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("JXA script failed: {}", err.trim()).into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// List open desktop windows on macOS using Quartz CoreGraphics via JXA.
pub fn list_windows() -> Result<Vec<Window>, Box<dyn std::error::Error + Send + Sync>> {
    let script = r#"
ObjC.import('CoreGraphics');
var windowList = $.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly | $.kCGWindowListExcludeDesktopElements, $.kCGNullWindowID);
var count = $.CFArrayGetCount(windowList);
var list = [];
for (var i = 0; i < count; i++) {
    var dict = $.CFArrayGetValueAtIndex(windowList, i);
    var obj = ObjC.castRefToObject(dict);
    var layerVal = obj.valueForKey("kCGWindowLayer");
    var layer = layerVal ? layerVal.js : null;
    if (layer === 0) {
        var idVal = obj.valueForKey("kCGWindowNumber");
        var id = idVal ? idVal.js : 0;
        var ownerVal = obj.valueForKey("kCGWindowOwnerName");
        var owner = (ownerVal && ownerVal.js) ? ownerVal.js : "";
        var nameVal = obj.valueForKey("kCGWindowName");
        var name = (nameVal && nameVal.js) ? nameVal.js : "";
        var bounds = obj.valueForKey("kCGWindowBounds");
        var rect = null;
        if (bounds) {
            rect = {
                x: Math.round(bounds.valueForKey("X").js),
                y: Math.round(bounds.valueForKey("Y").js),
                width: Math.round(bounds.valueForKey("Width").js),
                height: Math.round(bounds.valueForKey("Height").js)
            };
        }
        if (owner !== "Dock" && owner !== "Window Server" && owner !== "") {
            list.push({ id: id, app: owner, title: name.length > 0 ? name : null, rect: rect });
        }
    }
}
JSON.stringify(list);
"#;

    match run_jxa(script) {
        Ok(out) => {
            let wins: Vec<Window> = serde_json::from_str(&out)?;
            Ok(wins)
        }
        Err(_) => {
            // Fallback to AppleScript System Events
            let ascript = r#"
tell application "System Events"
    set res to {}
    set appList to every application process whose background only is false
    repeat with p in appList
        set pName to name of p
        try
            set pWins to every window of p
            repeat with w in pWins
                set wName to ""
                try
                    set wName to name of w
                end try
                set wPos to position of w
                set wSize to size of w
                set end of res to "{\"id\":0,\"app\":\"" & pName & "\",\"title\":\"" & wName & "\",\"rect\":{\"x\":" & (item 1 of wPos) & ",\"y\":" & (item 2 of wPos) & ",\"width\":" & (item 1 of wSize) & ",\"height\":" & (item 2 of wSize) & "}}"
            end repeat
        end try
    end repeat
    set AppleScript's text item delimiters to ","
    return "[" & (res as string) & "]"
end tell
"#;
            let out = run_osascript(ascript)?;
            let wins: Vec<Window> = serde_json::from_str(&out).unwrap_or_default();
            Ok(wins)
        }
    }
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

/// Bring a window to foreground (restoring and focusing its parent app).
pub fn activate_window(id: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let win = get_window(id)?;
    let escaped_app = win.app.replace('\\', "\\\\").replace('\'', "\\'");
    let script = format!(
        r#"
ObjC.import('AppKit');
var apps = $.NSWorkspace.sharedWorkspace.runningApplications;
var count = apps.count;
for (var i = 0; i < count; i++) {{
    var a = apps.objectAtIndex(i);
    var name = a.localizedName;
    if (name && name.js === '{}') {{
        a.activateWithOptions($.NSApplicationActivateIgnoringOtherApps);
        break;
    }}
}}
"#,
        escaped_app
    );
    if run_jxa(&script).is_err() {
        let _ = Command::new("open").arg("-a").arg(&win.app).status();
    }
    Ok(())
}

/// Launch an application using macOS `open`.
pub fn launch_app(app: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let status = Command::new("open").arg("-a").arg(app).status();
    if status.is_err() || !status.as_ref().unwrap().success() {
        let status2 = Command::new("open").arg(app).status()?;
        if !status2.success() {
            return Err(format!("Failed to open application: {}", app).into());
        }
    }
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
    let bytes = std::fs::read(&temp_path)?;
    let _ = std::fs::remove_file(&temp_path);

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
    let status = Command::new("screencapture")
        .args(["-l", &id.to_string(), "-o", "-x", path])
        .status()?;

    if !status.success() {
        return Err(format!("screencapture failed with status: {}", status).into());
    }

    let img = image::open(path)?;
    Ok((img.width(), img.height()))
}

/// Inspect accessibility elements for a window.
pub fn inspect_accessibility(
    id: usize,
) -> Result<(AccessibilityState, Vec<IndexedElement>), Box<dyn std::error::Error + Send + Sync>> {
    let win = get_window(id)?;
    let script = format!(
        r#"
tell application "System Events"
    set p to first application process whose name is "{}"
    set w to front window of p
    set res to {{}}
    set todo to every UI element of w
    set elemCount to 0
    repeat with el in todo
        if elemCount > 120 then exit repeat
        try
            set elRole to (role of el) as string
            set elTitle to ""
            try
                set elTitle to (title of el) as string
            end try
            if elTitle is "missing value" or elTitle is "" then
                try
                    set elTitle to (accessibility description of el) as string
                end try
            end if
            if elTitle is "missing value" or elTitle is "" then
                try
                    set elTitle to (description of el) as string
                end try
            end if
            if elTitle is "missing value" or elTitle is "" then
                try
                    set elTitle to (value of el) as string
                end try
            end if
            if elTitle is "missing value" then set elTitle to ""
            set elPos to (position of el)
            set elSize to (size of el)
            
            set AppleScript's text item delimiters to "\""
            set titleParts to text items of elTitle
            set AppleScript's text item delimiters to "\\\""
            set escapedTitle to titleParts as string
            
            set end of res to "{{\"role\":\"" & elRole & "\",\"name\":\"" & escapedTitle & "\",\"x\":" & (item 1 of elPos) & ",\"y\":" & (item 2 of elPos) & ",\"width\":" & (item 1 of elSize) & ",\"height\":" & (item 2 of elSize) & "}}"
            set elemCount to elemCount + 1
            
            try
                repeat with child in (every UI element of el)
                    if elemCount > 120 then exit repeat
                    try
                        set cRole to (role of child) as string
                        set cTitle to ""
                        try
                            set cTitle to (title of child) as string
                        end try
                        if cTitle is "missing value" or cTitle is "" then
                            try
                                set cTitle to (accessibility description of child) as string
                            end try
                        end if
                        if cTitle is "missing value" or cTitle is "" then
                            try
                                set cTitle to (description of child) as string
                            end try
                        end if
                        if cTitle is "missing value" or cTitle is "" then
                            try
                                set cTitle to (value of child) as string
                            end try
                        end if
                        if cTitle is "missing value" then set cTitle to ""
                        set cPos to (position of child)
                        set cSize to (size of child)
                        
                        set AppleScript's text item delimiters to "\""
                        set cParts to text items of cTitle
                        set AppleScript's text item delimiters to "\\\""
                        set escapedCTitle to cParts as string
                        
                        set end of res to "{{\"role\":\"" & cRole & "\",\"name\":\"" & escapedCTitle & "\",\"x\":" & (item 1 of cPos) & ",\"y\":" & (item 2 of cPos) & ",\"width\":" & (item 1 of cSize) & ",\"height\":" & (item 2 of cSize) & "}}"
                        set elemCount to elemCount + 1
                    end try
                end repeat
            end try
        end try
    end repeat
    set AppleScript's text item delimiters to ","
    return "[" & (res as string) & "]"
end tell
"#,
        win.app
    );

    #[derive(serde::Deserialize)]
    struct RawElement {
        role: String,
        name: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    }

    let mut elements: Vec<IndexedElement> = Vec::new();
    let mut tree_lines: Vec<String> = Vec::new();

    if let Ok(raw_json) = run_osascript(&script) {
        if let Ok(raw_elements) = serde_json::from_str::<Vec<RawElement>>(&raw_json) {
            for (idx, raw) in raw_elements.into_iter().enumerate() {
                let elem_idx = idx + 1;
                let line = format!(
                    "[{}] {} '{}' ({}, {}) {}x{}",
                    elem_idx, raw.role, raw.name, raw.x, raw.y, raw.width, raw.height
                );
                tree_lines.push(line);
                elements.push(IndexedElement {
                    index: elem_idx,
                    name: raw.name,
                    control_type: raw.role.clone(),
                    class_name: raw.role,
                    rect: WindowRect {
                        x: raw.x,
                        y: raw.y,
                        width: raw.width,
                        height: raw.height,
                    },
                    is_focused: false,
                    is_enabled: true,
                    value: None,
                });
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
    let win = get_window(id)?;
    let rect = win.rect.unwrap_or(WindowRect {
        x: 0,
        y: 0,
        width: 800,
        height: 600,
    });
    let screen_x = rect.x + x;
    let screen_y = rect.y + y;

    let script = format!(
        r#"
ObjC.import('CoreGraphics');
var pt = {{ x: {}, y: {} }};
var b = '{}';
var btnDown = b === 'right' ? $.kCGEventRightMouseDown : (b === 'middle' ? $.kCGEventOtherMouseDown : $.kCGEventLeftMouseDown);
var btnUp = b === 'right' ? $.kCGEventRightMouseUp : (b === 'middle' ? $.kCGEventOtherMouseUp : $.kCGEventLeftMouseUp);
var mouseBtn = b === 'right' ? $.kCGMouseButtonRight : (b === 'middle' ? $.kCGMouseButtonCenter : $.kCGMouseButtonLeft);
for (var i = 0; i < {}; i++) {{
    var d = $.CGEventCreateMouseEvent(null, btnDown, pt, mouseBtn);
    var u = $.CGEventCreateMouseEvent(null, btnUp, pt, mouseBtn);
    if ({} > 1) {{
        $.CGEventSetIntegerValueField(d, $.kCGMouseEventClickState, i + 1);
        $.CGEventSetIntegerValueField(u, $.kCGMouseEventClickState, i + 1);
    }}
    $.CGEventPost($.kCGHIDEventTap, d);
    $.CGEventPost($.kCGHIDEventTap, u);
}}
"#,
        screen_x, screen_y, button, count, count
    );

    run_jxa(&script)?;
    Ok(())
}

/// Click an indexed accessibility element.
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

    let script = format!(
        r#"
ObjC.import('CoreGraphics');
var pt = {{ x: {}, y: {} }};
var b = '{}';
var btnDown = b === 'right' ? $.kCGEventRightMouseDown : (b === 'middle' ? $.kCGEventOtherMouseDown : $.kCGEventLeftMouseDown);
var btnUp = b === 'right' ? $.kCGEventRightMouseUp : (b === 'middle' ? $.kCGEventOtherMouseUp : $.kCGEventLeftMouseUp);
var mouseBtn = b === 'right' ? $.kCGMouseButtonRight : (b === 'middle' ? $.kCGMouseButtonCenter : $.kCGMouseButtonLeft);
for (var i = 0; i < {}; i++) {{
    var d = $.CGEventCreateMouseEvent(null, btnDown, pt, mouseBtn);
    var u = $.CGEventCreateMouseEvent(null, btnUp, pt, mouseBtn);
    if ({} > 1) {{
        $.CGEventSetIntegerValueField(d, $.kCGMouseEventClickState, i + 1);
        $.CGEventSetIntegerValueField(u, $.kCGMouseEventClickState, i + 1);
    }}
    $.CGEventPost($.kCGHIDEventTap, d);
    $.CGEventPost($.kCGHIDEventTap, u);
}}
"#,
        center_x, center_y, button, count, count
    );

    run_jxa(&script)?;
    Ok(())
}

/// Type text into the active window.
pub fn type_text(
    id: usize,
    text: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    activate_window(id)?;
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        r#"
tell application "System Events"
    keystroke "{}"
end tell
"#,
        escaped
    );
    run_osascript(&script)?;
    Ok(())
}

/// Press a key chord (e.g. "Control_L+a", "cmd+c", "Return", "Tab", "Escape").
pub fn press_key(
    id: usize,
    chord: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    activate_window(id)?;
    let parts: Vec<&str> = chord.split('+').map(|s| s.trim()).collect();
    let mut modifiers: Vec<&str> = Vec::new();
    let mut key = "";

    for part in parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "ctrl" | "control" | "control_l" | "control_r" => modifiers.push("control down"),
            "cmd" | "command" | "super" | "win" => modifiers.push("command down"),
            "shift" | "shift_l" | "shift_r" => modifiers.push("shift down"),
            "alt" | "opt" | "option" => modifiers.push("option down"),
            _ => key = part,
        }
    }

    let mod_clause = if modifiers.is_empty() {
        "".to_string()
    } else {
        format!(" using {{{}}}", modifiers.join(", "))
    };

    let key_script = match key.to_lowercase().as_str() {
        "return" | "enter" => format!("key code 36{}", mod_clause),
        "tab" => format!("key code 48{}", mod_clause),
        "space" => format!("key code 49{}", mod_clause),
        "escape" | "esc" => format!("key code 53{}", mod_clause),
        "backspace" | "back" => format!("key code 51{}", mod_clause),
        "delete" | "del" => format!("key code 117{}", mod_clause),
        "up" => format!("key code 126{}", mod_clause),
        "down" => format!("key code 125{}", mod_clause),
        "left" => format!("key code 123{}", mod_clause),
        "right" => format!("key code 124{}", mod_clause),
        _ => {
            let escaped_key = key.replace('\\', "\\\\").replace('"', "\\\"");
            format!("keystroke \"{}\"{}", escaped_key, mod_clause)
        }
    };

    let script = format!(
        r#"
tell application "System Events"
    {}
end tell
"#,
        key_script
    );
    run_osascript(&script)?;
    Ok(())
}

/// Replace the value of an editable element.
pub fn set_value(
    id: usize,
    element_idx: usize,
    value: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    click_element(id, element_idx, "left", 1)?;
    press_key(id, "cmd+a")?;
    press_key(id, "backspace")?;
    type_text(id, value)?;
    Ok(())
}

/// Scroll with mouse wheel.
pub fn scroll(
    _id: usize,
    _x: i32,
    _y: i32,
    dx: i32,
    dy: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let script = format!(
        r#"
ObjC.import('CoreGraphics');
var scrollEvent = $.CGEventCreateScrollWheelEvent(null, $.kCGScrollEventUnitPixel, 2, {}, {});
$.CGEventPost($.kCGHIDEventTap, scrollEvent);
"#,
        dy, dx
    );
    run_jxa(&script)?;
    Ok(())
}

/// Drag mouse from one point to another.
pub fn drag(
    _id: usize,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let script = format!(
        r#"
ObjC.import('CoreGraphics');
var ptFrom = {{ x: {}, y: {} }};
var ptTo = {{ x: {}, y: {} }};
var down = $.CGEventCreateMouseEvent(null, $.kCGEventLeftMouseDown, ptFrom, $.kCGMouseButtonLeft);
var drag = $.CGEventCreateMouseEvent(null, $.kCGEventLeftMouseDragged, ptTo, $.kCGMouseButtonLeft);
var up = $.CGEventCreateMouseEvent(null, $.kCGEventLeftMouseUp, ptTo, $.kCGMouseButtonLeft);
$.CGEventPost($.kCGHIDEventTap, down);
$.CGEventPost($.kCGHIDEventTap, drag);
$.CGEventPost($.kCGHIDEventTap, up);
"#,
        from_x, from_y, to_x, to_y
    );
    run_jxa(&script)?;
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
