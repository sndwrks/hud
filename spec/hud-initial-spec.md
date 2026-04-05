# SNDWRKS HUD — Tauri App Plan

> A cross-platform OSC-driven heads-up display for live events, inspired by [Figure53/QDisplay](https://github.com/Figure53/QDisplay).

---

## 1. Concept

QDisplay is a macOS-only app that shows a floating text window controlled via AppleScript, built for QLab workflows. **SNDWRKS HUD** replaces AppleScript with **OSC (Open Sound Control)** over UDP (and optionally TCP), making it cross-platform (macOS, Windows, Linux) and controllable from any OSC-capable software — QLab, EOS, Bitfocus Companion, TouchOSC, custom scripts, etc.

### Core behavior
- A **HUD window** (always-on-top, borderless, dark background) shows large text.
- A **settings window** (normal app window) configures the OSC ports, HUD dimensions, default text color, and other preferences.
- Incoming OSC messages update what the HUD window shows.

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────┐
│  Rust Backend (Tauri core)                          │
│                                                     │
│  ┌───────────────┐    ┌──────────────────────────┐  │
│  │ OSC Listener  │───▶│ Message Dispatcher       │  │
│  │ UDP :52100    │    │ - parse address pattern  │  │
│  │ TCP :52101    │    │ - extract args            │  │
│  └───────────────┘    │ - emit to frontend        │  │
│                       └──────────────────────────┘  │
│                                                     │
│  ┌───────────────┐    ┌──────────────────────────┐  │
│  │ Config Store  │    │ Window Manager           │  │
│  │ (JSON file)   │    │ - HUD window             │  │
│  └───────────────┘    │ - settings window        │  │
│                       └──────────────────────────┘  │
└─────────────────────────────────────────────────────┘
          │  Tauri events (emit/listen)
          ▼
┌─────────────────────────────────────────────────────┐
│  Frontend (WebView)                                 │
│                                                     │
│  ┌────────────────────┐  ┌───────────────────────┐  │
│  │ HUD Window         │  │ Settings Window       │  │
│  │ - message text     │  │ - OSC UDP/TCP ports   │  │
│  │ - color            │  │ - HUD W × H           │  │
│  │ - flash animation  │  │ - default color       │  │
│  │ - multi-line       │  │ - font size mode      │  │
│  └────────────────────┘  └───────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### Tech stack

| Layer       | Choice                | Rationale                                           |
| ----------- | --------------------- | --------------------------------------------------- |
| Framework   | **Tauri v2**          | Small binary, native windows, Rust backend          |
| Frontend    | **Vanilla HTML/CSS/JS** or **Solid/React** | HUD window is trivial; settings is a small form. Vanilla keeps deps minimal, but a framework is fine too. |
| OSC parsing | **`rosc`** (Rust)     | Mature, no-std-compatible OSC codec                 |
| UDP socket  | **`tokio` + `tokio::net::UdpSocket`** | Async UDP listener in the Tauri async runtime |
| TCP socket  | **`tokio::net::TcpListener`** | Async TCP listener, SLIP-framed OSC packets   |
| Config      | **`tauri-plugin-store`** or raw serde JSON | Persist settings between launches             |

---

## 3. OSC API

All addresses live under the `/sndwrks/hud/` namespace.

### 3.1 Single message

```
/sndwrks/hud/message/single  <string: message> [string: color]
```

Displays a single line of text. Replaces whatever is currently shown.

| Arg | Type   | Required | Description                          |
| --- | ------ | -------- | ------------------------------------ |
| 1   | String | ✓        | The message text                     |
| 2   | String |          | CSS-compatible color (name or hex). Falls back to default color from settings. |

**Examples:**
```
/sndwrks/hud/message/single "CUE 42 GO"
/sndwrks/hud/message/single "STANDBY" "red"
/sndwrks/hud/message/single "ALL CLEAR" "#00ff88"
```

### 3.2 Multiple messages (stacked lines)

```
/sndwrks/hud/message/lines  <string: msg1> <string: msg2> ... [string: color]
```

Displays multiple lines of text stacked vertically. The **last** argument is treated as color **only if** it matches a recognized color pattern (named CSS color or `#hex`). Otherwise all arguments are treated as message lines.

| Arg   | Type   | Required | Description                     |
| ----- | ------ | -------- | ------------------------------- |
| 1..N  | String | ✓ (≥1)  | Message lines                   |
| last  | String |          | Color, if it parses as one      |

**Examples:**
```
/sndwrks/hud/message/lines "Line 1" "Line 2" "Line 3"
/sndwrks/hud/message/lines "ACT II" "Scene 3" "yellow"
```

> **Color detection heuristic:** if the final string starts with `#` and is 4 or 7 chars, or matches a set of ~20 named colors (red, green, blue, yellow, cyan, magenta, white, orange, purple, pink, lime, aqua, coral, gold, salmon, teal, violet, indigo, maroon, navy), treat it as color. Otherwise treat it as another message line.

### 3.3 Flash message

```
/sndwrks/hud/message/flash  <string: message> <float: duration_s> [string: color]
```

Displays a message, flashes the window background the specified color (or a bright flash color), then returns to normal after `duration_s` seconds.

| Arg | Type   | Required | Description                                 |
| --- | ------ | -------- | ------------------------------------------- |
| 1   | String | ✓        | The message text                            |
| 2   | Float  | ✓        | Flash duration in seconds (e.g. `0.5`, `2.0`) |
| 3   | String |          | Flash/text color. Default: white flash      |

**Behavior:**
1. Set message text immediately.
2. Set the **window background** to the flash color (full opacity).
3. After `duration_s`, animate background back to default (black/dark).
4. Message text remains visible after flash.

**Examples:**
```
/sndwrks/hud/message/flash "GO GO GO" 1.0 "red"
/sndwrks/hud/message/flash "WARNING" 0.5
```

### 3.4 Utility commands

```
/sndwrks/hud/clear                       — clear all text
/sndwrks/hud/color         <string>      — change text color persistently (until next explicit color)
/sndwrks/hud/background    <string>      — change background color
/sndwrks/hud/fontsize      <int>         — override auto font size (0 = return to auto-fit)
/sndwrks/hud/countdown     <float: secs> — show a mm:ss.s countdown timer (like QDisplay's timeRemaining)
```

---

## 4. Windows

### 4.1 HUD Window

| Property            | Detail                                                    |
| ------------------- | --------------------------------------------------------- |
| Title bar           | **None** — frameless/borderless window                    |
| Always on top       | Yes (configurable)                                        |
| Background          | Solid black (default, configurable)                       |
| Text                | Centered, auto-sized to fill the window                   |
| Font                | System monospace or a bold sans-serif (configurable)      |
| Resizable           | Via settings (width × height), or manual drag if desired  |
| Position            | Remembers last position on screen                         |
| Multi-monitor       | User drags to whichever screen; position persists         |
| Draggable           | Click-and-drag anywhere to reposition (since no title bar)|

**Text auto-sizing:** The font size should scale so the longest line fits the window width with some padding, up to a maximum. For multi-line, size is determined by the longest line and total line count.

### 4.2 Settings Window

A standard windowed form with:

| Setting               | Type        | Default     | Notes                                              |
| --------------------- | ----------- | ----------- | -------------------------------------------------- |
| OSC UDP Port          | Number      | `52100`     | 1024–65535. Restart listener on change.             |
| OSC TCP Port          | Number      | `52101`     | 1024–65535. Restart listener on change.             |
| HUD Width             | Number (px) | `800`       | Applied to HUD window.                             |
| HUD Height            | Number (px) | `300`       | Applied to HUD window.                             |
| Default Text Color    | Color       | `#FFFFFF`   | Used when no color arg is provided.                 |
| Default Background    | Color       | `#000000`   | HUD window background.                             |
| Always on Top         | Toggle      | On          |                                                     |
| Font Family           | Dropdown    | System bold | Optional: let user pick from a few good choices.    |
| Auto-fit Font Size    | Toggle      | On          | Scales text to fill window. Off = use fixed size.   |
| Fixed Font Size       | Number      | `72`        | Only active when auto-fit is off.                   |

Changes take effect immediately (live preview). Settings persist to disk via `tauri-plugin-store` or a JSON config file at the app data directory.

---

## 5. Project Structure

```
sndwrks-hud/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs              # Tauri entry, window setup
│   │   ├── osc.rs               # OSC listener (async UDP + TCP tasks)
│   │   ├── config.rs            # Settings load/save
│   │   └── commands.rs          # Tauri IPC commands (get/set settings, etc.)
│   └── icons/
├── src/                          # Frontend (webview)
│   ├── hud.html                 # HUD window page
│   ├── hud.css
│   ├── hud.js
│   ├── settings.html            # Settings window page
│   ├── settings.css
│   └── settings.js
├── package.json                  # (if using npm for dev tooling)
└── README.md
```

---

## 6. Rust Backend Detail

### 6.1 OSC Listener (`osc.rs`)

```rust
// Pseudocode — UDP listener
async fn start_udp_listener(port: u16, app_handle: AppHandle) {
    let socket = UdpSocket::bind(("0.0.0.0", port)).await?;
    let mut buf = [0u8; 4096];

    loop {
        let (len, _addr) = socket.recv_from(&mut buf).await?;
        let (_, packet) = rosc::decoder::decode_udp(&buf[..len])?;

        match packet {
            OscPacket::Message(msg) => handle_message(msg, &app_handle),
            OscPacket::Bundle(bundle) => {
                for p in bundle.content {
                    // recurse
                }
            }
        }
    }
}

// TCP listener — SLIP-framed OSC
async fn start_tcp_listener(port: u16, app_handle: AppHandle) {
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    loop {
        let (stream, _addr) = listener.accept().await?;
        let app = app_handle.clone();
        tokio::spawn(async move {
            handle_tcp_connection(stream, app).await;
        });
    }
}

fn handle_message(msg: OscMessage, app: &AppHandle) {
    match msg.addr.as_str() {
        "/sndwrks/hud/message/single"   => { /* extract args, emit event */ }
        "/sndwrks/hud/message/lines"    => { /* ... */ }
        "/sndwrks/hud/message/flash"    => { /* ... */ }
        "/sndwrks/hud/clear"            => { /* ... */ }
        "/sndwrks/hud/color"            => { /* ... */ }
        "/sndwrks/hud/background"       => { /* ... */ }
        "/sndwrks/hud/fontsize"         => { /* ... */ }
        "/sndwrks/hud/countdown"        => { /* ... */ }
        _ => { /* unknown, ignore or log */ }
    }
}
```

Events are emitted to the HUD window's webview via `app.emit("hud-update", payload)`.

### 6.2 Event payloads (Rust → Frontend)

```rust
#[derive(Serialize, Clone)]
#[serde(tag = "type")]
enum HudEvent {
    Single   { message: String, color: Option<String> },
    Lines    { messages: Vec<String>, color: Option<String> },
    Flash    { message: String, duration_s: f32, color: Option<String> },
    Clear,
    SetColor { color: String },
    SetBackground { color: String },
    SetFontSize { size: u32 },  // 0 = auto
    Countdown { seconds: f32 },
}
```

### 6.3 IPC Commands (`commands.rs`)

```rust
#[tauri::command]
fn get_settings(state: State<AppConfig>) -> Settings { ... }

#[tauri::command]
fn save_settings(state: State<AppConfig>, settings: Settings) -> Result<(), String> { ... }

#[tauri::command]
fn restart_osc_listeners(state: State<OscState>, udp_port: u16, tcp_port: u16) -> Result<(), String> { ... }
```

---

## 7. Frontend Detail

### 7.1 HUD Window (`hud.html`)

Minimal DOM:

```html
<body id="hud-body">
  <div id="message-container">
    <div id="message-text"></div>
  </div>
  <div id="countdown-text"></div>
</body>
```

**JavaScript logic:**
- Listen for `hud-update` Tauri events.
- On `Single`: set `#message-text` innerHTML, apply color.
- On `Lines`: join messages with `<br>`, apply color.
- On `Flash`: set text, animate `#hud-body` background color, then fade/snap back after duration.
- On `Clear`: empty the text.
- Auto-size: use a resize observer + binary search on font size to find the largest size that fits without overflow.

**CSS:**
```css
body {
  margin: 0;
  background: var(--bg-color, #000000);
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
  /* Allow dragging the frameless window */
}

#message-text {
  color: var(--text-color, #ffffff);
  font-family: var(--font-family, 'system-ui');
  font-weight: 700;
  text-align: center;
  padding: 1rem;
  transition: color 0.15s ease;
  word-break: break-word;
}

/* Flash animation */
.flash-active {
  animation: flash-pulse var(--flash-duration, 1s) ease-out;
}
```

### 7.2 Settings Window (`settings.html`)

A simple form. On save, calls the `save_settings` Tauri command and triggers an OSC listener restart if either port changed.

---

## 8. Cargo Dependencies

```toml
[dependencies]
tauri = { version = "2", features = ["protocol-asset"] }
tauri-plugin-store = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
rosc = "0.10"
```

---

## 9. Build & Distribution

| Platform | Target                | Notes                                           |
| -------- | --------------------- | ----------------------------------------------- |
| macOS    | `.dmg` / `.app`       | Universal binary (aarch64 + x86_64)             |
| Windows  | `.msi` / `.exe`       | NSIS or WiX installer                           |
| Linux    | `.AppImage` / `.deb`  | AppImage for portability                        |

Tauri v2 handles all of this with `tauri build`.

---

## 10. Testing the OSC API

Use any OSC tool to test. Quick examples with `oscsend` (from `liblo`):

```bash
# single message
oscsend localhost 52100 /sndwrks/hud/message/single ss "Hello World" "green"

# multiple lines
oscsend localhost 52100 /sndwrks/hud/message/lines sss "Line A" "Line B" "cyan"

# flash
oscsend localhost 52100 /sndwrks/hud/message/flash sfs "STANDBY" 2.0 "red"

# clear
oscsend localhost 52100 /sndwrks/hud/clear

# countdown
oscsend localhost 52100 /sndwrks/hud/countdown f 120.0
```

Or from Python with `python-osc`:

```python
from pythonosc.udp_client import SimpleUDPClient
client = SimpleUDPClient("127.0.0.1", 52100)
client.send_message("/sndwrks/hud/message/single", ["CUE 42 GO", "yellow"])
```

---

## 11. Implementation Milestones

| #  | Milestone                        | Scope                                                        |
| -- | -------------------------------- | ------------------------------------------------------------ |
| 1  | **Scaffold**                     | `cargo tauri init`, two-window setup (HUD + settings), frameless HUD window |
| 2  | **OSC UDP listener**             | UDP socket with `rosc`, parse `/single`, emit Tauri event     |
| 3  | **HUD rendering**                | Frontend listens for events, shows text, auto-sizes font      |
| 4  | **Settings persistence**         | Load/save ports, dimensions, colors to JSON config            |
| 5  | **Full OSC API**                 | `/lines`, `/flash`, `/clear`, `/color`, `/background`, `/fontsize`, `/countdown` |
| 6  | **Flash animation**              | Background color pulse with configurable duration             |
| 7  | **Countdown timer**              | Frontend-side `requestAnimationFrame` countdown display       |
| 8  | **OSC TCP listener**             | TCP socket with SLIP framing, same message handler            |
| 9  | **Polish**                       | Drag-to-move, position memory, multi-monitor, tray icon       |
| 10 | **Package & release**            | CI builds for macOS/Windows/Linux                             |

---

## 12. Future Ideas (out of scope for v1)

- **OSC feedback/query:** `/sndwrks/hud/ping` responds with a reply so controllers can discover the app.
- **Multiple HUD instances:** run N HUD windows, each on a different namespace (`/sndwrks/hud/1/...`, `/sndwrks/hud/2/...`).
- **Image display:** show an image file path via OSC.
- **Web remote:** a small built-in HTTP server that serves a preview and a control page.
- **Cue list / message queue:** stack messages and cycle through them.