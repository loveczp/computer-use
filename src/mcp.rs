use std::io::{self, BufRead, Write};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::platform::*;
use crate::session::save_elements;
use crate::types::WindowState;

#[derive(Debug, Deserialize)]
struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

pub fn run_mcp_loop() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: McpRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                });
                writeln!(stdout, "{}", err_resp)?;
                stdout.flush()?;
                continue;
            }
        };

        let resp_id = req.id.unwrap_or(Value::Null);

        match req.method.as_str() {
            "initialize" => {
                let init_result = json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "computer-use",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                });
                let resp = McpResponse {
                    jsonrpc: "2.0",
                    id: resp_id,
                    result: Some(init_result),
                    error: None,
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            }
            "notifications/initialized" => {
                // No response for notifications
            }
            "tools/list" => {
                let tools = get_tool_definitions();
                let resp = McpResponse {
                    jsonrpc: "2.0",
                    id: resp_id,
                    result: Some(json!({ "tools": tools })),
                    error: None,
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            }
            "tools/call" => {
                let tool_name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = req.params.get("arguments").cloned().unwrap_or(json!({}));

                let call_result = handle_tool_call(tool_name, &args);
                match call_result {
                    Ok(content) => {
                        let resp = McpResponse {
                            jsonrpc: "2.0",
                            id: resp_id,
                            result: Some(json!({
                                "content": content,
                                "isError": false
                            })),
                            error: None,
                        };
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    }
                    Err(err_msg) => {
                        let resp = McpResponse {
                            jsonrpc: "2.0",
                            id: resp_id,
                            result: Some(json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!("Error: {}", err_msg)
                                }],
                                "isError": true
                            })),
                            error: None,
                        };
                        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    }
                }
            }
            _ => {
                let resp = McpResponse {
                    jsonrpc: "2.0",
                    id: resp_id,
                    result: None,
                    error: Some(json!({
                        "code": -32601,
                        "message": format!("Method not found: {}", req.method)
                    })),
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            }
        }
        stdout.flush()?;
    }

    Ok(())
}

fn get_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "computer_use_list_windows",
            "description": "List all targetable open top-level desktop windows with their IDs, process names, and titles.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "computer_use_list_apps",
            "description": "List running apps and group their open targetable windows.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "computer_use_activate_window",
            "description": "Bring an open window to the foreground, restoring it if minimized.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "The window handle (HWND) ID." }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "computer_use_launch_app",
            "description": "Launch an application by executable name, path, or URI protocol.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": "Application name (e.g. 'notepad.exe'), path, or URI protocol." }
                },
                "required": ["app"]
            }
        }),
        json!({
            "name": "computer_use_get_window_state",
            "description": "Observe an open window: captures a screenshot (as data URL image) and/or an accessibility tree of visible UI elements with element indices.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "The window handle (HWND) ID." },
                    "include_screenshot": { "type": "boolean", "description": "Whether to capture screenshot (default true)." },
                    "include_text": { "type": "boolean", "description": "Whether to capture accessibility tree text (default true)." }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "computer_use_click",
            "description": "Click an element by its accessibility element index or window-relative coordinates (x, y).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "Target window handle (HWND) ID." },
                    "element_index": { "type": "number", "description": "Element index from the latest get_window_state accessibility tree." },
                    "x": { "type": "number", "description": "Window-relative X coordinate (when element_index is not provided)." },
                    "y": { "type": "number", "description": "Window-relative Y coordinate (when element_index is not provided)." },
                    "mouse_button": { "type": "string", "enum": ["left", "right", "middle"], "description": "Mouse button (default 'left')." },
                    "click_count": { "type": "number", "description": "Number of clicks (1 for single click, 2 for double click)." }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "computer_use_type_text",
            "description": "Type text into the currently focused element of a window.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "Target window handle (HWND) ID." },
                    "text": { "type": "string", "description": "Text to type." }
                },
                "required": ["id", "text"]
            }
        }),
        json!({
            "name": "computer_use_press_key",
            "description": "Press a key or '+'-separated chord (e.g. 'Control_L+a', 'Ctrl+C', 'Return', 'Tab', 'Escape', 'Alt+F4').",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "Target window handle (HWND) ID." },
                    "key": { "type": "string", "description": "Key or key chord." }
                },
                "required": ["id", "key"]
            }
        }),
        json!({
            "name": "computer_use_set_value",
            "description": "Replace the value of an editable element identified by its accessibility index.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "Target window handle (HWND) ID." },
                    "element_index": { "type": "number", "description": "Element index from get_window_state." },
                    "value": { "type": "string", "description": "Replacement text value." }
                },
                "required": ["id", "element_index", "value"]
            }
        }),
        json!({
            "name": "computer_use_scroll",
            "description": "Scroll at a coordinate in a window.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "Target window handle (HWND) ID." },
                    "x": { "type": "number", "description": "Window-relative X coordinate." },
                    "y": { "type": "number", "description": "Window-relative Y coordinate." },
                    "scrollX": { "type": "number", "description": "Horizontal delta." },
                    "scrollY": { "type": "number", "description": "Vertical delta (negative = scroll down, positive = scroll up)." }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "computer_use_drag",
            "description": "Drag the mouse from (from_x, from_y) to (to_x, to_y) in a window.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "Target window handle (HWND) ID." },
                    "from_x": { "type": "number" },
                    "from_y": { "type": "number" },
                    "to_x": { "type": "number" },
                    "to_y": { "type": "number" }
                },
                "required": ["id", "from_x", "from_y", "to_x", "to_y"]
            }
        }),
        json!({
            "name": "computer_use_perform_secondary_action",
            "description": "Invoke an action like 'Expand', 'Collapse', or 'Toggle' on an element.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "number", "description": "Target window handle (HWND) ID." },
                    "element_index": { "type": "number", "description": "Element index." },
                    "action": { "type": "string", "description": "Action name." }
                },
                "required": ["id", "element_index", "action"]
            }
        })
    ]
}

fn handle_tool_call(name: &str, args: &Value) -> Result<Vec<Value>, String> {
    match name {
        "computer_use_list_windows" => {
            let wins = list_windows().map_err(|e| e.to_string())?;
            let formatted = serde_json::to_string_pretty(&wins).map_err(|e| e.to_string())?;
            Ok(vec![json!({ "type": "text", "text": formatted })])
        }
        "computer_use_list_apps" => {
            let apps = list_apps().map_err(|e| e.to_string())?;
            let formatted = serde_json::to_string_pretty(&apps).map_err(|e| e.to_string())?;
            Ok(vec![json!({ "type": "text", "text": formatted })])
        }
        "computer_use_activate_window" => {
            let id = args.get("id").and_then(|v| v.as_u64()).ok_or("Missing 'id'")? as usize;
            activate_window(id).map_err(|e| e.to_string())?;
            Ok(vec![json!({ "type": "text", "text": format!("Window {} activated", id) })])
        }
        "computer_use_launch_app" => {
            let app = args.get("app").and_then(|v| v.as_str()).ok_or("Missing 'app'")?;
            launch_app(app).map_err(|e| e.to_string())?;
            Ok(vec![json!({ "type": "text", "text": format!("Launched '{}'", app) })])
        }
        "computer_use_get_window_state" => {
            let id = args.get("id").and_then(|v| v.as_u64()).ok_or("Missing 'id'")? as usize;

            let include_screenshot = args.get("include_screenshot").and_then(|v| v.as_bool()).unwrap_or(true);
            let include_text = args.get("include_text").and_then(|v| v.as_bool()).unwrap_or(true);

            let win = get_window(id).map_err(|e| e.to_string())?;

            let mut content = Vec::new();

            let mut screenshots = Vec::new();
            if include_screenshot {
                let shot = capture_window(id, "screenshot-0").map_err(|e| e.to_string())?;
                // Add image to MCP content
                if let Some(pos) = shot.url.find(',') {
                    let base64_data = &shot.url[pos + 1..];
                    content.push(json!({
                        "type": "image",
                        "data": base64_data,
                        "mimeType": "image/png"
                    }));
                }
                screenshots.push(shot);
            }

            let mut accessibility_state = None;
            if include_text {
                let (acc, elements) = inspect_accessibility(id).map_err(|e| e.to_string())?;
                save_elements(id, elements);
                content.push(json!({
                    "type": "text",
                    "text": format!("Accessibility Tree:\n{}", acc.tree)
                }));
                accessibility_state = Some(acc);
            }

            let state = WindowState {
                window: win,
                screenshots,
                accessibility: accessibility_state,
            };

            let state_json = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
            content.push(json!({
                "type": "text",
                "text": format!("Window State JSON:\n{}", state_json)
            }));

            Ok(content)
        }
        "computer_use_click" => {
            let id = args.get("id").and_then(|v| v.as_u64()).ok_or("Missing 'id'")? as usize;
            let button = args.get("mouse_button").and_then(|v| v.as_str()).unwrap_or("left");
            let count = args.get("click_count").and_then(|v| v.as_u64()).unwrap_or(1) as usize;

            if let Some(elem_idx) = args.get("element_index").and_then(|v| v.as_u64()) {
                click_element(id, elem_idx as usize, button, count).map_err(|e| e.to_string())?;
                Ok(vec![json!({ "type": "text", "text": format!("Clicked element [{}] with {} button (count: {})", elem_idx, button, count) })])
            } else {
                let x = args.get("x").and_then(|v| v.as_i64()).ok_or("Missing 'x' coordinate")? as i32;
                let y = args.get("y").and_then(|v| v.as_i64()).ok_or("Missing 'y' coordinate")? as i32;
                click_at(id, x, y, button, count).map_err(|e| e.to_string())?;
                Ok(vec![json!({ "type": "text", "text": format!("Clicked coordinates ({}, {}) with {} button (count: {})", x, y, button, count) })])
            }
        }
        "computer_use_type_text" => {
            let id = args.get("id").and_then(|v| v.as_u64()).ok_or("Missing 'id'")? as usize;
            let text = args.get("text").and_then(|v| v.as_str()).ok_or("Missing 'text'")?;
            type_text(id, text).map_err(|e| e.to_string())?;
            Ok(vec![json!({ "type": "text", "text": format!("Typed text ({} chars)", text.len()) })])
        }
        "computer_use_press_key" => {
            let id = args.get("id").and_then(|v| v.as_u64()).ok_or("Missing 'id'")? as usize;
            let key = args.get("key").and_then(|v| v.as_str()).ok_or("Missing 'key'")?;
            press_key(id, key).map_err(|e| e.to_string())?;
            Ok(vec![json!({ "type": "text", "text": format!("Pressed key chord '{}'", key) })])
        }
        "computer_use_set_value" => {
            let id = args.get("id").and_then(|v| v.as_u64()).ok_or("Missing 'id'")? as usize;
            let elem_idx = args.get("element_index").and_then(|v| v.as_u64()).ok_or("Missing 'element_index'")? as usize;
            let value = args.get("value").and_then(|v| v.as_str()).ok_or("Missing 'value'")?;
            set_value(id, elem_idx, value).map_err(|e| e.to_string())?;
            Ok(vec![json!({ "type": "text", "text": format!("Set element [{}] value to '{}'", elem_idx, value) })])
        }
        "computer_use_scroll" => {
            let id = args.get("id").and_then(|v| v.as_u64()).ok_or("Missing 'id'")? as usize;
            let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let scroll_x = args.get("scrollX").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let scroll_y = args.get("scrollY").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            scroll(id, x, y, scroll_x, scroll_y).map_err(|e| e.to_string())?;
            Ok(vec![json!({ "type": "text", "text": format!("Scrolled at ({}, {}): dx={}, dy={}", x, y, scroll_x, scroll_y) })])
        }
        "computer_use_drag" => {
            let id = args.get("id").and_then(|v| v.as_u64()).ok_or("Missing 'id'")? as usize;
            let from_x = args.get("from_x").and_then(|v| v.as_i64()).ok_or("Missing from_x")? as i32;
            let from_y = args.get("from_y").and_then(|v| v.as_i64()).ok_or("Missing from_y")? as i32;
            let to_x = args.get("to_x").and_then(|v| v.as_i64()).ok_or("Missing to_x")? as i32;
            let to_y = args.get("to_y").and_then(|v| v.as_i64()).ok_or("Missing to_y")? as i32;
            drag(id, from_x, from_y, to_x, to_y).map_err(|e| e.to_string())?;
            Ok(vec![json!({ "type": "text", "text": format!("Dragged from ({}, {}) to ({}, {})", from_x, from_y, to_x, to_y) })])
        }
        "computer_use_perform_secondary_action" => {
            let id = args.get("id").and_then(|v| v.as_u64()).ok_or("Missing 'id'")? as usize;
            let elem_idx = args.get("element_index").and_then(|v| v.as_u64()).ok_or("Missing 'element_index'")? as usize;
            let action = args.get("action").and_then(|v| v.as_str()).ok_or("Missing 'action'")?;
            perform_secondary_action(id, elem_idx, action).map_err(|e| e.to_string())?;
            Ok(vec![json!({ "type": "text", "text": format!("Invoked action '{}' on element [{}]", action, elem_idx) })])
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}
