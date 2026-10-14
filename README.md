# Cross-Platform Native Computer Use (Rust)

A completely **self-contained**, high-performance desktop automation runtime implemented in **Rust** for **Windows, macOS, and Linux**.

Designed as a drop-in, zero-dependency replacement for OpenAI Codex's Computer Use runtime (`@oai/sky` / `codex-computer-use`), eliminating all requirements for Node.js, npm, proprietary Codex desktop app instances, Swift DLLs, and named pipe bridges.

---

## Supported Operating Systems

| OS | Window Discovery & Activation | Screenshot Capture | Accessibility Inspection | Input Injection |
|---|---|---|---|---|
| **Windows** | `EnumDesktopWindows`, `SetForegroundWindow`, `AttachThreadInput`, `WinSta0\Default` desktop auto-switch | `PrintWindow` (`PW_RENDERFULLCONTENT`) + `BitBlt` GDI fallback | `IUIAutomation` COM tree traversal | Native Win32 `SendInput` (Unicode, mouse, keyboard chords) |
| **macOS** | Quartz CoreGraphics via JXA (`CGWindowListCopyWindowInfo`), AppleScript System Events | Native `screencapture -l <id> -o -x` | AppleScript System Events UI element hierarchy traversal | CoreGraphics `CGEvent` mouse/scroll + AppleScript keystrokes/chords |
| **Linux** | `wmctrl -l -p -G` + `/proc` inspection, `xdotool` fallback | ImageMagick `import`, `maim`, or `xwd` + `convert` | `xwininfo -tree` & `xprop` structural hierarchy | `xdotool` (mouse clicks, drags, wheel scrolling, typing, key chords) |

---

## Highlights

- **Zero External Runtime Dependencies**: Compiles into a single standalone binary (~1.4 MB).
- **Interactive Desktop Attachment (Windows)**: Automatically attaches background/sandboxed threads to `WinSta0\Default`, allowing agent shells to control real GUI applications without session isolation issues.
- **Java Swing & Custom Canvas Support**: Seamlessly automates custom canvas applications (e.g. JetBrains PyCharm, IntelliJ IDEA, NetBeans) alongside native apps (Notepad, Chrome, Terminal).
- **Resolution-Independent Accessibility**: Traverses application trees, generating numbered element indices (`[1]`, `[2]`, ...) that survive window movements and UI theme changes.
- **Triple Interface**:
  1. **CLI Mode**: Fast, single-command shell actions (`computer-use windows`, `computer-use state <id>`, `computer-use click <id> --element <idx>`).
  2. **JSON-RPC Mode (`computer-use rpc`)**: Line-delimited JSON-RPC over stdin/stdout compatible with OpenAI Codex's `@oai/sky` protocol.
  3. **MCP Mode (`computer-use mcp`)**: Standard Model Context Protocol (MCP) server over stdio for any MCP client (Antigravity, Claude Desktop, Cursor, etc.).

---

## Building from Source

The codebase uses standard Rust edition 2024 with platform-conditional dependencies (`Cargo.toml`). It compiles out-of-the-box on all three platforms without requiring third-party C/C++ libraries.

### Windows
```pwsh
cargo build --release
# Output: target\release\computer-use.exe
```

### macOS
```bash
cargo build --release
# Output: target/release/computer-use
```

### Linux
```bash
# Recommended standard desktop utilities:
sudo apt-get install wmctrl xdotool imagemagick

cargo build --release
# Output: target/release/computer-use
```

---

## CLI Usage & Examples

Window identifiers can be specified in hexadecimal (e.g. `0x4b155c` on Windows, `0x03a00003` on Linux) or decimal.

### 1. Window & App Discovery

```bash
# List all visible windows
computer-use windows

# Output as formatted JSON
computer-use windows --json

# List running applications and their open windows
computer-use apps
```

### 2. Targeting & Window Activation

```bash
# Bring a window to foreground (and restore if minimized)
computer-use activate 0x4b155c

# Launch an application
computer-use launch notepad.exe    # Windows
computer-use launch "PyCharm"      # macOS / Linux
```

### 3. Observing State

```bash
# Observe window: print accessibility tree and capture screenshot
computer-use state 0x4b155c

# Save screenshot directly to a PNG file
computer-use state 0x4b155c --save screenshot.png

# Output complete state (including base64 screenshot data URL) as JSON
computer-use state 0x4b155c --json
```

Example accessibility output:
```text
Window: 0x4b155c (notepad.exe)

Accessibility Tree:
[1] Window: "Untitled - Notepad"
  [2] Pane: "NotepadTextBox"
    [3] Document: "Text editor" [focused]
  [4] Pane: "Microsoft.UI.Content.DesktopChildSiteBridge"
    [13] MenuBar: ""
      [14] MenuItem: "File"
      [15] MenuItem: "Edit"
      [16] MenuItem: "View"
```

### 4. Sending Input

```bash
# Click by element index (from latest state observation)
computer-use click 0x4b155c --element 14

# Click by window-relative coordinates (x, y)
computer-use click 0x4b155c --x 250 --y 180

# Double-click or right-click
computer-use click 0x4b155c --x 250 --y 180 --count 2
computer-use click 0x4b155c --element 3 --button right

# Type Unicode text into active focus
computer-use type 0x4b155c "Self-contained Rust Computer Use works!"

# Press key chords
computer-use key 0x4b155c "Control_L+a"
computer-use key 0x4b155c "Ctrl+Shift+A"
computer-use key 0x4b155c "Return"
computer-use key 0x4b155c "Escape"

# Replace value of an editable element
computer-use set-value 0x4b155c --element 3 "Replaced value"

# Mouse wheel scroll
computer-use scroll 0x4b155c --x 300 --y 300 --dy -120

# Drag mouse
computer-use drag 0x4b155c --from-x 100 --from-y 100 --to-x 400 --to-y 400
```

---

## Line-Delimited JSON-RPC Mode

To integrate with OpenAI Codex or any standard subprocess runner, use `computer-use rpc`. It reads JSON-RPC requests line-by-line from `stdin` and writes JSON replies to `stdout`:

```bash
# Request
{"id": 1, "method": "list_windows", "params": {}}

# Response
{"id": 1, "ok": true, "result": [{"id": 4920668, "app": "notepad.exe", "title": "Untitled - Notepad"}]}
```

Supported methods: `list_windows`, `list_apps`, `get_window`, `activate_window`, `launch_app`, `get_window_state`, `click`, `click_element`, `type_text`, `press_key`, `set_value`, `scroll`, `drag`, `perform_secondary_action`, `close`.

---

## Model Context Protocol (MCP) Mode

Run as an MCP stdio server:

```bash
computer-use mcp
```

Provides full MCP tools:
- `computer_use_list_windows`
- `computer_use_list_apps`
- `computer_use_activate_window`
- `computer_use_launch_app`
- `computer_use_get_window_state`
- `computer_use_click`
- `computer_use_type_text`
- `computer_use_press_key`
- `computer_use_set_value`
- `computer_use_scroll`
- `computer_use_drag`
- `computer_use_perform_secondary_action`

---

## License

MIT OR Apache-2.0
