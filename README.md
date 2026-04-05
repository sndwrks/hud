![sndwrks logo](images/sndwrks-square_200px-high.png)

# sndwrks hud

A cross-platform, OSC-driven display. Receives Open Sound Control messages over UDP, TCP and displays text in a window.

It's modeled off of the [Figure 53 Q Display](https://github.com/Figure53/QDisplay), but uses `OSC` instead of `Applescript` to display the messages. Very much written with a lot of help from AI, but this is tested and actively maintained, and we use it ourselves.

![sndwrks hud app](images/sndwrks-hud-screenshot.png)

## Install

Download the appropriate file for your system from the latest release. [Releases](https://github.com/sndwrks/hud/releases)

### macOS (Apple Silicon)

Download the `macos-arm` file. After downloading, right-click the app and select "Open". If that doesn't work, go to System Settings > Privacy & Security and click "Open Anyway" next to the blocked app message.

### macOS (Intel)

Download the `macos-intel` file. Follow the same steps as above to allow the app to run.

### Windows

Download the `windows` installer and run it. Windows may show a SmartScreen warning—click "More info" and then "Run anyway".

### Linux

Download the `linux` AppImage or deb file. For AppImage, make it executable with `chmod +x` and run it. For deb, install with your package manager.

> **Note**
> We primarily test on macOS ARM. Please open an issue if it doesn't work on your OS.

## OSC API

The app listens for OSC messages on a configurable UDP port (default `52100`). All addresses are prefixed with `/sndwrks/hud/`.

| Address | Args | Description |
| --- | --- | --- |
| `/sndwrks/hud/message/single` | `text` `[color]` | Display a single line of text |
| `/sndwrks/hud/message/lines` | `text...` `[color]` | Display multiple stacked lines |
| `/sndwrks/hud/message/flash` | `text` `duration` `[color]` | Display text with a flash animation |
| `/sndwrks/hud/clear` | | Clear the display |
| `/sndwrks/hud/color` | `color` | Set the default text color |
| `/sndwrks/hud/background` | `color` | Set the background color |
| `/sndwrks/hud/fontsize` | `size` | Set the font size |

Colors can be named (`white`, `red`, `green`, `blue`, `yellow`, `teal`, `orange`, `purple`, `pink`) or hex (`#ff0000`, `#f00`).

## Settings

Open Settings from the app menu. You can configure:

- **UDP/TCP ports** — which ports to listen on (default: 52100/52101)
- **Text color** — default color for displayed text
- **Always on top** — keep the HUD above other windows
- **Auto-fit font** — automatically size text to fill the window
- **Fixed font size** — set a specific font size when auto-fit is off

Settings are saved to `~/.config/sndwrks-hud/settings.json`.

## Development

- Rust toolchain (install via [rustup](https://rustup.rs))
- Node.js 18+
- npm 9+

**Linux additional deps:**

```bash
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev \
  libappindicator3-dev librsvg2-dev patchelf
```

### Install and Run

```bash
npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

### Testing

```bash
# Rust tests
npm test
```

### Architecture

```
src/                    # Frontend (vanilla HTML/CSS/JS)
  hud.html              # Main HUD window
  hud.js                # HUD logic (auto-sizing, events)
  settings.html         # Settings window
  settings.js           # Settings form logic
  help.html             # Help documentation
src-tauri/              # Rust backend
  src/
    main.rs             # App entry, window setup, menu
    osc.rs              # OSC UDP listener, message parsing
    config.rs           # Settings persistence (JSON)
    commands.rs         # Tauri IPC commands
```
