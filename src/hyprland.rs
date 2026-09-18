//! Hyprland compositor integration.
//!
//! Provides utilities for querying cursor position and monitor configurations
//! from Hyprland via `hyprctl`. This allows Hyprvyl to dynamically spawn the
//! radial menu directly centered under the user's cursor on the active display.

use gtk4::gdk;
use gtk4::prelude::*;
use serde::Deserialize;
use std::process::Command;

/// Represents a monitor configuration reported by Hyprland or GDK fallback.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct HyprMonitor {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub width: i32,
    pub height: i32,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub focused: bool,
    #[serde(rename = "activeWorkspace")]
    pub active_workspace: Option<WorkspaceInfo>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct WorkspaceInfo {
    pub id: i64,
    pub name: String,
}

/// Represents the global (x, y) coordinates of the cursor in Wayland compositor space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32,
}

/// Queries Hyprland for the current cursor position using `hyprctl cursorpos -j` or plain text.
pub fn get_cursor_position() -> Option<CursorPosition> {
    // 1. Try hyprctl cursorpos -j (JSON format: {"x": 1234, "y": 567})
    if let Ok(output) = Command::new("hyprctl").args(["cursorpos", "-j"]).output()
        && output.status.success()
            && let Ok(pos) = serde_json::from_slice::<CursorPosition>(&output.stdout) {
                return Some(pos);
            }

    // 2. Try hyprctl cursorpos (comma-delimited text format: "1234, 567")
    if let Ok(output) = Command::new("hyprctl").arg("cursorpos").output()
        && output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = text.trim().split(',').collect();
            if parts.len() == 2
                && let (Ok(x), Ok(y)) = (parts[0].trim().parse::<i32>(), parts[1].trim().parse::<i32>()) {
                    return Some(CursorPosition { x, y });
                }
        }

    None
}

/// Queries Hyprland for all active monitors via `hyprctl monitors -j`, falling back to GDK monitors.
pub fn get_monitors() -> Vec<HyprMonitor> {
    if let Ok(output) = Command::new("hyprctl").args(["monitors", "-j"]).output()
        && output.status.success()
            && let Ok(mons) = serde_json::from_slice::<Vec<HyprMonitor>>(&output.stdout)
                && !mons.is_empty() {
                    return mons;
                }

    get_gdk_monitors_fallback()
}

/// Fallback: queries GTK4 GDK display monitors directly.
fn get_gdk_monitors_fallback() -> Vec<HyprMonitor> {
    let mut result = Vec::new();
    if let Some(display) = gdk::Display::default() {
        let mons = display.monitors();
        for i in 0..mons.n_items() {
            if let Some(item) = mons.item(i)
                && let Ok(mon) = item.downcast::<gdk::Monitor>() {
                    let geom = mon.geometry();
                    let scale = mon.scale_factor() as f64;
                    let name = mon.connector().map(|s| s.to_string()).unwrap_or_else(|| format!("MON-{}", i));
                    result.push(HyprMonitor {
                        id: i as i64,
                        name,
                        description: mon.model().map(|s| s.to_string()),
                        width: geom.width(),
                        height: geom.height(),
                        x: geom.x(),
                        y: geom.y(),
                        scale: if scale > 0.0 { scale } else { 1.0 },
                        focused: i == 0,
                        active_workspace: None,
                    });
                }
        }
    }

    if result.is_empty() {
        result.push(HyprMonitor {
            id: 0,
            name: "eDP-1".to_string(),
            description: None,
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            scale: 1.0,
            focused: true,
            active_workspace: None,
        });
    }

    result
}

/// Determines which monitor currently contains the specified cursor coordinate.
///
/// If cursor position is outside all monitor bounds (or undetectable), returns the focused monitor
/// or the first available monitor.
pub fn find_monitor_for_cursor(cursor: CursorPosition, monitors: &[HyprMonitor]) -> Option<&HyprMonitor> {
    // Test if the cursor falls within monitor bounding rectangles
    for monitor in monitors {
        let w = (monitor.width as f64 / monitor.scale).round() as i32;
        let h = (monitor.height as f64 / monitor.scale).round() as i32;
        if cursor.x >= monitor.x
            && cursor.x < monitor.x + w
            && cursor.y >= monitor.y
            && cursor.y < monitor.y + h
        {
            return Some(monitor);
        }
    }

    // Fallback: look for focused monitor
    monitors
        .iter()
        .find(|m| m.focused)
        .or_else(|| monitors.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyprctl_monitor_parsing() {
        let monitors = get_monitors();
        println!("Monitors detected: {:?}", monitors);
        let cur = get_cursor_position();
        println!("Cursor pos: {:?}", cur);
        assert!(!monitors.is_empty(), "get_monitors returned empty vec");
    }
}

