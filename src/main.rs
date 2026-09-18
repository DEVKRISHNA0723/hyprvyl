//! Hyprvyl — Minimal, high-performance application launcher for Hyprland (Rovyl-matched).

mod apps;
mod config;
mod favicons;
mod hyprland;
mod icons;
mod ipc;
mod overlay;
mod renderer;
mod settings;
mod trigger;

use config::Config;
use gtk4::gio;
use gtk4::prelude::*;
use overlay::OverlayWindow;
use settings::SettingsWindow;
use std::env;
use std::time::{Duration, Instant};

const APP_ID: &str = "org.hyprvyl.launcher";
const SETTINGS_APP_ID: &str = "org.hyprvyl.settings";

fn print_usage() {
    println!(
        r#"Hyprvyl — Minimal Radial Application Launcher for Hyprland

USAGE:
    hyprvyl [OPTIONS]

OPTIONS:
    --settings, -S       Open the dedicated Hyprvyl Settings application window
    --toggle, -t         Toggle overlay visibility (sends IPC to daemon)
    --show, -s           Show overlay at cursor position
    --hide               Hide overlay (enable click-through)
    --release            Simulate hold-trigger release (selects hovered app or cancels)
    --status             Query daemon status
    --reload, -r         Trigger background reload of applications & config via IPC
    --apps-count         Query number of loaded applications from daemon via IPC
    --list-apps          Scan and list all discovered applications and icons
    --verify-apps        Comprehensive pass/fail verification of all discovered apps
    --launch <ID/NAME>   Test launch an application by ID or name
    --bench-apps [N]     Benchmark application discovery performance (default: 50 runs)
    --cycle-test [N]     Run automated open/close cycle verification test (default: 20)
    --help, -h           Show this help message

Configuration file: ~/.config/hyprvyl/config.toml
Running without arguments starts the background daemon, evdev listener, and layer-shell surface.
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let flag = &args[1];
        match flag.as_str() {
            "--help" | "-h" => {
                print_usage();
                return;
            }
            "--settings" | "-S" => {
                match ipc::send_command("SETTINGS") {
                    Ok(resp) => {
                        print!("{}", resp);
                        return;
                    }
                    Err(_) => {
                        run_settings_app();
                        return;
                    }
                }
            }
            "--reload" | "-r" | "--refresh" => match ipc::send_command("RELOAD") {
                Ok(resp) => {
                    print!("{}", resp);
                    return;
                }
                Err(e) => {
                    eprintln!("[Hyprvyl] Failed to send RELOAD command: {}", e);
                    std::process::exit(1);
                }
            },
            "--apps-count" => match ipc::send_command("APPS") {
                Ok(resp) => {
                    print!("{}", resp);
                    return;
                }
                Err(e) => {
                    eprintln!("[Hyprvyl] Failed to query APPS count: {}", e);
                    std::process::exit(1);
                }
            },
            "--list-apps" => {
                let _ = gtk4::init();
                let app_list = apps::discover_applications();
                println!("\n{:<35} {:<30} {:<10} {:<35} LAUNCH COMMAND", "DESKTOP ID", "APP NAME", "TERMINAL", "ICON PATH");
                println!("{:-<140}", "");
                for app in &app_list {
                    let icon_str = app
                        .icon_path
                        .as_ref()
                        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                        .unwrap_or_else(|| "[NONE]".to_string());
                    let cmd_str = app.launch_command.join(" ");
                    println!(
                        "{:<35} {:<30} {:<10} {:<35} {}",
                        truncate_str(&app.id, 33),
                        truncate_str(&app.name, 28),
                        if app.terminal { "Yes" } else { "No" },
                        truncate_str(&icon_str, 33),
                        truncate_str(&cmd_str, 40)
                    );
                }
                println!("\nTotal Discovered Applications: {}", app_list.len());
                let term = apps::detect_terminal_emulator().unwrap_or_else(|| "None found".to_string());
                println!("Detected Terminal Emulator: {}", term);
                return;
            }
            "--release" => match ipc::send_command("RELEASE") {
                Ok(resp) => {
                    print!("{}", resp);
                    return;
                }
                Err(e) => {
                    eprintln!("[Hyprvyl] Failed to send RELEASE command: {}", e);
                    std::process::exit(1);
                }
            },
            "--verify-apps" => {
                let _ = gtk4::init();
                let app_list = apps::discover_applications();
                println!("\n========================================================================================================================");
                println!("                                    HYPRVYL APPLICATION VERIFICATION SUITE                                    ");
                println!("========================================================================================================================");
                println!("{:<4} {:<32} {:<24} {:<10} {:<12} {:<8} DETAILS", "#", "APP NAME", "DESKTOP ID", "BINARY", "TERMINAL", "RESULT");
                println!("{:-<120}", "");

                let mut pass_count = 0;
                let mut fail_count = 0;

                for (idx, app) in app_list.iter().enumerate() {
                    let (passed, binary_ok, term_ok, detail) = verify_app_launch_entry(app);
                    if passed {
                        pass_count += 1;
                    } else {
                        fail_count += 1;
                    }

                    let status_str = if passed { "PASS" } else { "FAIL" };
                    let binary_str = if binary_ok { "Found" } else { "Missing" };
                    let term_str = if app.terminal { if term_ok { "kitty (OK)" } else { "kitty (ERR)" } } else { "GUI" };

                    println!(
                        "{:<4} {:<32} {:<24} {:<10} {:<12} {:<8} {}",
                        idx + 1,
                        truncate_str(&app.name, 30),
                        truncate_str(&app.id, 22),
                        binary_str,
                        term_str,
                        status_str,
                        truncate_str(&detail, 38)
                    );
                }

                println!("{:=<120}", "");
                println!("TOTAL DISCOVERED APPS: {}", app_list.len());
                println!("PASSED:                {} / {} ({:.1}%)", pass_count, app_list.len(), (pass_count as f64 / app_list.len().max(1) as f64) * 100.0);
                println!("FAILED:                {}", fail_count);
                println!("========================================================================================================================\n");
                return;
            }
            "--launch" => {
                let target = match args.get(2) {
                    Some(t) => t,
                    None => {
                        eprintln!("[Hyprvyl] Usage: hyprvyl --launch <APP_ID_OR_NAME>");
                        std::process::exit(1);
                    }
                };
                let _ = gtk4::init();
                let app_list = apps::discover_applications();
                let target_lower = target.to_lowercase();
                let matched = app_list.iter().find(|a| {
                    a.id.to_lowercase() == target_lower
                        || a.id.to_lowercase().starts_with(&target_lower)
                        || a.name.to_lowercase() == target_lower
                });
                match matched {
                    Some(app) => {
                        println!("[Hyprvyl] Found application: '{}' (ID: {})", app.name, app.id);
                        println!("  Exec: {:?}", app.exec);
                        println!("  Terminal: {}", app.terminal);
                        println!("  Launch Command: {:?}", app.launch_command);
                        println!("  Icon: {:?}", app.icon_path);
                        if let Err(e) = apps::launch_app(app) {
                            eprintln!("[Hyprvyl] Launch error: {}", e);
                            std::process::exit(1);
                        } else {
                            println!("[Hyprvyl] Process launched successfully.");
                        }
                    }
                    None => {
                        eprintln!("[Hyprvyl] Application '{}' not found in discovered apps.", target);
                        std::process::exit(1);
                    }
                }
                return;
            }
            "--bench-apps" => {
                let runs: usize = args
                    .get(2)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(50);
                let _ = gtk4::init();
                println!("[Hyprvyl Benchmark] Running {} application discovery iterations...", runs);
                let mut total_duration = Duration::ZERO;
                let mut min_duration = Duration::MAX;
                let mut max_duration = Duration::ZERO;
                let mut app_count = 0;

                for _ in 0..runs {
                    let start = Instant::now();
                    let list = apps::discover_applications();
                    let elapsed = start.elapsed();
                    app_count = list.len();
                    total_duration += elapsed;
                    if elapsed < min_duration {
                        min_duration = elapsed;
                    }
                    if elapsed > max_duration {
                        max_duration = elapsed;
                    }
                }

                let avg_ms = (total_duration.as_secs_f64() * 1000.0) / (runs as f64);
                let min_ms = min_duration.as_secs_f64() * 1000.0;
                let max_ms = max_duration.as_secs_f64() * 1000.0;

                println!("\n[Hyprvyl Benchmark Results]");
                println!("  Discovered Apps:   {}", app_count);
                println!("  Iterations:        {}", runs);
                println!("  Average Scan Time: {:.2} ms", avg_ms);
                println!("  Fastest Scan:      {:.2} ms", min_ms);
                println!("  Slowest Scan:      {:.2} ms", max_ms);
                return;
            }
            "--toggle" | "-t" => match ipc::send_command("TOGGLE") {
                Ok(resp) => {
                    print!("{}", resp);
                    return;
                }
                Err(_) => {
                    eprintln!("[Hyprvyl] Daemon not running. Launching new instance with overlay shown...");
                }
            },
            "--show" | "-s" => match ipc::send_command("SHOW") {
                Ok(resp) => {
                    print!("{}", resp);
                    return;
                }
                Err(_) => {
                    eprintln!("[Hyprvyl] Daemon not running. Launching new instance with overlay shown...");
                }
            },
            "--hide" => match ipc::send_command("HIDE") {
                Ok(resp) => {
                    print!("{}", resp);
                    return;
                }
                Err(e) => {
                    eprintln!("[Hyprvyl] Failed to send HIDE command: {}", e);
                    std::process::exit(1);
                }
            },
            "--status" => match ipc::send_command("STATUS") {
                Ok(resp) => {
                    print!("{}", resp);
                    return;
                }
                Err(e) => {
                    eprintln!("[Hyprvyl] Daemon is not running ({})", e);
                    std::process::exit(1);
                }
            },
            "--cycle-test" => {
                let cycles: usize = args
                    .get(2)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(20);
                run_cycle_test(cycles);
                return;
            }
            _ => {
                eprintln!("[Hyprvyl] Unknown option '{}'. Use --help for usage.", flag);
                std::process::exit(1);
            }
        }
    }

    // Load or generate configuration
    let config = Config::load_or_create();

    // Start GTK4 Application
    let app = gtk4::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    let initial_show = args.iter().any(|a| a == "--show" || a == "-s" || a == "--toggle" || a == "-t");

    app.connect_activate(move |app| {
        let overlay = OverlayWindow::new(app, config.clone());

        // Pre-discover and cache applications in memory
        let app_list = apps::discover_applications();
        overlay.set_apps(app_list);

        // Start background file-watch invalidation listener for live updates
        if config.advanced.live_watcher {
            let (invalidation_tx, invalidation_rx) = async_channel::unbounded::<()>();
            let overlay_for_watcher = overlay.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                while let Ok(()) = invalidation_rx.recv().await {
                    println!("[Hyprvyl] Live invalidation triggered. Rescanning application entries...");
                    let fresh_apps = apps::discover_applications();
                    println!(
                        "[Hyprvyl] App registry refreshed: {} applications currently registered.",
                        fresh_apps.len()
                    );
                    overlay_for_watcher.set_apps(fresh_apps);
                }
            });
            apps::start_desktop_file_watcher(invalidation_tx);
        }

        // Start IPC server
        match ipc::start_server(overlay.clone()) {
            Ok(ipc_sender) => {
                // Start background evdev input listener
                trigger::start_trigger_listener(config.clone(), ipc_sender);
            }
            Err(e) => {
                eprintln!("[Hyprvyl] Warning: Failed to start IPC socket server: {}", e);
            }
        }

        if initial_show {
            overlay.show_at_cursor();
        }

        println!("[Hyprvyl] Daemon running. Ready for layer-shell overlay triggers.");
    });

    app.run_with_args::<String>(&[]);
}

/// Runs the standalone Settings GUI application.
fn run_settings_app() {
    let app = gtk4::Application::builder()
        .application_id(SETTINGS_APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(|app| {
        let settings_win = SettingsWindow::new(app);
        settings_win.present();
    });

    app.run_with_args::<String>(&[]);
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}

/// Automated cycle tester to verify open/close stability under Hyprland.
fn run_cycle_test(cycles: usize) {
    println!("[Hyprvyl Test] Starting {} open/close verification cycles via IPC...", cycles);

    // Verify daemon ping
    if let Err(e) = ipc::send_command("PING") {
        eprintln!("[Hyprvyl Test] Error: Hyprvyl daemon is not running. Please start 'hyprvyl' first.");
        eprintln!("Details: {}", e);
        std::process::exit(1);
    }

    for i in 1..=cycles {
        // Show
        let show_resp = ipc::send_command("SHOW").expect("Failed to send SHOW command");
        assert!(show_resp.contains("OK"));

        // Small pause to allow compositor frame rendering
        std::thread::sleep(Duration::from_millis(60));

        // Query status
        let status_resp = ipc::send_command("STATUS").expect("Failed to query STATUS");
        assert!(status_resp.contains("VISIBLE=true"));

        // Hide
        let hide_resp = ipc::send_command("HIDE").expect("Failed to send HIDE command");
        assert!(hide_resp.contains("OK"));

        std::thread::sleep(Duration::from_millis(40));

        let status_hide = ipc::send_command("STATUS").expect("Failed to query STATUS");
        assert!(status_hide.contains("VISIBLE=false"));

        if i % 5 == 0 || i == cycles {
            println!("[Hyprvyl Test] Completed cycle {}/{}", i, cycles);
        }
    }

    println!("[Hyprvyl Test] SUCCESS: All {} open/close cycles completed flawlessly with zero errors.", cycles);
}

/// Validates an application entry for complete launch readiness.
fn verify_app_launch_entry(app: &apps::AppEntry) -> (bool, bool, bool, String) {
    if app.launch_command.is_empty() {
        return (false, false, false, "Empty launch command".to_string());
    }

    let program = &app.launch_command[0];
    let terminal_detected = apps::detect_terminal_emulator();
    let is_terminal_app = app.terminal;
    let term_ok = true;

    // Check terminal emulator wrapping
    if is_terminal_app {
        if let Some(ref term) = terminal_detected {
            if program != term {
                return (false, true, false, format!("Not wrapped with detected terminal '{}'", term));
            }
        } else {
            return (false, true, false, "Terminal required but no terminal emulator detected".to_string());
        }
    }

    // Check actual target executable availability
    let actual_binary = if is_terminal_app {
        if app.launch_command.len() > 2 {
            &app.launch_command[2]
        } else {
            &app.launch_command[0]
        }
    } else {
        &app.launch_command[0]
    };

    let binary_exists = is_binary_executable(actual_binary);
    if !binary_exists {
        return (false, false, term_ok, format!("Executable '{}' not found in PATH", actual_binary));
    }

    // Check for illegal leftover raw field codes
    for token in &app.launch_command {
        if token == "%f" || token == "%F" || token == "%u" || token == "%U" || token == "%d" || token == "%n" {
            return (false, binary_exists, term_ok, format!("Unstripped field code '{}'", token));
        }
    }

    let details = format!("Exec: {}", app.exec);
    (true, binary_exists, term_ok, details)
}

fn is_binary_executable(bin: &str) -> bool {
    let path = std::path::Path::new(bin);
    if path.is_absolute() || bin.contains('/') {
        return path.is_file();
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let full = std::path::Path::new(dir).join(bin);
            if full.is_file() {
                return true;
            }
        }
    }
    false
}
