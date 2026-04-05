# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

sndwrks hud — a cross-platform OSC-driven heads-up display for live events, built with Tauri 2 (Rust backend + vanilla HTML/CSS/JS frontend). Receives Open Sound Control messages over UDP and displays text in a floating window.

## Commands

```bash
npm install              # Install Tauri CLI and API dependencies
npm run tauri dev        # Dev mode with hot reload
npm run tauri build      # Production build (platform-specific bundles)
npm test                 # Run all Rust tests (cargo test in src-tauri/)
```

There are no frontend tests, linter, or TypeScript — the frontend is vanilla JS.

## Architecture

### Message Flow: UDP → Rust → Frontend

1. `osc.rs`: `start_udp_listener()` receives UDP packets on `0.0.0.0:{port}` via tokio
2. `rosc` decodes packets; `handle_packet()` recursively unpacks bundles
3. `parse_osc_message()` returns a `HudEvent` enum variant
4. `app.emit("hud-update", &event)` sends to frontend via Tauri event system
5. `hud.js`: `listen('hud-update', ...)` dispatches by `event.type` to update DOM

### OSC Listener Restart

Uses a `tokio::sync::watch` channel. When `restart_osc` command is called, it sends to `port_tx`, which breaks the `tokio::select!` in the listener loop, restarting with the new port. No manual thread killing needed.

### State

- **`AppConfig`** — `Mutex<Settings>` + config file path. Managed by Tauri's `.manage()`, accessed in commands via `State<AppConfig>`. Settings are cloned on access (not held locked).
- **`OscState`** — holds the `watch::Sender<u16>` for port change signaling.
- **Frontend** — minimal state in JS globals; settings loaded once on startup via IPC.

### Windows

Three persistent windows defined in `tauri.conf.json`: `hud`, `settings`, `help`. Settings and help are hidden on startup and toggled via menu/buttons — close is intercepted with `prevent_close()` + `hide()` to avoid recreating them.

### IPC Commands (commands.rs)

- `get_settings()` — returns cloned Settings
- `save_settings(settings)` — updates config, preserves `hud_x`/`hud_y` from old settings (position managed by window events, not the form)
- `restart_osc()` — reads current port from settings, sends to watch channel

### Color Convention

For `/message/single` and `/message/lines`, the parser checks if the **last** string arg is a valid color (named or hex). If so, it's extracted as styling; otherwise treated as content. Frontend maps named colors to pastel hex values via `PASTEL_COLORS` object.

Named colors: `white red green blue yellow teal orange purple pink` plus hex `#FFF` or `#ffffff`.

### Auto-fit Text

`hud.js` uses binary search (range 8–500px) to find the largest font size that fits the container. Triggered by a `ResizeObserver` on the HUD container, so it adapts when the window is resized.

### Config Persistence

Settings saved as JSON to `~/.config/sndwrks-hud/settings.json` (resolved via `dirs` crate). Window position (`hud_x`, `hud_y`) is updated on every move event.

## Adding a New OSC Command

1. Add variant to `HudEvent` enum in `osc.rs`
2. Add match arm in `parse_osc_message()`
3. Add tests
4. Handle new event type in `hud.js` listen callback

## Adding a New Setting

1. Add field with default to `Settings` struct in `config.rs`
2. Add form input in `settings.html`
3. Add field ref in `settings.js` `fields` object
4. Wire save/load logic in `settings.js`

## Git Conventions

- Branch naming: `main.<feature-name>`
- Never commit directly to `main`
- No Claude branding or attribution in commits, PRs, or code

## Key Dependencies

- **tauri 2** — app framework, IPC, window management
- **tokio** (net, rt, sync, macros) — async UDP, watch channel
- **rosc 0.10** — OSC packet parsing
- **serde/serde_json** — settings serialization
- **dirs 6** — platform config directory
- **@tauri-apps/api 2** — frontend JS bindings (global via `window.__TAURI__`, enabled by `withGlobalTauri: true`)
