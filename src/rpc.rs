use std::io::{self, BufRead, Write};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::platform::*;
use crate::session::save_elements;
use crate::types::WindowState;

#[derive(Debug, Deserialize)]
struct RpcRequest {
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct RpcSuccessResponse<'a> {
    id: &'a Value,
    ok: bool,
    result: Value,
}

#[derive(Debug, Serialize)]
struct RpcErrorResponse<'a> {
    id: &'a Value,
    ok: bool,
    error: String,
}

/// Run the stdio line-delimited JSON-RPC loop.
pub fn run_rpc_loop() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

        let req: RpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = serde_json::json!({
                    "id": null,
                    "ok": false,
                    "error": format!("Invalid JSON request: {}", e)
                });
                writeln!(stdout, "{}", err_resp)?;
                stdout.flush()?;
                continue;
            }
        };

        let result = handle_rpc_method(&req.method, &req.params);

        match result {
            Ok(val) => {
                let resp = RpcSuccessResponse {
                    id: &req.id,
                    ok: true,
                    result: val,
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            }
            Err(err_msg) => {
                let resp = RpcErrorResponse {
                    id: &req.id,
                    ok: false,
                    error: err_msg,
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            }
        }
        stdout.flush()?;
    }

    Ok(())
}

fn handle_rpc_method(method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "list_windows" => {
            let wins = list_windows().map_err(|e| e.to_string())?;
            serde_json::to_value(wins).map_err(|e| e.to_string())
        }
        "list_apps" => {
            let apps = list_apps().map_err(|e| e.to_string())?;
            serde_json::to_value(apps).map_err(|e| e.to_string())
        }
        "get_window" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "Missing 'id' parameter".to_string())? as usize;
            let win = get_window(id).map_err(|e| e.to_string())?;
            serde_json::to_value(win).map_err(|e| e.to_string())
        }
        "activate_window" => {
            let id = extract_window_id(params)?;
            activate_window(id).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "launch_app" => {
            let app = params
                .get("app")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'app' parameter".to_string())?;
            launch_app(app).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "get_window_state" => {
            let id = extract_window_id(params)?;

            let include_screenshot = params
                .get("include_screenshot")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let include_text = params
                .get("include_text")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let win = get_window(id).map_err(|e| e.to_string())?;

            let mut screenshots = Vec::new();
            if include_screenshot {
                let shot = capture_window(id, "screenshot-0").map_err(|e| e.to_string())?;
                screenshots.push(shot);
            }

            let mut accessibility_state = None;
            if include_text {
                let (acc, elements) = inspect_accessibility(id).map_err(|e| e.to_string())?;
                save_elements(id, elements);
                accessibility_state = Some(acc);
            }

            let state = WindowState {
                window: win,
                screenshots,
                accessibility: accessibility_state,
            };

            serde_json::to_value(state).map_err(|e| e.to_string())
        }
        "click" => {
            let id = extract_window_id(params)?;

            if let Some(elem_idx) = params.get("element_index").and_then(|v| v.as_u64()) {
                let button = params.get("mouse_button").and_then(|v| v.as_str()).unwrap_or("left");
                let count = params.get("click_count").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                click_element(id, elem_idx as usize, button, count).map_err(|e| e.to_string())?;
            } else {
                let x = params.get("x").and_then(|v| v.as_i64()).ok_or("Missing x coordinate")? as i32;
                let y = params.get("y").and_then(|v| v.as_i64()).ok_or("Missing y coordinate")? as i32;
                let button = params.get("mouse_button").and_then(|v| v.as_str()).unwrap_or("left");
                let count = params.get("click_count").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                click_at(id, x, y, button, count).map_err(|e| e.to_string())?;
            }

            Ok(Value::Null)
        }
        "click_element" => {
            let id = extract_window_id(params)?;
            let elem_idx = params
                .get("element_index")
                .and_then(|v| v.as_u64())
                .ok_or("Missing 'element_index'")? as usize;
            let button = params.get("mouse_button").and_then(|v| v.as_str()).unwrap_or("left");
            let count = params.get("click_count").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            click_element(id, elem_idx, button, count).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "type_text" => {
            let id = extract_window_id(params)?;
            let text = params.get("text").and_then(|v| v.as_str()).ok_or("Missing 'text'")?;
            type_text(id, text).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "press_key" => {
            let id = extract_window_id(params)?;
            let key = params.get("key").and_then(|v| v.as_str()).ok_or("Missing 'key'")?;
            press_key(id, key).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "set_value" => {
            let id = extract_window_id(params)?;
            let elem_idx = params
                .get("element_index")
                .and_then(|v| v.as_u64())
                .ok_or("Missing 'element_index'")? as usize;
            let value = params.get("value").and_then(|v| v.as_str()).ok_or("Missing 'value'")?;
            set_value(id, elem_idx, value).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "scroll" => {
            let id = extract_window_id(params)?;
            let x = params.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = params.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let scroll_x = params.get("scrollX").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let scroll_y = params.get("scrollY").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            scroll(id, x, y, scroll_x, scroll_y).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "drag" => {
            let id = extract_window_id(params)?;
            let from_x = params.get("from_x").and_then(|v| v.as_i64()).ok_or("Missing from_x")? as i32;
            let from_y = params.get("from_y").and_then(|v| v.as_i64()).ok_or("Missing from_y")? as i32;
            let to_x = params.get("to_x").and_then(|v| v.as_i64()).ok_or("Missing to_x")? as i32;
            let to_y = params.get("to_y").and_then(|v| v.as_i64()).ok_or("Missing to_y")? as i32;
            drag(id, from_x, from_y, to_x, to_y).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "perform_secondary_action" => {
            let id = extract_window_id(params)?;
            let elem_idx = params
                .get("element_index")
                .and_then(|v| v.as_u64())
                .ok_or("Missing 'element_index'")? as usize;
            let action = params.get("action").and_then(|v| v.as_str()).ok_or("Missing 'action'")?;
            perform_secondary_action(id, elem_idx, action).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "close" | "end_turn" => Ok(Value::Null),
        other => Err(format!("Unknown method: {}", other)),
    }
}

fn extract_window_id(params: &Value) -> Result<usize, String> {
    if let Some(id) = params.get("id").and_then(|v| v.as_u64()) {
        return Ok(id as usize);
    }
    if let Some(win) = params.get("window") {
        if let Some(id) = win.get("id").and_then(|v| v.as_u64()) {
            return Ok(id as usize);
        }
    }
    Err("Cannot determine target window id from parameters".to_string())
}
