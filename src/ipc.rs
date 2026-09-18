//! Unix Domain Socket IPC for Hyprvyl.
//!
//! Enables external triggers (such as Hyprland keybinds, CLI commands, or scripts)
//! to instantaneously control the Hyprvyl overlay daemon without startup delay.

use crate::overlay::OverlayWindow;
use gtk4::glib;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::Sender as SyncSender;
use std::thread;
use std::time::Duration;

/// Commands supported by the Hyprvyl IPC daemon.
#[derive(Debug)]
pub enum IpcCommand {
    Show,
    Hide,
    Toggle,
    Release,
    OpenSettings,
    GetStatus(SyncSender<bool>),
    GetAppCount(SyncSender<usize>),
    Reload(SyncSender<usize>),
}

/// Returns the default path for the Hyprvyl IPC socket.
pub fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("hyprvyl.sock")
    } else {
        let runtime = glib::user_runtime_dir();
        runtime.join("hyprvyl.sock")
    }
}

/// Sends an IPC command to an existing Hyprvyl daemon.
pub fn send_command(command: &str) -> Result<String, Box<dyn std::error::Error>> {
    let socket_path = get_socket_path();
    let mut stream = UnixStream::connect(&socket_path)?;
    stream.set_read_timeout(Some(Duration::from_millis(2000)))?;
    stream.set_write_timeout(Some(Duration::from_millis(2000)))?;
    let cmd_payload = format!("{}\n", command.trim());
    stream.write_all(cmd_payload.as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

/// Starts the IPC listener and dispatches events onto the GTK main loop.
pub fn start_server(overlay: Rc<OverlayWindow>) -> std::io::Result<async_channel::Sender<IpcCommand>> {
    let socket_path = get_socket_path();

    if socket_path.exists() {
        let _ = fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path)?;
    println!("[Hyprvyl] IPC socket listening at: {}", socket_path.display());

    let (sender, receiver) = async_channel::unbounded::<IpcCommand>();

    // Attach async receiver to GTK main context
    let overlay_for_events = overlay;
    glib::MainContext::default().spawn_local(async move {
        while let Ok(cmd) = receiver.recv().await {
            match cmd {
                IpcCommand::Show => {
                    overlay_for_events.show_at_cursor();
                }
                IpcCommand::Hide => {
                    overlay_for_events.hide();
                }
                IpcCommand::Toggle => {
                    overlay_for_events.toggle();
                }
                IpcCommand::Release => {
                    overlay_for_events.handle_trigger_release();
                }
                IpcCommand::OpenSettings => {
                    overlay_for_events.open_settings();
                }
                IpcCommand::GetStatus(resp_tx) => {
                    let _ = resp_tx.send(overlay_for_events.is_visible());
                }
                IpcCommand::GetAppCount(resp_tx) => {
                    let _ = resp_tx.send(overlay_for_events.get_app_count());
                }
                IpcCommand::Reload(resp_tx) => {
                    let fresh_cfg = crate::config::Config::load_or_create();
                    overlay_for_events.update_config(fresh_cfg);
                    let fresh_apps = crate::apps::discover_applications();
                    let count = fresh_apps.len();
                    overlay_for_events.set_apps(fresh_apps);
                    crate::favicons::clear_favicon_fetch_state();
                    let _ = resp_tx.send(count);
                }
            }
        }
    });

    // Background thread accepting client socket connections
    let sender_for_thread = sender.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut sock) => {
                    let _ = sock.set_read_timeout(Some(Duration::from_millis(800)));
                    let _ = sock.set_write_timeout(Some(Duration::from_millis(800)));
                    let mut reader = BufReader::new(&sock);
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_ok() && !line.trim().is_empty() {
                        let cmd_str = line.trim().to_uppercase();
                        match cmd_str.as_str() {
                            "SHOW" => {
                                let _ = sender_for_thread.send_blocking(IpcCommand::Show);
                                let _ = sock.write_all(b"OK: SHOWN\n");
                            }
                            "HIDE" => {
                                let _ = sender_for_thread.send_blocking(IpcCommand::Hide);
                                let _ = sock.write_all(b"OK: HIDDEN\n");
                            }
                            "TOGGLE" => {
                                let _ = sender_for_thread.send_blocking(IpcCommand::Toggle);
                                let _ = sock.write_all(b"OK: TOGGLED\n");
                            }
                            "RELEASE" => {
                                let _ = sender_for_thread.send_blocking(IpcCommand::Release);
                                let _ = sock.write_all(b"OK: RELEASED\n");
                            }
                            "SETTINGS" | "CONFIG" | "PREFERENCES" => {
                                let _ = sender_for_thread.send_blocking(IpcCommand::OpenSettings);
                                let _ = sock.write_all(b"OK: SETTINGS OPENED\n");
                            }
                            "STATUS" => {
                                let (tx, rx) = std::sync::mpsc::channel();
                                let _ = sender_for_thread.send_blocking(IpcCommand::GetStatus(tx));
                                let is_vis = rx.recv_timeout(Duration::from_millis(1500)).unwrap_or(false);
                                let resp = format!("OK: VISIBLE={}\n", is_vis);
                                let _ = sock.write_all(resp.as_bytes());
                            }
                            "APPS" | "COUNT" => {
                                let (tx, rx) = std::sync::mpsc::channel();
                                let _ = sender_for_thread.send_blocking(IpcCommand::GetAppCount(tx));
                                let count = rx.recv_timeout(Duration::from_millis(1500)).unwrap_or(0);
                                let resp = format!("OK: APPS={}\n", count);
                                let _ = sock.write_all(resp.as_bytes());
                            }
                            "RELOAD" | "REFRESH" => {
                                let (tx, rx) = std::sync::mpsc::channel();
                                let _ = sender_for_thread.send_blocking(IpcCommand::Reload(tx));
                                let count = rx.recv_timeout(Duration::from_millis(2500)).unwrap_or(0);
                                let resp = format!("OK: RELOADED count={}\n", count);
                                let _ = sock.write_all(resp.as_bytes());
                            }
                            "PING" => {
                                let _ = sock.write_all(b"PONG\n");
                            }
                            _ => {
                                let resp = format!("ERR: UNKNOWN COMMAND '{}'\n", cmd_str);
                                let _ = sock.write_all(resp.as_bytes());
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("[Hyprvyl] IPC accept error: {}", err);
                }
            }
        }
    });

    Ok(sender)
}
