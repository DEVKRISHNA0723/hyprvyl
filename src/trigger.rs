//! Global Evdev input listener with configurable Hold/Click timing for Hyprvyl.



use crate::config::{Config, TriggerMode};
use crate::ipc::IpcCommand;
use async_channel::Sender;
use evdev::{Device, EventType, KeyCode};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Resolves a string name or number into an evdev KeyCode.
pub fn parse_key(name: &str) -> Option<KeyCode> {
    let name_upper = name.trim().to_uppercase();

    // Check if it's a numeric code directly
    if let Ok(code) = name_upper.parse::<u16>() {
        return Some(KeyCode::new(code));
    }

    match name_upper.as_str() {
        // Mouse Buttons
        "BTN_LEFT" | "MOUSE:272" => Some(KeyCode::BTN_LEFT),
        "BTN_RIGHT" | "MOUSE:273" => Some(KeyCode::BTN_RIGHT),
        "BTN_MIDDLE" | "MOUSE:274" => Some(KeyCode::BTN_MIDDLE),
        "BTN_SIDE" | "MOUSE:275" | "BTN_BACK" => Some(KeyCode::BTN_SIDE),
        "BTN_EXTRA" | "MOUSE:276" | "BTN_FORWARD" => Some(KeyCode::BTN_EXTRA),
        "BTN_FORWARD_ALT" | "MOUSE:277" => Some(KeyCode::BTN_FORWARD),
        "BTN_BACK_ALT" | "MOUSE:278" => Some(KeyCode::BTN_BACK),
        "BTN_TASK" => Some(KeyCode::BTN_TASK),

        // Keyboard Keys
        "KEY_GRAVE" | "GRAVE" | "TILDE" => Some(KeyCode::KEY_GRAVE),
        "KEY_SPACE" | "SPACE" => Some(KeyCode::KEY_SPACE),
        "KEY_TAB" | "TAB" => Some(KeyCode::KEY_TAB),
        "KEY_CAPSLOCK" | "CAPSLOCK" => Some(KeyCode::KEY_CAPSLOCK),
        "KEY_LEFTMETA" | "SUPER" | "KEY_SUPER" => Some(KeyCode::KEY_LEFTMETA),
        "KEY_RIGHTMETA" => Some(KeyCode::KEY_RIGHTMETA),
        "KEY_LEFTALT" | "ALT" => Some(KeyCode::KEY_LEFTALT),
        "KEY_RIGHTALT" => Some(KeyCode::KEY_RIGHTALT),
        "KEY_LEFTCTRL" | "CTRL" => Some(KeyCode::KEY_LEFTCTRL),
        "KEY_RIGHTCTRL" => Some(KeyCode::KEY_RIGHTCTRL),
        "KEY_LEFTSHIFT" | "SHIFT" => Some(KeyCode::KEY_LEFTSHIFT),
        "KEY_RIGHTSHIFT" => Some(KeyCode::KEY_RIGHTSHIFT),
        "KEY_ESC" | "ESCAPE" | "ESC" => Some(KeyCode::KEY_ESC),
        "KEY_F1" => Some(KeyCode::KEY_F1),
        "KEY_F2" => Some(KeyCode::KEY_F2),
        "KEY_F3" => Some(KeyCode::KEY_F3),
        "KEY_F4" => Some(KeyCode::KEY_F4),
        "KEY_F5" => Some(KeyCode::KEY_F5),
        "KEY_F6" => Some(KeyCode::KEY_F6),
        "KEY_F7" => Some(KeyCode::KEY_F7),
        "KEY_F8" => Some(KeyCode::KEY_F8),
        "KEY_F9" => Some(KeyCode::KEY_F9),
        "KEY_F10" => Some(KeyCode::KEY_F10),
        "KEY_F11" => Some(KeyCode::KEY_F11),
        "KEY_F12" => Some(KeyCode::KEY_F12),
        _ => {
            eprintln!("[Hyprvyl Trigger] Unknown key name '{}'.", name);
            None
        }
    }
}

/// Helper to print a detailed, helpful permission error message.
fn report_permission_error(path: Option<&Path>) {
    let dev_str = path
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "/dev/input/*".to_string());

    eprintln!(
        r#"
================================================================================
[Hyprvyl Trigger Warning] Permission Denied accessing input device: {}

To enable global mouse/key hold-trigger detection:
  1. Add your user to the 'input' group:
       sudo usermod -aG input $USER
  2. Log out and log back in (or reboot).

Alternatively, use Hyprland's native keybinds (no special permissions required):
  Add to ~/.config/hypr/hyprland.conf:
       bind = SUPER, mouse:275, exec, hyprvyl --toggle
================================================================================
"#,
        dev_str
    );
}

/// Discovers candidate input devices supporting the target key.
fn discover_devices(target_key: KeyCode, explicit_device: &str) -> Vec<Device> {
    if !explicit_device.is_empty() {
        let path = Path::new(explicit_device);
        match Device::open(path) {
            Ok(dev) => {
                println!("[Hyprvyl Trigger] Using explicit input device: {} ({:?})", path.display(), dev.name());
                return vec![dev];
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    report_permission_error(Some(path));
                } else {
                    eprintln!("[Hyprvyl Trigger] Failed to open specified device '{}': {}", explicit_device, e);
                }
                return Vec::new();
            }
        }
    }

    let mut matched_devices = Vec::new();
    let mut permission_denied_encountered = false;

    // Enumerate /dev/input/event*
    if let Ok(entries) = fs::read_dir("/dev/input") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(fname) = path.file_name().and_then(|n| n.to_str())
                && fname.starts_with("event") {
                    match Device::open(&path) {
                        Ok(dev) => {
                            if let Some(keys) = dev.supported_keys()
                                && keys.contains(target_key) {
                                    println!(
                                        "[Hyprvyl Trigger] Auto-detected input device: {} ({:?})",
                                        path.display(),
                                        dev.name().unwrap_or("Unnamed Device")
                                    );
                                    matched_devices.push(dev);
                                }
                        }
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::PermissionDenied {
                                permission_denied_encountered = true;
                            }
                        }
                    }
                }
        }
    }

    if matched_devices.is_empty() {
        if permission_denied_encountered {
            report_permission_error(None);
        } else {
            println!(
                "[Hyprvyl Trigger] No active input device found matching key/button code {}. Waiting for events via IPC.",
                target_key.code()
            );
        }
    }

    matched_devices
}

/// Spawns the background evdev input listener threads.
pub fn start_trigger_listener(config: Config, ipc_sender: Sender<IpcCommand>) {
    let target_key = match parse_key(&config.trigger.button) {
        Some(k) => k,
        None => {
            eprintln!(
                "[Hyprvyl Trigger] Invalid button '{}' in config. Trigger listener aborted.",
                config.trigger.button
            );
            return;
        }
    };

    let devices = discover_devices(target_key, &config.trigger.device);
    if devices.is_empty() {
        println!("[Hyprvyl Trigger] Evdev background listener is idle (IPC trigger remains active).");
        return;
    }

    let mode = config.trigger.mode;
    let hold_duration = Duration::from_millis(config.trigger.hold_duration_ms);

    println!(
        "[Hyprvyl Trigger] Active trigger: {} (code {}) | Mode: {:?} | Hold Duration: {}ms | Devices monitored: {}",
        config.trigger.button,
        target_key.code(),
        mode,
        config.trigger.hold_duration_ms,
        devices.len()
    );

    // Shared state across device listener threads
    let is_held = Arc::new(AtomicBool::new(false));
    let press_epoch = Arc::new(AtomicU64::new(0));

    for mut dev in devices {
        let ipc_tx = ipc_sender.clone();
        let is_held_clone = is_held.clone();
        let press_epoch_clone = press_epoch.clone();

        thread::Builder::new()
            .name(format!("hyprvyl-evdev-{}", dev.name().unwrap_or("dev")))
            .spawn(move || {
                loop {
                    match dev.fetch_events() {
                        Ok(events) => {
                            for ev in events {
                                if ev.event_type() == EventType::KEY
                                    && ev.code() == target_key.code() {
                                        let val = ev.value();
                                        if val == 1 {
                                            // Press event
                                            let new_epoch = press_epoch_clone.fetch_add(1, Ordering::SeqCst) + 1;
                                            is_held_clone.store(true, Ordering::SeqCst);

                                            match mode {
                                                TriggerMode::Click => {
                                                    // Immediately toggle overlay
                                                    let _ = ipc_tx.send_blocking(IpcCommand::Toggle);
                                                }
                                                TriggerMode::Hold => {
                                                    // Spawn hold timer thread
                                                    let tx = ipc_tx.clone();
                                                    let held = is_held_clone.clone();
                                                    let epoch_tracker = press_epoch_clone.clone();

                                                    thread::spawn(move || {
                                                        thread::sleep(hold_duration);
                                                        // If still held and this is still the same press epoch
                                                        if held.load(Ordering::SeqCst)
                                                            && epoch_tracker.load(Ordering::SeqCst) == new_epoch
                                                        {
                                                            let _ = tx.send_blocking(IpcCommand::Show);
                                                        }
                                                    });
                                                }
                                            }
                                        } else if val == 0 {
                                            // Release event
                                            is_held_clone.store(false, Ordering::SeqCst);
                                            if mode == TriggerMode::Hold {
                                                let _ = ipc_tx.send_blocking(IpcCommand::Release);
                                            }
                                        }
                                        // val == 2 is autorepeat; ignored.
                                    }
                            }
                        }
                        Err(e) => {
                            eprintln!("[Hyprvyl Trigger] Device read error: {}. Exiting listener for this device.", e);
                            break;
                        }
                    }
                }
            })
            .expect("Failed to spawn evdev listener thread");
    }
}
