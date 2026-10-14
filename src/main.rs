mod mcp;
mod platform;
mod rpc;
mod session;
mod types;

use clap::{Parser, Subcommand};

use crate::platform::*;
use crate::session::save_elements;
use crate::types::WindowState;

#[derive(Parser, Debug)]
#[command(
    name = "computer-use",
    about = "Self-contained native Windows Computer Use runtime in Rust",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run in JSON-RPC server mode over stdin/stdout (Codex helper compatible)
    #[arg(long)]
    rpc: bool,

    /// Run in Model Context Protocol (MCP) server mode over stdin/stdout
    #[arg(long)]
    mcp: bool,
}

fn parse_hwnd(s: &str) -> Result<usize, String> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        usize::from_str_radix(&s[2..], 16).map_err(|e| format!("Invalid hex HWND: {}", e))
    } else {
        s.parse::<usize>().map_err(|e| format!("Invalid HWND: {}", e))
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List open desktop windows
    Windows {
        /// Output formatted JSON
        #[arg(long)]
        json: bool,
    },
    /// List running apps with open windows
    Apps {
        /// Output formatted JSON
        #[arg(long)]
        json: bool,
    },
    /// Get details of a specific window by HWND
    Window {
        /// Window handle (HWND in decimal or hex like 0x5707da)
        #[arg(value_parser = parse_hwnd)]
        id: usize,
        /// Output formatted JSON
        #[arg(long)]
        json: bool,
    },
    /// Bring a window to foreground (restoring if minimized)
    Activate {
        /// Window handle (HWND in decimal or hex like 0x5707da)
        #[arg(value_parser = parse_hwnd)]
        id: usize,
    },
    /// Launch an application
    Launch {
        /// Executable name, path, or URI protocol
        app: String,
    },
    /// Observe window state (screenshot, accessibility tree)
    State {
        /// Window handle (HWND in decimal or hex like 0x5707da)
        #[arg(value_parser = parse_hwnd)]
        id: usize,
        /// Include screenshot (default true)
        #[arg(long, default_value_t = true)]
        screenshot: bool,
        /// Include accessibility tree text (default true)
        #[arg(long, default_value_t = true)]
        tree: bool,
        /// Save screenshot directly to a PNG file path
        #[arg(long)]
        save: Option<String>,
        /// Output complete state as JSON
        #[arg(long)]
        json: bool,
    },
    /// Click inside a window (by element index or coordinates)
    Click {
        /// Window handle (HWND in decimal or hex like 0x5707da)
        #[arg(value_parser = parse_hwnd)]
        id: usize,
        /// Accessibility element index from the latest state observation
        #[arg(long, short)]
        element: Option<usize>,
        /// Window-relative X coordinate
        #[arg(long)]
        x: Option<i32>,
        /// Window-relative Y coordinate
        #[arg(long)]
        y: Option<i32>,
        /// Mouse button ("left", "right", "middle")
        #[arg(long, short, default_value = "left")]
        button: String,
        /// Number of clicks (1 for single click, 2 for double click)
        #[arg(long, short = 'c', default_value_t = 1)]
        count: usize,
    },
    /// Type text into the focused element
    Type {
        /// Window handle (HWND in decimal or hex like 0x5707da)
        #[arg(value_parser = parse_hwnd)]
        id: usize,
        /// Text string to type
        text: String,
    },
    /// Press a key chord (e.g. "Control_L+a", "Ctrl+C", "Return", "Tab", "Escape")
    Key {
        /// Window handle (HWND in decimal or hex like 0x5707da)
        #[arg(value_parser = parse_hwnd)]
        id: usize,
        /// Key or key chord
        chord: String,
    },
    /// Replace the value of an editable element
    SetValue {
        /// Window handle (HWND in decimal or hex like 0x5707da)
        #[arg(value_parser = parse_hwnd)]
        id: usize,
        /// Accessibility element index
        #[arg(long, short)]
        element: usize,
        /// Replacement value
        value: String,
    },
    /// Scroll with mouse wheel at coordinate
    Scroll {
        /// Window handle (HWND in decimal or hex like 0x5707da)
        #[arg(value_parser = parse_hwnd)]
        id: usize,
        /// Window-relative X coordinate
        #[arg(long, default_value_t = 0)]
        x: i32,
        /// Window-relative Y coordinate
        #[arg(long, default_value_t = 0)]
        y: i32,
        /// Horizontal scroll delta
        #[arg(long, default_value_t = 0)]
        dx: i32,
        /// Vertical scroll delta (negative = down, positive = up)
        #[arg(long, default_value_t = 0)]
        dy: i32,
    },
    /// Drag mouse from one point to another
    Drag {
        /// Window handle (HWND in decimal or hex like 0x5707da)
        #[arg(value_parser = parse_hwnd)]
        id: usize,
        #[arg(long)]
        from_x: i32,
        #[arg(long)]
        from_y: i32,
        #[arg(long)]
        to_x: i32,
        #[arg(long)]
        to_y: i32,
    },
    /// Perform secondary accessibility action on an element (Expand, Collapse, Toggle)
    Action {
        /// Window handle (HWND in decimal or hex like 0x5707da)
        #[arg(value_parser = parse_hwnd)]
        id: usize,
        /// Accessibility element index
        #[arg(long, short)]
        element: usize,
        /// Action name (e.g. "Expand", "Collapse", "Toggle")
        action: String,
    },
    /// Run as a JSON-RPC stdio daemon (Codex helper compatible)
    Rpc,
    /// Run as a Model Context Protocol (MCP) server
    Mcp,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    if cli.mcp {
        return mcp::run_mcp_loop();
    }
    if cli.rpc {
        return rpc::run_rpc_loop();
    }

    match cli.command {
        Some(Commands::Mcp) => mcp::run_mcp_loop(),
        Some(Commands::Rpc) => rpc::run_rpc_loop(),
        Some(Commands::Windows { json }) => {
            let wins = list_windows()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&wins)?);
            } else {
                println!("{:<12} {:<24} {}", "HWND", "APP", "TITLE");
                println!("{:-<12} {:-<24} {:-<40}", "", "", "");
                for w in wins {
                    println!(
                        "0x{:<10x} {:<24} {}",
                        w.id,
                        w.app,
                        w.title.unwrap_or_else(|| "<no title>".to_string())
                    );
                }
            }
            Ok(())
        }
        Some(Commands::Apps { json }) => {
            let apps = list_apps()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&apps)?);
            } else {
                for app in apps {
                    println!(
                        "App: {} ({}) [windows: {}]",
                        app.id,
                        app.display_name.unwrap_or_default(),
                        app.windows.len()
                    );
                    for w in app.windows {
                        println!(
                            "  - HWND: 0x{:x} | Title: '{}'",
                            w.id,
                            w.title.unwrap_or_default()
                        );
                    }
                }
            }
            Ok(())
        }
        Some(Commands::Window { id, json }) => {
            let win = get_window(id)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&win)?);
            } else {
                println!("Window 0x{:x}:", win.id);
                println!("  App:   {}", win.app);
                println!("  Title: {}", win.title.unwrap_or_default());
                if let Some(r) = win.rect {
                    println!("  Rect:  ({}, {}) {}x{}", r.x, r.y, r.width, r.height);
                }
            }
            Ok(())
        }
        Some(Commands::Activate { id }) => {
            activate_window(id)?;
            println!("Window 0x{:x} activated", id);
            Ok(())
        }
        Some(Commands::Launch { app }) => {
            launch_app(&app)?;
            println!("Launched application: {}", app);
            Ok(())
        }
        Some(Commands::State {
            id,
            screenshot,
            tree,
            save,
            json,
        }) => {
            let win = get_window(id)?;

            let mut screenshots = Vec::new();
            if screenshot {
                if let Some(ref save_path) = save {
                    let (w, h) = capture_window_to_file(id, save_path)?;
                    println!("Saved screenshot ({}x{}) to {}", w, h, save_path);
                }
                let shot = capture_window(id, "screenshot-0")?;
                screenshots.push(shot);
            }

            let mut accessibility_state = None;
            if tree {
                let (acc, elements) = inspect_accessibility(id)?;
                save_elements(id, elements);
                accessibility_state = Some(acc);
            }

            let state = WindowState {
                window: win,
                screenshots,
                accessibility: accessibility_state,
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&state)?);
            } else {
                println!("Window: 0x{:x} ({})", state.window.id, state.window.app);
                if let Some(ref acc) = state.accessibility {
                    println!("\nAccessibility Tree:\n{}", acc.tree);
                    if let Some(ref f) = acc.focused_element {
                        println!("\nFocused: {}", f);
                    }
                }
                if !state.screenshots.is_empty() {
                    let s = &state.screenshots[0];
                    println!(
                        "\nScreenshot: {} ({}x{})",
                        s.id,
                        s.width.unwrap_or(0),
                        s.height.unwrap_or(0)
                    );
                }
            }
            Ok(())
        }
        Some(Commands::Click {
            id,
            element,
            x,
            y,
            button,
            count,
        }) => {
            if let Some(elem_idx) = element {
                click_element(id, elem_idx, &button, count)?;
                println!(
                    "Clicked element [{}] in window 0x{:x} with {} button (count: {})",
                    elem_idx, id, button, count
                );
            } else if let (Some(x_pos), Some(y_pos)) = (x, y) {
                click_at(id, x_pos, y_pos, &button, count)?;
                println!(
                    "Clicked coordinates ({}, {}) in window 0x{:x} with {} button (count: {})",
                    x_pos, y_pos, id, button, count
                );
            } else {
                return Err("Must specify either --element <index> or both --x and --y".into());
            }
            Ok(())
        }
        Some(Commands::Type { id, text }) => {
            type_text(id, &text)?;
            println!("Typed {} characters into window 0x{:x}", text.len(), id);
            Ok(())
        }
        Some(Commands::Key { id, chord }) => {
            press_key(id, &chord)?;
            println!("Pressed key chord '{}' in window 0x{:x}", chord, id);
            Ok(())
        }
        Some(Commands::SetValue { id, element, value }) => {
            set_value(id, element, &value)?;
            println!(
                "Set value of element [{}] in window 0x{:x} to '{}'",
                element, id, value
            );
            Ok(())
        }
        Some(Commands::Scroll { id, x, y, dx, dy }) => {
            scroll(id, x, y, dx, dy)?;
            println!("Scrolled window 0x{:x} at ({}, {}): dx={}, dy={}", id, x, y, dx, dy);
            Ok(())
        }
        Some(Commands::Drag {
            id,
            from_x,
            from_y,
            to_x,
            to_y,
        }) => {
            drag(id, from_x, from_y, to_x, to_y)?;
            println!(
                "Dragged in window 0x{:x} from ({}, {}) to ({}, {})",
                id, from_x, from_y, to_x, to_y
            );
            Ok(())
        }
        Some(Commands::Action { id, element, action }) => {
            perform_secondary_action(id, element, &action)?;
            println!(
                "Invoked action '{}' on element [{}] in window 0x{:x}",
                action, element, id
            );
            Ok(())
        }
        None => {
            // Default to RPC mode if no subcommand is given
            rpc::run_rpc_loop()
        }
    }
}
