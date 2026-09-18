# Hyprvyl

Hyprvyl is a fast, lightweight, and modern radial application launcher designed specifically for the **Hyprland** Wayland compositor.

---

## Architecture

### Stage 1: Core Wayland Layer-Shell Overlay
- **Stack**: Rust + GTK4 + `gtk4-layer-shell` + Cairo vector rendering.
- **Protocol**: Pure `wlr-layer-shell-unstable-v1` at `Layer::Overlay` (not an `xdg-toplevel` window).
- **Transparency**: True per-pixel alpha transparency via CSS and transparent Cairo surface. Surrounding desktop windows and wallpaper show through with zero rendering artifacts.
- **Input & Pass-Through**:
  - **Hidden / Idle**: Surface unmapped, keyboard mode `None` → 100% click-through to underlying applications.
  - **Shown**: Surface presented, keyboard mode `Exclusive` → captures clicks inside radial sectors, Escape/q key dismissal, and dismissal on click-outside.
- **Multi-Monitor & Dynamic Cursor Placement**:
  - Automatically queries active cursor coordinates via `hyprctl cursorpos`.
  - Determines the active monitor and attaches the layer surface to that specific display (`set_monitor`).
  - Anchors radial launcher center directly under the user's cursor.
- **Zero Window Rules Required**: Because it uses Wayland Layer Shell natively, it is never tiled or managed as a standard window and requires no `windowrulev2` overrides.

### Stage 2: Global Input & Trigger System
- **Dual-Engine Architecture**:
  1. **Built-in Global Evdev Listener (`/dev/input/event*`)**:
     - Auto-detects mouse and keyboard devices matching the target button.
     - **`hold` mode**: High-precision press-duration timing (default 200ms). Short clicks (<200ms) pass through untouched to underlying apps; holding the button opens Hyprvyl at the cursor.
     - **`click` mode**: Triggers overlay immediately on press.
     - **Toggle behavior**: Pressing trigger while overlay is open dismisses it.
     - **Graceful Permission Handling**: If `/dev/input` lacks permissions, logs a clear, actionable guide and keeps the IPC engine running without crashing.
  2. **Zero-Permission Native IPC Trigger (`hyprvyl --toggle`)**:
     - Fast Unix Domain Socket IPC (`$XDG_RUNTIME_DIR/hyprvyl.sock`).
     - Allows direct triggering from `hyprland.conf` with zero root or udev configuration.

---

## Configuration (`~/.config/hyprvyl/config.toml`)

Hyprvyl generates a default configuration file automatically upon first launch:

```toml
[trigger]
# Trigger mode: "hold" or "click"
# - "hold": Waits hold_duration_ms before opening. Quick clicks pass through to underlying windows.
# - "click": Opens immediately on button/key press.
mode = "hold"

# Duration in milliseconds to hold before opening in "hold" mode (default: 200)
hold_duration_ms = 200

# Button or key name to trigger the overlay.
# Common mouse buttons:
#   BTN_SIDE    (Mouse Button 4 / Back thumb button)
#   BTN_EXTRA   (Mouse Button 5 / Forward thumb button)
#   BTN_MIDDLE  (Middle mouse wheel click)
#   BTN_RIGHT   (Right mouse button)
# Common keys:
#   KEY_GRAVE   (Tilde / Grave key ` )
#   KEY_CAPSLOCK
#   KEY_LEFTMETA (Super key)
button = "BTN_EXTRA"

# Optional explicit device path (e.g. "/dev/input/event5").
# If left empty (""), Hyprvyl will automatically scan and detect input devices that support the button.
device = ""
```

---

## Building from Source

### Dependencies (Arch Linux)
```bash
sudo pacman -S gtk4 gtk4-layer-shell rust cargo
```

### Build
```bash
cargo build --release
```

---

## Permissions Setup (For Evdev Hold Mode)

To enable background mouse/keyboard hold-detection via `/dev/input`:
```bash
sudo usermod -aG input $USER
# Log out and log back in (or reboot) for the group change to take effect.
```

---

## Usage & IPC Commands

Start the background daemon:
```bash
./target/release/hyprvyl
```

Trigger or control the overlay via CLI or Hyprland keybinds:
```bash
# Toggle overlay visibility
./target/release/hyprvyl --toggle

# Show overlay at active cursor coordinates
./target/release/hyprvyl --show

# Hide overlay (enables click-through)
./target/release/hyprvyl --hide

# Query status
./target/release/hyprvyl --status

# Run automated 25-cycle stability test
./target/release/hyprvyl --cycle-test 25
```

### Hyprland Keybind Integration Example
Add this to your `~/.config/hypr/hyprland.conf`:
```conf
# Launch / toggle Hyprvyl radial menu under cursor
bind = SUPER, mouse:275, exec, hyprvyl --toggle
# or keyboard shortcut:
bind = SUPER, SPACE, exec, hyprvyl --toggle
```
