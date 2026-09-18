# Hyprvyl — Complete Project Specification, Architecture & Changelog

**Project Name**: Hyprvyl  
**Language**: Rust (2021 edition)  
**Binary Location**: `/home/dev/.local/bin/hyprvyl` (Source: `/home/dev/Users/projects/hyprvyl`)  
**Config Path**: `~/.config/hyprvyl/config.toml`  
**Socket Path**: `$XDG_RUNTIME_DIR/hyprvyl.sock` (Fallback: `/run/user/<UID>/hyprvyl.sock`)  

---

## 1. Project Overview & Vision

**Hyprvyl** is a blazing-fast, lightweight, Wayland-native radial application launcher and workspace navigation overlay daemon designed specifically for **Hyprland**. Inspired by the sleek Fluent dark aesthetic of **Rovyl**, Hyprvyl provides:
- Instantaneous mouse and keyboard activation via low-level `evdev` listening and Unix Domain Socket IPC.
- Zero startup lag by running as a resident background daemon (`gtk4::Application`).
- Unmanaged, transparent Wayland layer-shell surface (`wlr-layer-shell` via `gtk4-layer-shell`) anchored to the active monitor.
- Direct Cairo 2D vector graphics rendering with smooth hover scaling, drop shadows, dynamic tooltip pills, and a crisp helm wheel logo.
- Unified single-process architecture housing both the radial layer-shell overlay and the dedicated Fluent Settings GUI window.

---

## 2. Complete Architecture & Source Modules

```
hyprvyl/
├── Cargo.toml               # Rust package definitions, dependencies & metadata
├── assets/
│   ├── icon.png             # Hyprvyl icon
│   └── logo.png             # Ship's helm steering wheel emblem
├── src/
│   ├── main.rs              # Entry point, CLI dispatcher, daemon loop, test suites
│   ├── overlay.rs           # Layer-shell overlay window, controllers, open_settings logic
│   ├── renderer.rs          # Cairo 2D vector rendering engine, helm logo, polar layouts
│   ├── settings.rs          # Dedicated Fluent dark Settings GUI, icon picker, CRUD
│   ├── config.rs            # TOML schema, serde, ItemKind (App/Url/Folder/File/Binary)
│   ├── apps.rs              # XDG .desktop parsing, launch routing, terminal & file manager overrides
│   ├── icons.rs             # Bundled 189 SVG icon registry, category filtering, pixbuf loader
│   ├── favicons.rs          # Non-blocking async URL favicon fetcher, HTML scraper & PNG disk cache
│   ├── ipc.rs               # Unix domain socket server & client IPC protocol
│   ├── hyprland.rs          # hyprctl cursor & monitor queries, monitor geometry matcher
│   └── trigger.rs           # evdev device input listener for hold/release and key triggers
└── PROJECT_STATE.md         # This complete project record
```

### Module Breakdown

#### `src/main.rs`
- **Application IDs**: `org.hyprvyl.launcher` (Daemon) & `org.hyprvyl.settings` (Standalone fallback).
- **CLI Commands**:
  - `--settings` / `-S`: Sends `SETTINGS` command to running daemon via IPC, or launches standalone if daemon is offline.
  - `--toggle` / `-t`: Toggles overlay visibility via IPC.
  - `--show` / `-s`: Shows overlay at active cursor location.
  - `--hide`: Hides overlay and enables full click-through pass-through.
  - `--release`: Simulates hold-trigger release (selects hovered item or opens settings).
  - `--reload` / `-r`: Hot-reloads configuration and rescans applications.
  - `--apps-count`: Queries loaded application count.
  - `--status`: Queries visibility status (`VISIBLE=true` / `VISIBLE=false`).
  - `--list-apps`: Prints table of all discovered applications, icon paths, and launch commands.
  - `--verify-apps`: Runs 100% automated readiness verification across all installed desktop apps.
  - `--launch <ID/NAME>`: Launches specific app directly with validation.
  - `--bench-apps [N]`: Benchmarks application discovery performance across N runs.
  - `--cycle-test [N]`: Runs automated open/close IPC stress cycle test.
- **Daemon Lifecycle**: Boots GTK application, initializes `OverlayWindow`, discovers applications, attaches `inotify` live watcher, starts IPC socket listener, and spawns `evdev` trigger thread.

#### `src/overlay.rs`
- **Layer-Shell Surface**: Layer `Overlay`, exclusive zone `-1`, anchors to all 4 edges, transparent CSS styling.
- **Unified Settings Reference**: Holds `app: Application` and `settings_win: Rc<RefCell<Option<Rc<SettingsWindow>>>>`.
- **`open_settings(&self)`**: Hides radial overlay and calls `present()` on `SettingsWindow`, reusing instance across opens.
- **Navigation Hierarchy**:
  - Root level (`active_workspace_id = None`): Displays list of configured workspaces.
  - Sub-level (`active_workspace_id = Some(id)`): Displays items inside selected workspace.
- **Event Controllers**:
  - `EventControllerMotion`: Live cursor tracking, dynamic button highlight, breadcrumb button hover.
  - `GestureClick`: Center hub click -> opens settings; item click -> launches app/url/folder; click outside -> dismisses overlay.
  - `EventControllerScroll`: Smooth page cycling for workspaces with >8 items.
  - `EventControllerKey`: Live type-to-filter search, Backspace, Escape (cancels search or closes), Enter (launches selected), Number keys 1–9 for workspace switching.

#### `src/renderer.rs`
- **Cairo 2D Graphics Engine**: Pure vector mathematics without raster dependencies.
- **`draw_helm_logo`**: Renders the crisp monochrome ship's helm emblem (center hub, 3 concentric rings, 8 spokes with 4 cardinal terminal pins and 4 diagonals). Matches `assets/logo.png`.
- **Always Rendered**: The helm logo is permanently rendered in the center hub across all levels (root and workspace sub-levels).
- **Polar Coordinate Geometry**: Dynamic arc positioning, adjustable arc radius (`140px`), span degrees (`140°`), button size (`56px`), corner radius (`14px`).
- **Interactive States**:
  - Normal buttons: `#1c1c1e` dark translucent background with drop shadow.
  - Hovered buttons: Scaled by 8%, filled with bright white `#ffffff`, dark icon inversion, and adjacent tooltip pill label (`#ffffff` pill with dark `#111111` bold text).
  - Bottom Segmented Pill: `[ Main | Back ]` or `[ Hyprvyl | Center ]`.

#### `src/settings.rs`
- **Design Aesthetic**: Fluent dark theme (`#111113` background, `#0d0d0f` sidebar, `#161619` input surfaces, `#27272a` borders).
- **HeaderBar**: Navigation controls, window controls, and animated auto-dismissing "All changes saved" status badge.
- **Categories**:
  1. **General**: Autostart toggle, Close on launch toggle, and **"Reload Hyprvyl"** instant action button.
  2. **Activation**: Trigger mode (Hold vs Click), Hold duration slider (`100ms - 1000ms`), Trigger button **DropDown selector** (Mouse Forward/Back/Middle/Left/Right, Space, CapsLock, Super, Alt, F1–F12, etc.).
  3. **Appearance**: Radial layout preview, Button size (`40-80px`), Corner radius (`0-28px`), Arc radius (`100-240px`), Arc span (`90-360°`), Items per page (`4-12`), Icon size (`24-64px`), Theme.
  4. **Workspaces**: Full CRUD management of Workspaces (Name, Icon, Order) and nested Items (App, URL, Folder), with move up/down reordering and type badges.
  5. **Advanced**: Terminal emulator override (`kitty`, `foot`, `alacritty`), live watcher toggle, debug logging toggle.
- **Window Lifecycle**: Uses `connect_close_request` to hide rather than destroy, providing zero-latency reopening.

#### `src/config.rs`
- **Data Model**: Structured TOML configuration with automatic schema migration and defaults.
- **Default Location**: Centered on active monitor.
- **Autostart Engine**:
  - `sync_hyprland_autostart()`: Installs binary to `/home/dev/.local/bin/hyprvyl` (`0755`), writes/updates `exec-once = /home/dev/.local/bin/hyprvyl` in `~/.config/hypr/hyprland.conf`.
  - `sync_xdg_autostart()`: Writes `~/.config/autostart/hyprvyl.desktop` with `Exec=/home/dev/.local/bin/hyprvyl` and `Terminal=false`.

#### `src/apps.rs`
- **XDG Discovery**: Scans `/usr/share/applications`, `/usr/local/share/applications`, `~/.local/share/applications`, and Flatpak export directories.
- **Icon Resolution**: Resolves themed icons, direct `.png`/`.svg` file paths, strips `.desktop` from targets, and resolves dedicated icons for common URLs (WhatsApp, GitHub, YouTube, Discord, Claude, ChatGPT, etc.) and folders.
- **Terminal Detection**: Auto-detects installed terminal emulators (`kitty`, `foot`, `alacritty`, `konsole`, `wezterm`, etc.) and wraps CLI applications accordingly.
- **Inotify Watcher**: Live file watcher on desktop directories triggering seamless cache invalidation without daemon restarts.

#### `src/ipc.rs`
- **Socket**: Unix domain socket at `$XDG_RUNTIME_DIR/hyprvyl.sock`.
- **Timeouts & Buffering**: Configured with 2-second read/write timeouts and buffered line reading to prevent socket lockups.
- **Commands**: `SHOW`, `HIDE`, `TOGGLE`, `RELEASE`, `SETTINGS`, `STATUS`, `APPS`, `RELOAD`, `PING`.

#### `src/hyprland.rs`
- **Active Cursor**: Queries `hyprctl cursorpos` to determine cursor `(X, Y)`.
- **Monitor Detection**: Queries `hyprctl monitors -j` to match cursor coordinates against active monitor viewport and scale factor.

#### `src/trigger.rs`
- **Evdev Listener**: Scans `/dev/input/event*` for configured mouse button (`BTN_EXTRA`, `BTN_SIDE`, etc.) or keyboard key.
- **Hold-Release Mechanism**: Opens overlay on keydown; selects hovered app or opens settings on keyup if duration exceeded.

---

## 3. Detailed Chronological Changelog

### Phase 1: Diagnostics & Quality Hardening
1. **Fixed Signal Handling (`SIGCHLD`)**:
   - Removed `libc::signal(libc::SIGCHLD, libc::SIG_IGN)` in `main.rs` which was causing `std::process::Command` calls to `hyprctl` to fail with `ECHILD` (No child processes).
2. **IPC Socket Robustness**:
   - Added `BufReader` and read/write timeouts to IPC server and client streams to eliminate hanging connections.
3. **Clippy Linter Resolution**:
   - Resolved all 48 compiler and clippy warnings (`cargo clippy -- -D warnings` now completely clean).

### Phase 2: User Requested Enhancements
4. **Default Radial Location Centered**:
   - Removed obsolete spawn position configurations and set `SpawnPosition::Center` as the permanent default.
   - Removed the "Spawn position" UI row from Settings.
5. **Reload Button in Settings**:
   - Added an interactive "Reload Hyprvyl" row and button to the General settings page in `src/settings.rs` that triggers IPC `RELOAD` and refreshes app cache live.
6. **Trigger Button DropDown Selector**:
   - Replaced free-text input with a comprehensive GTK `DropDown` widget containing standard mouse buttons and keyboard shortcut keys.
7. **Autostart Fix & Root Cause Resolution**:
   - Identified that `~/.config/autostart/hyprvyl.desktop` had `Terminal=falsep` syntax error.
   - Identified that display managers (SDDM/GDM) start Hyprland without `~/.local/bin` in environment `$PATH`.
   - Updated `sync_hyprland_autostart()` and `sync_xdg_autostart()` to write the absolute target path `/home/dev/.local/bin/hyprvyl`.
   - Cleaned `~/.config/hypr/hyprland.conf` to contain `exec-once = /home/dev/.local/bin/hyprvyl`.
8. **App / Helm Logo Persistence Across Workspaces**:
   - Fixed center hub rendering in `src/renderer.rs` so `draw_helm_logo` is always rendered across all workspace navigation levels instead of disappearing or being replaced.
9. **Combined Single-Process Daemon & Settings**:
   - Unified `OverlayWindow` and `SettingsWindow` under the same `gtk4::Application`.
   - Added `SETTINGS` IPC command in `src/ipc.rs`.
   - Updated `hyprvyl --settings` to send IPC command to running daemon rather than requiring a separate terminal.
   - Added `connect_close_request` to Settings window to hide rather than destroy for zero-latency reopening.
10. **Center Hub Click/Release Opens Settings**:
    - Wired `click_gesture` in `src/overlay.rs` on center hub to call `overlay.open_settings()`.
    - Wired `handle_trigger_release` in `src/overlay.rs` on center hub to call `self.open_settings()`.

---

## 4. Configuration Reference (`~/.config/hyprvyl/config.toml`)

```toml
[general]
start_with_hyprland = true    # Write exec-once to hyprland.conf
autostart = true              # Write ~/.config/autostart/hyprvyl.desktop
close_on_launch = true        # Automatically hide overlay when an app/URL is launched

[trigger]
mode = "hold"                 # "hold" | "click"
hold_duration_ms = 250        # Milliseconds required for hold activation
button = "BTN_EXTRA"          # Evdev button code (e.g. BTN_EXTRA, BTN_SIDE, KEY_GRAVE)
device = ""                   # Optional specific input device path override

[appearance]
button_size = 56.0            # Button icon size (px)
corner_radius = 14.0          # Rounded corner radius (px)
arc_radius = 140.0            # Distance from center hub to buttons (px)
arc_span_degrees = 140.0      # Radial sweep angle
items_per_page = 8            # Max items per page before pagination
icon_size = 36                # Target icon pixel size
opacity = 0.90                # Canvas opacity
theme = "dark"

[workspaces]
spawn_at_cursor = true        # Center wheel on the active monitor containing the cursor
follow_workspace = true       # Follow active workspace changes
workspace_switching = "picker"# "picker" (visual wheel) | "keys" (number keys 1..9)

[advanced]
terminal_override = ""        # Custom terminal (e.g. "kitty") or empty for auto-detect
debug_logging = false         # Verbose terminal logs
live_watcher = true           # Inotify watcher for .desktop file updates

[[workspace]]
id = "main"
name = "Main"
icon = "folder"
order = 0

  [[workspace.item]]
  name = "Chrome"
  kind = "app"
  target = "google-chrome.desktop"
  order = 0

  [[workspace.item]]
  name = "WhatsApp"
  kind = "url"
  target = "https://web.whatsapp.com"
  order = 1

  [[workspace.item]]
  name = "Notepad"
  kind = "app"
  target = "org.gnome.TextEditor.desktop"
  order = 2

  [[workspace.item]]
  name = "Calculator"
  kind = "app"
  target = "org.gnome.Calculator.desktop"
  order = 3

[[workspace]]
id = "work"
name = "Work"
icon = "folder"
order = 1

  [[workspace.item]]
  name = "Firefox"
  kind = "app"
  target = "firefox.desktop"
  order = 0

  [[workspace.item]]
  name = "Anthropic"
  kind = "url"
  target = "https://anthropic.com"
  order = 1

  [[workspace.item]]
  name = "Projects"
  kind = "folder"
  target = "/home/dev/Projects"
  order = 2
```

---

## 5. IPC Protocol Reference

Socket file: `$XDG_RUNTIME_DIR/hyprvyl.sock`

| Command | Description | Response Example |
|---|---|---|
| `SHOW` | Displays radial overlay at cursor position | `OK: SHOWN` |
| `HIDE` | Hides radial overlay and releases grabs | `OK: HIDDEN` |
| `TOGGLE` | Toggles overlay visibility | `OK: TOGGLED` |
| `RELEASE` | Simulates trigger release on hovered element | `OK: RELEASED` |
| `SETTINGS` | Opens and focuses Settings GUI window | `OK: SETTINGS OPENED` |
| `STATUS` | Queries whether overlay is currently visible | `OK: VISIBLE=true` / `OK: VISIBLE=false` |
| `APPS` | Returns count of loaded desktop apps | `OK: APPS=74` |
| `RELOAD` | Reloads `config.toml` & rescans `.desktop` files | `OK: RELOADED count=74` |
| `PING` | Health check | `PONG` |

---

## 6. Verification & Test Commands

- **Run all unit tests**: `cargo test` (13/13 passing).
- **Run linter**: `cargo clippy -- -D warnings` (0 warnings).
- **Verify all desktop applications**: `cargo run -- --verify-apps` (74/74 passing).
- **Benchmark app discovery**: `cargo run -- --bench-apps 50` (~2.8ms average scan).
- **Run automated IPC cycle stress test**: `cargo run -- --cycle-test 20`.
- **Install release binary**: `cargo build --release && cp target/release/hyprvyl ~/.local/bin/hyprvyl && chmod 755 ~/.local/bin/hyprvyl`.
