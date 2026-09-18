//! Application Discovery and Icon Resolution for Hyprvyl (Stage 3).
//!
//! Scans XDG application directories per the Freedesktop Desktop Entry Specification,
//! extracts metadata (Name, localized variants, Exec, Icon, Terminal, Categories, etc.),
//! filters out NoDisplay/Hidden/DE-incompatible entries, resolves icons using GTK4
//! icon themes and standard pixmap paths, and prepares launch commands.

use gtk4::gdk;
use gtk4::prelude::*;
use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Represents a fully resolved, launchable application entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    /// Desktop file identifier (e.g. "org.mozilla.firefox.desktop" or "kitty.desktop")
    pub id: String,
    /// Display name (localized if available)
    pub name: String,
    /// Generic name (e.g. "Web Browser")
    pub generic_name: Option<String>,
    /// Description / tooltip comment
    pub comment: Option<String>,
    /// Sanitized executable command line string (field codes like %u, %F removed)
    pub exec: String,
    /// Executable arguments parsed into separate tokens
    pub exec_args: Vec<String>,
    /// Whether the app requires running inside a terminal emulator
    pub terminal: bool,
    /// Complete command to launch (includes terminal emulator wrapper if terminal=true)
    pub launch_command: Vec<String>,
    /// Original icon name or path specified in the .desktop file
    pub icon_name: String,
    /// Resolved absolute path to an icon file (.svg, .png, etc.), or fallback icon path
    pub icon_path: Option<PathBuf>,
    /// List of categories (e.g. ["Network", "WebBrowser"])
    pub categories: Vec<String>,
    /// List of search keywords
    pub keywords: Vec<String>,
    /// Working directory (from Path= in desktop file)
    pub working_dir: Option<PathBuf>,
    /// Source .desktop file path
    pub desktop_file: PathBuf,
}

/// Discovers and returns all valid XDG application directories in priority order.
/// Discovers and returns all valid XDG application directories in priority order.
///
/// Priority (later entries in the returned list override earlier ones for duplicate desktop IDs):
/// 1. Flatpak system exports / Snap system applications
/// 2. /usr/share/applications (and lowest-priority entries in $XDG_DATA_DIRS)
/// 3. /usr/local/share/applications (and higher-priority entries in $XDG_DATA_DIRS)
/// 4. Flatpak user exports (~/.local/share/flatpak/exports/share/applications)
/// 5. ~/.local/share/applications ($XDG_DATA_HOME/applications — highest priority)
pub fn get_application_search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    // 1. Base system directories (lowest priority)
    let default_system_dirs = [
        PathBuf::from("/var/lib/snapd/desktop/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    for d in default_system_dirs {
        if !dirs.contains(&d) {
            dirs.push(d);
        }
    }

    // 2. Directories from $XDG_DATA_DIRS in reverse order so higher priority entries appear later
    if let Ok(data_dirs_env) = env::var("XDG_DATA_DIRS") {
        let parts: Vec<&str> = data_dirs_env.split(':').collect();
        for part in parts.into_iter().rev() {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                let app_dir = PathBuf::from(trimmed).join("applications");
                if !dirs.contains(&app_dir) {
                    dirs.push(app_dir);
                }
            }
        }
    }

    // 3. User-local directories (highest priority, appended last so they override system entries)
    let data_home = match env::var("XDG_DATA_HOME") {
        Ok(val) if !val.is_empty() => PathBuf::from(val),
        _ => {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local").join("share")
        }
    };

    let user_flatpak_dir = data_home.join("flatpak").join("exports").join("share").join("applications");
    let user_app_dir = data_home.join("applications");

    if !dirs.contains(&user_flatpak_dir) {
        dirs.push(user_flatpak_dir);
    }
    if !dirs.contains(&user_app_dir) {
        dirs.push(user_app_dir);
    }

    dirs
}

/// Detects the user's preferred or installed terminal emulator.
///
/// Priority:
/// 1. $TERMINAL environment variable
/// 2. Common Wayland / Linux terminal emulators in order: kitty, alacritty, foot, wezterm, gnome-terminal, konsole, xfce4-terminal, xterm
pub fn detect_terminal_emulator() -> Option<String> {
    if let Ok(term) = env::var("TERMINAL") {
        let trimmed = term.trim();
        if !trimmed.is_empty() && is_command_available(trimmed) {
            return Some(trimmed.to_string());
        }
    }

    let candidates = [
        "kitty",
        "alacritty",
        "foot",
        "wezterm",
        "gnome-terminal",
        "konsole",
        "xfce4-terminal",
        "xterm",
    ];

    for candidate in candidates {
        if is_command_available(candidate) {
            return Some(candidate.to_string());
        }
    }

    None
}

/// Checks if a binary command is executable and exists in PATH or at an absolute path.
fn is_command_available(cmd: &str) -> bool {
    let binary = cmd.split_whitespace().next().unwrap_or(cmd);
    let path = Path::new(binary);
    if path.is_absolute() || binary.contains('/') {
        return path.is_file();
    }
    if let Ok(path_var) = env::var("PATH") {
        for dir in path_var.split(':') {
            let full_path = Path::new(dir).join(binary);
            if full_path.is_file() {
                return true;
            }
        }
    }
    false
}

/// Cleans and tokenizes an Exec line from a .desktop file, handling all Freedesktop field codes.
///
/// Field codes handled:
/// - `%f`, `%F`, `%u`, `%U`, `%d`, `%D`, `%n`, `%N`, `%v`, `%m`: stripped/omitted for zero-argument launcher invocation.
/// - Options containing field codes (e.g. `--uri=%u`, `--file=%f`, `-f=%F`): stripped.
/// - `%%`: replaced with literal `%`.
/// - `%i`: expanded to `--icon <icon>` if icon name is available.
/// - `%c`: expanded to translated application name.
/// - `%k`: expanded to desktop file path.
#[allow(dead_code)]
pub fn parse_exec_line(exec_raw: &str) -> (String, Vec<String>) {
    parse_exec_line_full(exec_raw, None, None, None)
}

/// Tokenizes and substitutes Freedesktop field codes with full application metadata context.
pub fn parse_exec_line_full(
    exec_raw: &str,
    app_name: Option<&str>,
    icon_name: Option<&str>,
    desktop_file: Option<&Path>,
) -> (String, Vec<String>) {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';
    let mut chars = exec_raw.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '\\' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == quote_char || next_ch == '\\' || next_ch == '$' || next_ch == '`' {
                        current.push(chars.next().unwrap());
                    } else {
                        current.push(ch);
                    }
                } else {
                    current.push(ch);
                }
            } else if ch == quote_char {
                in_quotes = false;
            } else {
                current.push(ch);
            }
        } else if ch == '\\' {
            if let Some(&next_ch) = chars.peek() {
                if next_ch == ' ' || next_ch == '\t' || next_ch == '\\' || next_ch == '"' || next_ch == '\'' {
                    current.push(chars.next().unwrap());
                } else {
                    current.push(ch);
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' || ch == '\'' {
            in_quotes = true;
            quote_char = ch;
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(current);
                current = String::new();
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    let mut filtered_tokens: Vec<String> = Vec::new();

    for token in tokens {
        // Freedesktop field codes to omit completely when launching without files/URLs
        if token == "%f"
            || token == "%F"
            || token == "%u"
            || token == "%U"
            || token == "%d"
            || token == "%D"
            || token == "%n"
            || token == "%N"
            || token == "%v"
            || token == "%m"
        {
            continue;
        }

        // Option with field code value, e.g. --uri=%u, --file=%F, -f=%f, --open=%u
        if (token.starts_with("--") || token.starts_with('-'))
            && (token.ends_with("=%u")
                || token.ends_with("=%U")
                || token.ends_with("=%f")
                || token.ends_with("=%F")
                || token.ends_with("=%d")
                || token.ends_with("=%D"))
        {
            continue;
        }

        // %i: expand to --icon <icon_name> if icon exists, or drop
        if token == "%i" {
            if let Some(icon) = icon_name.filter(|s| !s.trim().is_empty()) {
                filtered_tokens.push("--icon".to_string());
                filtered_tokens.push(icon.to_string());
            }
            continue;
        }

        // %c: expand to translated application name
        if token == "%c" {
            if let Some(name) = app_name.filter(|s| !s.trim().is_empty()) {
                filtered_tokens.push(name.to_string());
            }
            continue;
        }

        // %k: expand to desktop file path
        if token == "%k" {
            if let Some(path) = desktop_file {
                filtered_tokens.push(path.to_string_lossy().to_string());
            }
            continue;
        }

        // Handle inline replacements within a token (e.g. %% -> %, %c -> name, %k -> path)
        let mut processed = token;
        if processed.contains("%%") {
            processed = processed.replace("%%", "\x00_PERCENT_\x00");
        }
        if processed.contains("%c") {
            if let Some(name) = app_name {
                processed = processed.replace("%c", name);
            } else {
                processed = processed.replace("%c", "");
            }
        }
        if processed.contains("%k") {
            if let Some(path) = desktop_file {
                processed = processed.replace("%k", &path.to_string_lossy());
            } else {
                processed = processed.replace("%k", "");
            }
        }
        // Strip any residual %f/%F/%u/%U from compound options
        if processed.contains("%f") {
            processed = processed.replace("%f", "");
        }
        if processed.contains("%F") {
            processed = processed.replace("%F", "");
        }
        if processed.contains("%u") {
            processed = processed.replace("%u", "");
        }
        if processed.contains("%U") {
            processed = processed.replace("%U", "");
        }
        if processed.contains("\x00_PERCENT_\x00") {
            processed = processed.replace("\x00_PERCENT_\x00", "%");
        }

        if !processed.is_empty() {
            filtered_tokens.push(processed);
        }
    }

    let clean_exec_str = filtered_tokens.join(" ");
    (clean_exec_str, filtered_tokens)
}

/// Determines the current locale identifiers to match against localized keys like Name[en_US], Name[en].
fn get_locale_candidates() -> Vec<String> {
    let mut locales = Vec::new();
    let raw = env::var("LC_ALL")
        .or_else(|_| env::var("LC_MESSAGES"))
        .or_else(|_| env::var("LANG"))
        .unwrap_or_else(|_| "en_US.UTF-8".to_string());

    // Clean up e.g. "en_US.UTF-8" -> full "en_US", lang "en"
    let without_encoding = raw.split('.').next().unwrap_or(&raw);
    let without_modifier = without_encoding.split('@').next().unwrap_or(without_encoding);

    if !without_modifier.is_empty() {
        locales.push(without_modifier.to_string());
        if let Some(lang) = without_modifier.split('_').next()
            && lang != without_modifier && !lang.is_empty() {
                locales.push(lang.to_string());
            }
    }

    locales
}

/// Parses a single .desktop file into raw key-value pairs belonging to the [Desktop Entry] section.
fn parse_desktop_entry_section(path: &Path) -> Option<HashMap<String, String>> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let mut in_desktop_entry = false;
    let mut map = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = &trimmed[1..trimmed.len() - 1];
            in_desktop_entry = section == "Desktop Entry";
            continue;
        }

        if in_desktop_entry
            && let Some((key, value)) = trimmed.split_once('=') {
                map.insert(key.trim().to_string(), value.trim().to_string());
            }
    }

    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

/// Resolves an icon name or absolute path to a concrete image file path.
///
/// Priority:
/// 1. If it's an existing absolute file path -> return it directly.
/// 2. Lookup via GTK4's active IconTheme (respects user's GTK theme).
/// 3. Lookup across standard user & system icon directories (including AppImage / hicolor paths).
/// 4. Fallback check in /usr/share/pixmaps.
/// 5. Generic application fallback icon.
pub fn resolve_icon(icon_name: &str, target_size: i32) -> Option<PathBuf> {
    let trimmed = icon_name.trim();
    if trimmed.is_empty() {
        return find_fallback_icon(target_size);
    }

    // 1. Direct absolute path check
    let direct_path = Path::new(trimmed);
    if direct_path.is_absolute() && direct_path.is_file() {
        return Some(direct_path.to_path_buf());
    }

    // Strip extension if specified (e.g. "firefox.png" -> lookup "firefox", "app.desktop" -> "app")
    let base_name = trimmed
        .strip_suffix(".desktop")
        .unwrap_or(trimmed);
    let base_name = base_name
        .strip_suffix(".png")
        .or_else(|| base_name.strip_suffix(".svg"))
        .or_else(|| base_name.strip_suffix(".xpm"))
        .unwrap_or(base_name);

    // 2. GTK4 IconTheme lookup (guarded for main thread execution)
    if gtk4::is_initialized_main_thread()
        && let Some(display) = gdk::Display::default()
    {
        let theme = gtk4::IconTheme::for_display(&display);
        let base_lower = base_name.to_lowercase();

        for query_name in [trimmed, base_name, &base_lower] {
            let paintable = theme.lookup_icon(
                query_name,
                &[],
                target_size,
                1, // 1x scale base
                gtk4::TextDirection::None,
                gtk4::IconLookupFlags::empty(),
            );

                if let Some(file) = paintable.file()
                    && let Some(path) = file.path()
                        && path.is_file() {
                            return Some(path);
                        }
            }
        }

    // 3. User & System icon directory direct checks (including non-standard AppImage 0x0 or hicolor paths)
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let search_roots = [
        PathBuf::from(&home).join(".local/share/icons"),
        PathBuf::from(&home).join(".icons"),
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/local/share/icons"),
        PathBuf::from("/usr/share/pixmaps"),
    ];

    let icon_extensions = ["svg", "png", "xpm"];

    for root in &search_roots {
        if !root.is_dir() {
            continue;
        }

        // Direct root match
        for ext in icon_extensions {
            let p = root.join(format!("{}.{}", base_name, ext));
            if p.is_file() {
                return Some(p);
            }
        }
        let direct_in_root = root.join(trimmed);
        if direct_in_root.is_file() {
            return Some(direct_in_root);
        }

        // Check common subdirectories: hicolor/0x0/apps, hicolor/scalable/apps, hicolor/48x48/apps, etc.
        let subdirs = [
            "hicolor/scalable/apps",
            "hicolor/48x48/apps",
            "hicolor/0x0/apps",
            "hicolor/64x64/apps",
            "hicolor/32x32/apps",
            "hicolor/128x128/apps",
            "hicolor/256x256/apps",
            "hicolor/symbolic/apps",
            "breeze/apps/48",
            "Papirus/48x48/apps",
            "Qogir/scalable/apps",
        ];

        for sub in subdirs {
            for ext in icon_extensions {
                let p = root.join(sub).join(format!("{}.{}", base_name, ext));
                if p.is_file() {
                    return Some(p);
                }
            }
            let p = root.join(sub).join(trimmed);
            if p.is_file() {
                return Some(p);
            }
        }
    }

    // 4. Return generic application fallback icon
    find_fallback_icon(target_size)
}

/// Finds a sensible fallback icon on the system.
pub fn find_fallback_icon(target_size: i32) -> Option<PathBuf> {
    let fallback_names = [
        "application-x-executable",
        "system-run",
        "applications-other",
        "application-default-icon",
        "image-missing",
    ];

    if let Some(display) = gdk::Display::default() {
        let theme = gtk4::IconTheme::for_display(&display);
        for name in fallback_names {
            let paintable = theme.lookup_icon(
                name,
                &[],
                target_size,
                1,
                gtk4::TextDirection::None,
                gtk4::IconLookupFlags::empty(),
            );
            if let Some(file) = paintable.file()
                && let Some(path) = file.path()
                    && path.is_file() {
                        return Some(path);
                    }
        }
    }

    // Hardcoded standard system pixmap and icon fallbacks
    for name in fallback_names {
        for ext in ["svg", "png"] {
            let p = PathBuf::from(format!("/usr/share/pixmaps/{}.{}", name, ext));
            if p.is_file() {
                return Some(p);
            }
            let p2 = PathBuf::from(format!("/usr/share/icons/hicolor/scalable/apps/{}.{}", name, ext));
            if p2.is_file() {
                return Some(p2);
            }
        }
    }

    None
}

/// Scans all XDG application directories and returns the de-duplicated, sorted list of applications.
pub fn discover_applications() -> Vec<AppEntry> {
    let start_time = Instant::now();
    let search_dirs = get_application_search_dirs();
    let terminal_emulator = detect_terminal_emulator();
    let locales = get_locale_candidates();

    let current_desktop = env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_else(|_| "Hyprland".to_string())
        .to_uppercase();
    let current_desktops: HashSet<String> = current_desktop
        .split(':')
        .map(|s| s.trim().to_string())
        .collect();

    // Map: desktop_id -> AppEntry. Iterating in ascending directory priority means later entries overwrite earlier ones.
    let mut apps_map: HashMap<String, AppEntry> = HashMap::new();
    let mut total_files_scanned = 0;
    let mut skipped_count = 0;

    for dir in search_dirs {
        if !dir.is_dir() {
            continue;
        }

        // Read all entries in the directory (non-recursive per Freedesktop spec for desktop files)
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
                    continue;
                }

                total_files_scanned += 1;
                let file_name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };

                let fields = match parse_desktop_entry_section(&path) {
                    Some(f) => f,
                    None => {
                        skipped_count += 1;
                        continue;
                    }
                };

                // Type must be Application (or omitted/default Application)
                if let Some(t) = fields.get("Type")
                    && t != "Application" {
                        skipped_count += 1;
                        continue;
                    }

                // Filter NoDisplay=true and Hidden=true
                if fields.get("NoDisplay").map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(false) {
                    continue;
                }
                if fields.get("Hidden").map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(false) {
                    continue;
                }

                // Filter OnlyShowIn / NotShowIn
                if let Some(only_show) = fields.get("OnlyShowIn") {
                    let allowed: HashSet<String> = only_show
                        .split(';')
                        .map(|s| s.trim().to_uppercase())
                        .filter(|s| !s.is_empty())
                        .collect();

                    let is_allowed = allowed.iter().any(|de| current_desktops.contains(de) || de == "GENERIC" || de == "WAYLAND");
                    if !is_allowed {
                        continue;
                    }
                }

                if let Some(not_show) = fields.get("NotShowIn") {
                    let disallowed: HashSet<String> = not_show
                        .split(';')
                        .map(|s| s.trim().to_uppercase())
                        .filter(|s| !s.is_empty())
                        .collect();

                    let is_disallowed = disallowed.iter().any(|de| current_desktops.contains(de));
                    if is_disallowed {
                        continue;
                    }
                }

                // Check TryExec (spec requirement: if binary not found/executable, skip entry)
                if let Some(try_exec) = fields.get("TryExec") {
                    let trimmed = try_exec.trim();
                    if !trimmed.is_empty() && !is_command_available(trimmed) {
                        skipped_count += 1;
                        continue;
                    }
                }

                // Extract Exec (required)
                let exec_raw = match fields.get("Exec") {
                    Some(e) if !e.trim().is_empty() => e.trim(),
                    _ => {
                        eprintln!("[Hyprvyl Apps] Skipping {}: missing required 'Exec' key", path.display());
                        skipped_count += 1;
                        continue;
                    }
                };

                // Extract Name (required) - search localized variants first
                let mut name: Option<String> = None;
                for loc in &locales {
                    let key = format!("Name[{}]", loc);
                    if let Some(n) = fields.get(&key)
                        && !n.trim().is_empty() {
                            name = Some(n.trim().to_string());
                            break;
                        }
                }
                if name.is_none()
                    && let Some(n) = fields.get("Name")
                        && !n.trim().is_empty() {
                            name = Some(n.trim().to_string());
                        }

                let name = match name {
                    Some(n) => n,
                    None => {
                        eprintln!("[Hyprvyl Apps] Skipping {}: missing required 'Name' key", path.display());
                        skipped_count += 1;
                        continue;
                    }
                };

                // Extract GenericName
                let mut generic_name: Option<String> = None;
                for loc in &locales {
                    let key = format!("GenericName[{}]", loc);
                    if let Some(gn) = fields.get(&key)
                        && !gn.trim().is_empty() {
                            generic_name = Some(gn.trim().to_string());
                            break;
                        }
                }
                if generic_name.is_none() {
                    generic_name = fields.get("GenericName").map(|s| s.trim().to_string());
                }

                // Extract Comment
                let mut comment: Option<String> = None;
                for loc in &locales {
                    let key = format!("Comment[{}]", loc);
                    if let Some(c) = fields.get(&key)
                        && !c.trim().is_empty() {
                            comment = Some(c.trim().to_string());
                            break;
                        }
                }
                if comment.is_none() {
                    comment = fields.get("Comment").map(|s| s.trim().to_string());
                }

                // Extract Terminal flag
                let terminal = fields.get("Terminal").map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(false);

                // Extract Icon (standardized to 48px canvas size for radial menu slots)
                let icon_name = fields.get("Icon").cloned().unwrap_or_default();
                let mut icon_path = resolve_icon(&icon_name, 48);
                if icon_path.is_none()
                    && let Some(old_icon) = fields.get("X-AppImage-Old-Icon") {
                        icon_path = resolve_icon(old_icon, 48);
                    }

                // Parse Exec and build launch command
                let (clean_exec, exec_tokens) = parse_exec_line_full(
                    exec_raw,
                    Some(&name),
                    Some(&icon_name),
                    Some(&path),
                );
                if exec_tokens.is_empty() {
                    eprintln!("[Hyprvyl Apps] Skipping {}: empty parsed exec line", path.display());
                    skipped_count += 1;
                    continue;
                }

                let launch_command = if terminal {
                    if let Some(ref term) = terminal_emulator {
                        let mut cmd = vec![term.clone(), "-e".to_string()];
                        cmd.extend(exec_tokens.clone());
                        cmd
                    } else {
                        exec_tokens.clone()
                    }
                } else {
                    exec_tokens.clone()
                };

                // Extract Categories
                let categories: Vec<String> = fields
                    .get("Categories")
                    .map(|cats| {
                        cats.split(';')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();

                // Extract Keywords
                let keywords: Vec<String> = fields
                    .get("Keywords")
                    .map(|kws| {
                        kws.split(';')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();

                // Extract Path (working directory)
                let working_dir = fields.get("Path").and_then(|p| {
                    let trimmed = p.trim();
                    if !trimmed.is_empty() {
                        Some(PathBuf::from(trimmed))
                    } else {
                        None
                    }
                });

                let app_entry = AppEntry {
                    id: file_name.clone(),
                    name,
                    generic_name,
                    comment,
                    exec: clean_exec,
                    exec_args: exec_tokens,
                    terminal,
                    launch_command,
                    icon_name,
                    icon_path,
                    categories,
                    keywords,
                    working_dir,
                    desktop_file: path,
                };

                // Insert into map: higher-priority directories overwrite lower ones
                apps_map.insert(file_name, app_entry);
            }
        }
    }

    // Convert map to a alphabetically sorted list by application Name
    let mut app_list: Vec<AppEntry> = apps_map.into_values().collect();
    app_list.sort_by_key(|a| a.name.to_lowercase());

    let elapsed = start_time.elapsed();
    println!(
        "[Hyprvyl Apps] Discovered {} unique apps (scanned {} files, skipped {}) in {:.2}ms",
        app_list.len(),
        total_files_scanned,
        skipped_count,
        elapsed.as_secs_f64() * 1000.0
    );

    app_list
}

/// Spawns an application by its AppEntry.
///
/// Reliability guarantees:
/// 1. Working Directory: Respects .desktop `Path=` or defaults to user's `$HOME`.
/// 2. Environment Variables: Inherits complete environment (WAYLAND_DISPLAY, DISPLAY, XDG_*, PATH).
/// 3. Process Detachment: Uses `setsid` in `pre_exec` to create an independent session,
///    ensuring child processes survive Hyprvyl restarts/exits and never receive Hyprvyl signals.
/// 4. Standard I/O Detachment: Redirects stdin/stdout/stderr to /dev/null so daemon doesn't leak file descriptors.
/// 5. Terminal Apps: Wrapped in auto-detected terminal emulator (e.g. `kitty -e <cmd>`).
/// 6. Error Reporting & On-Screen Notification: Dispatches an on-screen toast if execution fails.
pub fn launch_app(app: &AppEntry) -> std::io::Result<()> {
    if app.launch_command.is_empty() {
        let err_msg = "Empty launch command".to_string();
        notify_launch_error(&app.name, &err_msg);
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            err_msg,
        ));
    }

    let program = &app.launch_command[0];
    let args = &app.launch_command[1..];

    // Determine working directory
    let working_dir = app
        .working_dir
        .as_ref()
        .filter(|p| p.is_dir())
        .cloned()
        .or_else(|| env::var("HOME").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/"));

    println!(
        "[Hyprvyl Launch] Launching '{}' (ID: {}) | Exec: {:?} | Cwd: {:?}",
        app.name, app.id, app.launch_command, working_dir
    );

    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    // 1. Try launching inside an isolated transient systemd user scope (cgroup isolation).
    // This ensures heavy processes (e.g. Firefox) do not pollute hyprvyl.service cgroup statistics.
    if is_systemd_user_available() {
        let unique_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() % 1_000_000)
            .unwrap_or(0);
        let unit_name = format!("app-hyprvyl-{}-{}", sanitize_unit_name(&app.id), unique_suffix);

        let mut sys_cmd = Command::new("systemd-run");
        sys_cmd.arg("--user")
            .arg("--scope")
            .arg("--slice=app.slice")
            .arg(format!("--unit={}", unit_name))
            .arg("--")
            .arg(program)
            .args(args)
            .current_dir(&working_dir)
            .envs(env::vars())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if sys_cmd.spawn().is_ok() {
            println!(
                "[Hyprvyl Launch] Successfully spawned '{}' in transient systemd scope '{}'.",
                app.name, unit_name
            );
            return Ok(());
        }
    }

    // 2. Direct detached fallback (for non-systemd environments or if systemd-run fails)
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(&working_dir)
        .envs(env::vars())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // Detach from parent session (setsid) so process is independent
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    match cmd.spawn() {
        Ok(_) => {
            println!("[Hyprvyl Launch] Successfully spawned '{}' (detached).", app.name);
            Ok(())
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            eprintln!(
                "[Hyprvyl Launch Error] Failed to launch application '{}' (ID: {}): {}",
                app.name, app.id, err_msg
            );
            notify_launch_error(&app.name, &err_msg);
            Err(e)
        }
    }
}

/// Dispatches an on-screen notification using Hyprland's native notification system
/// or notify-send when an application fails to launch.
pub fn notify_launch_error(app_name: &str, error_msg: &str) {
    let msg = format!("Failed to launch {}: {}", app_name, error_msg);
    // Non-blocking notification dispatch
    let _ = Command::new("hyprctl")
        .args(["notify", "0", "4000", "rgb(ff5555)", &format!("Hyprvyl Error: {}", msg)])
        .spawn();
}

/// Checks if systemd user manager is available for cgroup scope isolation.
fn is_systemd_user_available() -> bool {
    (env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() || env::var_os("XDG_RUNTIME_DIR").is_some())
        && is_command_available("systemd-run")
}

/// Sanitizes a desktop identifier into a valid systemd unit name.
fn sanitize_unit_name(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    if clean.is_empty() {
        "app".to_string()
    } else {
        clean
    }
}

/// Launches a target URL using the system's default handler (via xdg-open).
pub fn launch_url(url: &str) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    println!("[Hyprvyl Launch] Opening URL: {}", url);

    if is_systemd_user_available() {
        let unique_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() % 1_000_000)
            .unwrap_or(0);
        let unit_name = format!("app-hyprvyl-url-{}", unique_suffix);

        let mut sys_cmd = Command::new("systemd-run");
        sys_cmd.arg("--user")
            .arg("--scope")
            .arg("--slice=app.slice")
            .arg(format!("--unit={}", unit_name))
            .arg("--")
            .arg("xdg-open")
            .arg(url)
            .envs(env::vars())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if sys_cmd.spawn().is_ok() {
            println!("[Hyprvyl Launch] Successfully opened URL '{}' in systemd scope '{}'.", url, unit_name);
            return Ok(());
        }
    }

    let mut cmd = Command::new("xdg-open");
    cmd.arg(url)
        .envs(env::vars())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    match cmd.spawn() {
        Ok(_) => {
            println!("[Hyprvyl Launch] Successfully opened URL '{}'", url);
            Ok(())
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            eprintln!("[Hyprvyl Launch Error] Failed to open URL '{}': {}", url, err_msg);
            notify_launch_error(url, &err_msg);
            Err(e)
        }
    }
}

/// Auto-detects the best available graphical file manager on the system.
pub fn detect_file_manager() -> Option<String> {
    let candidates = [
        "dolphin",
        "nautilus",
        "thunar",
        "nemo",
        "pcmanfm",
        "pcmanfm-qt",
        "io.elementary.files",
        "caja",
        "peony",
    ];
    for candidate in candidates {
        if is_command_available(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Expands leading `~` or `~/` to the user's home directory.
fn expand_home_path(path: &str) -> String {
    let trimmed = path.trim();
    if let Some(stripped) = trimmed.strip_prefix("~/") {
        if let Ok(home) = env::var("HOME") {
            format!("{}/{}", home, stripped)
        } else {
            trimmed.to_string()
        }
    } else if trimmed == "~" {
        env::var("HOME").unwrap_or_else(|_| trimmed.to_string())
    } else {
        trimmed.to_string()
    }
}

/// Launches a target directory or folder using the specified file manager override,
/// or falling back to an auto-detected file manager (e.g. dolphin, thunar) or xdg-open.
pub fn launch_folder(path: &str, file_manager_override: Option<&str>) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    let expanded_path = expand_home_path(path);
    let override_clean = file_manager_override.map(|s| s.trim()).filter(|s| !s.is_empty());
    println!("[Hyprvyl Launch] Opening folder: {} (Override: {:?})", expanded_path, override_clean);

    let (program, extra_args): (String, Vec<String>) = if let Some(fm) = override_clean {
        let parts: Vec<&str> = fm.split_whitespace().collect();
        let prog = parts.first().copied().unwrap_or("xdg-open").to_string();
        let args = parts.iter().skip(1).map(|s| s.to_string()).collect();
        (prog, args)
    } else if let Some(detected_fm) = detect_file_manager() {
        println!("[Hyprvyl Launch] Using auto-detected file manager: '{}'", detected_fm);
        (detected_fm, Vec::new())
    } else {
        ("xdg-open".to_string(), Vec::new())
    };

    if is_systemd_user_available() {
        let unique_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() % 1_000_000)
            .unwrap_or(0);
        let unit_name = format!("app-hyprvyl-folder-{}", unique_suffix);

        let mut sys_cmd = Command::new("systemd-run");
        sys_cmd.arg("--user")
            .arg("--scope")
            .arg("--slice=app.slice")
            .arg(format!("--unit={}", unit_name))
            .arg("--")
            .arg(&program);
        for arg in &extra_args {
            sys_cmd.arg(arg);
        }
        sys_cmd.arg(&expanded_path)
            .envs(env::vars())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Ok(mut child) = sys_cmd.spawn() {
            match child.try_wait() {
                Ok(Some(status)) if !status.success() => {
                    eprintln!("[Hyprvyl Launch] systemd-run folder launch exited with status: {:?}. Trying direct spawn.", status);
                }
                _ => {
                    println!("[Hyprvyl Launch] Successfully opened folder '{}' in systemd scope '{}'.", expanded_path, unit_name);
                    return Ok(());
                }
            }
        }
    }

    let mut cmd = Command::new(&program);
    for arg in &extra_args {
        cmd.arg(arg);
    }
    cmd.arg(&expanded_path)
        .envs(env::vars())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    match cmd.spawn() {
        Ok(_) => {
            println!("[Hyprvyl Launch] Successfully opened folder '{}'", expanded_path);
            Ok(())
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            eprintln!("[Hyprvyl Launch Error] Failed to open folder '{}': {}", expanded_path, err_msg);
            notify_launch_error(&expanded_path, &err_msg);
            Err(e)
        }
    }
}

/// Launches a target file using the system's default MIME association (via xdg-open).
pub fn launch_file(path: &str) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    let expanded_path = expand_home_path(path);
    println!("[Hyprvyl Launch] Opening file: {}", expanded_path);

    if is_systemd_user_available() {
        let unique_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() % 1_000_000)
            .unwrap_or(0);
        let unit_name = format!("app-hyprvyl-file-{}", unique_suffix);

        let mut sys_cmd = Command::new("systemd-run");
        sys_cmd.arg("--user")
            .arg("--scope")
            .arg("--slice=app.slice")
            .arg(format!("--unit={}", unit_name))
            .arg("--")
            .arg("xdg-open")
            .arg(&expanded_path)
            .envs(env::vars())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Ok(mut child) = sys_cmd.spawn() {
            match child.try_wait() {
                Ok(Some(status)) if !status.success() => {
                    eprintln!("[Hyprvyl Launch] systemd-run file launch exited with status: {:?}. Trying direct spawn.", status);
                }
                _ => {
                    println!("[Hyprvyl Launch] Successfully opened file '{}' in systemd scope '{}'.", expanded_path, unit_name);
                    return Ok(());
                }
            }
        }
    }

    let mut cmd = Command::new("xdg-open");
    cmd.arg(&expanded_path)
        .envs(env::vars())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    match cmd.spawn() {
        Ok(_) => {
            println!("[Hyprvyl Launch] Successfully opened file '{}'", expanded_path);
            Ok(())
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            eprintln!("[Hyprvyl Launch Error] Failed to open file '{}': {}", expanded_path, err_msg);
            notify_launch_error(&expanded_path, &err_msg);
            Err(e)
        }
    }
}

/// Launches an executable binary directly at the specified absolute or relative path.
pub fn launch_binary(path: &str) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    let expanded = expand_home_path(path);
    println!("[Hyprvyl Launch] Launching executable binary: {}", expanded);

    if expanded.is_empty() {
        let err_msg = "Empty binary path".to_string();
        notify_launch_error(path, &err_msg);
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, err_msg));
    }

    // If the path directly exists as a file (even with spaces!), use it without splitting
    let (program, args): (String, Vec<String>) = if Path::new(&expanded).is_file() {
        (expanded.clone(), Vec::new())
    } else {
        let (_clean_str, tokens) = parse_exec_line_full(&expanded, None, None, None);
        if tokens.is_empty() {
            let err_msg = "Empty binary path".to_string();
            notify_launch_error(&expanded, &err_msg);
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, err_msg));
        }
        let prog = tokens[0].clone();
        let rest = tokens[1..].to_vec();
        (prog, rest)
    };

    let prog_path = Path::new(&program);
    let resolved_program = if prog_path.is_file() {
        program.clone()
    } else if (program.starts_with("./") || program.starts_with("../"))
        && let Ok(current_dir) = env::current_dir()
    {
        current_dir.join(&program).to_string_lossy().to_string()
    } else {
        program.clone()
    };

    // Ensure executable permissions (+x) if it is a local file
    let p_check = Path::new(&resolved_program);
    if p_check.is_file()
        && let Ok(metadata) = p_check.metadata()
    {
        let mode = metadata.permissions().mode();
        if mode & 0o111 == 0 {
            let mut perms = metadata.permissions();
            perms.set_mode(mode | 0o755);
            let _ = fs::set_permissions(p_check, perms);
        }
    }

    // Determine working directory
    let working_dir = if let Some(parent) = p_check.parent().filter(|p| p.is_dir()) {
        parent.to_path_buf()
    } else {
        env::var("HOME").ok().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
    };

    if is_systemd_user_available() {
        let unique_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() % 1_000_000)
            .unwrap_or(0);
        let clean_name = p_check.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "bin".to_string());
        let unit_name = format!("app-hyprvyl-bin-{}-{}", sanitize_unit_name(&clean_name), unique_suffix);

        let mut sys_cmd = Command::new("systemd-run");
        sys_cmd.arg("--user")
            .arg("--scope")
            .arg("--slice=app.slice")
            .arg(format!("--unit={}", unit_name))
            .arg("--")
            .arg(&resolved_program);
        for arg in &args {
            sys_cmd.arg(arg);
        }
        sys_cmd.current_dir(&working_dir)
            .envs(env::vars())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Ok(mut child) = sys_cmd.spawn() {
            match child.try_wait() {
                Ok(Some(status)) if !status.success() => {
                    eprintln!("[Hyprvyl Launch] systemd-run binary launch failed with status: {:?}. Falling back to direct spawn.", status);
                }
                _ => {
                    println!("[Hyprvyl Launch] Successfully spawned binary '{}' in systemd scope '{}'.", resolved_program, unit_name);
                    return Ok(());
                }
            }
        }
    }

    let mut cmd = Command::new(&resolved_program);
    for arg in &args {
        cmd.arg(arg);
    }
    cmd.current_dir(&working_dir)
        .envs(env::vars())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    match cmd.spawn() {
        Ok(_) => {
            println!("[Hyprvyl Launch] Successfully spawned binary '{}'", resolved_program);
            Ok(())
        }
        Err(e) => {
            // Fallback for non-ELF scripts: execute via /bin/sh -c
            if p_check.is_file() {
                let mut sh_cmd = Command::new("/bin/sh");
                sh_cmd.arg(&resolved_program);
                for arg in &args {
                    sh_cmd.arg(arg);
                }
                sh_cmd.current_dir(&working_dir)
                    .envs(env::vars())
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());

                unsafe {
                    sh_cmd.pre_exec(|| {
                        libc::setsid();
                        Ok(())
                    });
                }

                if sh_cmd.spawn().is_ok() {
                    println!("[Hyprvyl Launch] Successfully spawned binary '{}' via /bin/sh fallback", resolved_program);
                    return Ok(());
                }
            }

            let err_msg = format!("{}", e);
            eprintln!("[Hyprvyl Launch Error] Failed to execute binary '{}': {}", resolved_program, err_msg);
            notify_launch_error(&resolved_program, &err_msg);
            Err(e)
        }
    }
}

/// Dispatches launch for any workspace item kind ("app", "url", "folder", "file", "binary").
pub fn launch_workspace_item(
    kind: &str,
    target: &str,
    file_manager_override: Option<&str>,
    apps: &[AppEntry],
) -> std::io::Result<()> {
    let expanded = expand_home_path(target);
    match kind.to_lowercase().as_str() {
        "app" => {
            let target_lower = target.to_lowercase();
            let matched = apps.iter().find(|a| {
                a.id.to_lowercase() == target_lower
                    || a.id.to_lowercase().strip_suffix(".desktop") == Some(&target_lower)
                    || a.name.to_lowercase() == target_lower
            });

            if let Some(app) = matched {
                launch_app(app)
            } else if expanded.starts_with('/') || expanded.starts_with("./") {
                if Path::new(&expanded).is_dir() {
                    launch_folder(&expanded, file_manager_override)
                } else if Path::new(&expanded).is_file() {
                    launch_binary(&expanded).or_else(|_| launch_file(&expanded))
                } else {
                    launch_binary(&expanded)
                }
            } else if expanded.starts_with("http://") || expanded.starts_with("https://") {
                launch_url(&expanded)
            } else {
                launch_binary(&expanded)
            }
        }
        "url" => launch_url(&expanded),
        "folder" => launch_folder(&expanded, file_manager_override),
        "file" => launch_file(&expanded),
        "binary" | "bin" | "executable" => launch_binary(&expanded),
        _ => launch_url(&expanded),
    }
}

/// Resolves an icon path and icon name for any item kind ("app", "url", "folder", "file", "binary", "workspace").
pub fn resolve_kind_icon(
    kind: &str,
    target: &str,
    custom_icon: Option<&str>,
    apps: &[AppEntry],
    target_size: i32,
    auto_fetch_favicons: bool,
) -> (Option<PathBuf>, String) {
    if let Some(custom) = custom_icon
        && !custom.trim().is_empty() {
            let custom_clean = custom.trim();
            // Check bundled icons first
            if crate::icons::get_bundled_icon(custom_clean).is_some() {
                return (None, custom_clean.to_string());
            }
            let p = resolve_icon(custom_clean, target_size);
            return (p, custom_clean.to_string());
        }

    match kind.to_lowercase().as_str() {
        "app" => {
            let target_lower = target.to_lowercase();
            let target_no_desktop = target_lower.strip_suffix(".desktop").unwrap_or(&target_lower);
            if let Some(app) = apps.iter().find(|a| {
                a.id.to_lowercase() == target_lower
                    || a.id.to_lowercase().strip_suffix(".desktop") == Some(target_no_desktop)
                    || a.name.to_lowercase() == target_lower
                    || a.name.to_lowercase() == target_no_desktop
                    || a.exec.to_lowercase() == target_lower
                    || a.exec.to_lowercase() == target_no_desktop
            }) {
                if let Some(ref p) = app.icon_path {
                    (Some(p.clone()), app.icon_name.clone())
                } else {
                    let p = resolve_icon(&app.icon_name, target_size)
                        .or_else(|| resolve_icon(target_no_desktop, target_size))
                        .or_else(|| find_fallback_icon(target_size));
                    (p, app.icon_name.clone())
                }
            } else {
                let p = resolve_icon(target_no_desktop, target_size)
                    .or_else(|| resolve_icon(target, target_size))
                    .or_else(|| find_fallback_icon(target_size));
                (p, target.to_string())
            }
        }
        "url" => {
            // Check if favicon exists or should be auto-fetched
            if let Some(favicon_path) = crate::favicons::resolve_or_fetch_favicon(target, auto_fetch_favicons) {
                return (Some(favicon_path), "globe".to_string());
            }

            let target_lower = target.to_lowercase();
            let mut custom_candidates = Vec::new();
            if target_lower.contains("whatsapp") {
                custom_candidates.extend(["whatsapp", "web-whatsapp", "whatsapp-desktop", "browser"]);
            } else if target_lower.contains("github") {
                custom_candidates.extend(["github", "git", "applications-development"]);
            } else if target_lower.contains("youtube") {
                custom_candidates.extend(["youtube", "video-player", "applications-multimedia"]);
            } else if target_lower.contains("discord") {
                custom_candidates.extend(["discord", "discord-canary", "chat"]);
            } else if target_lower.contains("spotify") {
                custom_candidates.extend(["spotify", "audio-player", "multimedia"]);
            } else if target_lower.contains("anthropic") || target_lower.contains("claude") {
                custom_candidates.extend(["claude", "chat", "globe"]);
            }

            for name in custom_candidates {
                if crate::icons::get_bundled_icon(name).is_some() {
                    return (None, name.to_string());
                }
                if let Some(p) = resolve_icon(name, target_size) {
                    return (Some(p), name.to_string());
                }
            }

            let candidates = ["globe", "browser", "link", "applications-internet", "internet-web-browser"];
            for name in candidates {
                if crate::icons::get_bundled_icon(name).is_some() {
                    return (None, name.to_string());
                }
                if let Some(p) = resolve_icon(name, target_size) {
                    return (Some(p), name.to_string());
                }
            }
            (find_fallback_icon(target_size), "globe".to_string())
        }
        "folder" => {
            let candidates = ["folder", "folder-open", "inode-directory", "system-file-manager", "user-home"];
            for name in candidates {
                if crate::icons::get_bundled_icon(name).is_some() {
                    return (None, name.to_string());
                }
                if let Some(p) = resolve_icon(name, target_size) {
                    return (Some(p), name.to_string());
                }
            }
            (find_fallback_icon(target_size), "folder".to_string())
        }
        "file" => {
            let candidates = ["file-text", "file", "text-x-generic", "document", "accessories-text-editor"];
            for name in candidates {
                if crate::icons::get_bundled_icon(name).is_some() {
                    return (None, name.to_string());
                }
                if let Some(p) = resolve_icon(name, target_size) {
                    return (Some(p), name.to_string());
                }
            }
            (find_fallback_icon(target_size), "file-text".to_string())
        }
        "binary" | "bin" | "executable" => {
            let candidates = ["terminal", "binary", "application-x-executable", "system-run", "utilities-terminal"];
            for name in candidates {
                if crate::icons::get_bundled_icon(name).is_some() {
                    return (None, name.to_string());
                }
                if let Some(p) = resolve_icon(name, target_size) {
                    return (Some(p), name.to_string());
                }
            }
            (find_fallback_icon(target_size), "terminal".to_string())
        }
        "workspace" => {
            let icon_name = if target.trim().is_empty() { "folder" } else { target.trim() };
            if crate::icons::get_bundled_icon(icon_name).is_some() {
                return (None, icon_name.to_string());
            }
            let p = resolve_icon(icon_name, target_size)
                .or_else(|| resolve_icon("folder", target_size))
                .or_else(|| find_fallback_icon(target_size));
            (p, icon_name.to_string())
        }
        _ => {
            if crate::icons::get_bundled_icon(target).is_some() {
                return (None, target.to_string());
            }
            let p = resolve_icon(target, target_size).or_else(|| find_fallback_icon(target_size));
            (p, target.to_string())
        }
    }
}

/// Starts a background inotify file watcher for all XDG application directories.
///
/// Features:
/// - Monitors creation, modification, and deletion of `.desktop` files.
/// - 750ms quiet-window debouncing to coalesce rapid successive package manager writes.
/// - Signals the GTK main loop via an async channel to rescan and update the in-memory registry safely.
/// - Gracefully handles missing directories and inotify limits.
pub fn start_desktop_file_watcher(invalidation_sender: async_channel::Sender<()>) {
    let search_dirs = get_application_search_dirs();

    thread::Builder::new()
        .name("hyprvyl-app-watcher".to_string())
        .spawn(move || {
            let (tx, rx) = mpsc::channel();

            let mut watcher = match RecommendedWatcher::new(
                move |res: Result<Event, notify::Error>| {
                    if let Ok(event) = res {
                        let _ = tx.send(event);
                    }
                },
                NotifyConfig::default(),
            ) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!(
                        "[Hyprvyl Watcher] Warning: Failed to initialize inotify watcher: {}. Continuing with startup cache only.",
                        e
                    );
                    return;
                }
            };

            let mut watched_count = 0;
            for dir in &search_dirs {
                if dir.is_dir() {
                    match watcher.watch(dir, RecursiveMode::NonRecursive) {
                        Ok(_) => {
                            watched_count += 1;
                        }
                        Err(e) => {
                            eprintln!(
                                "[Hyprvyl Watcher] Notice: Could not watch directory {} ({}). Skipping.",
                                dir.display(),
                                e
                            );
                        }
                    }
                }
            }

            println!(
                "[Hyprvyl Watcher] Actively watching {} application directories for live changes.",
                watched_count
            );

            if watched_count == 0 {
                eprintln!("[Hyprvyl Watcher] No active application directories found. Live invalidation disabled.");
                return;
            }

            let debounce_duration = Duration::from_millis(750);

            while let Ok(first_event) = rx.recv() {
                if !is_relevant_desktop_event(&first_event) {
                    continue;
                }

                // Debounce window: drain rapid subsequent events
                let mut deadline = Instant::now() + debounce_duration;
                loop {
                    let now = Instant::now();
                    if now >= deadline {
                        break;
                    }
                    let remaining = deadline - now;
                    match rx.recv_timeout(remaining) {
                        Ok(subsequent_event) => {
                            if is_relevant_desktop_event(&subsequent_event) {
                                deadline = Instant::now() + debounce_duration;
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            return;
                        }
                    }
                }

                println!("[Hyprvyl Watcher] Changes detected in application directories. Triggering rescan...");
                if let Err(e) = invalidation_sender.send_blocking(()) {
                    eprintln!("[Hyprvyl Watcher] Failed to signal rescan: {}", e);
                    break;
                }
            }
        })
        .expect("Failed to spawn hyprvyl-app-watcher thread");
}

/// Evaluates if a filesystem event affects .desktop entries or watched application directories.
pub fn is_relevant_desktop_event(event: &Event) -> bool {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any => {
            if event.paths.is_empty() {
                return true;
            }
            event.paths.iter().any(|p| {
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    ext.eq_ignore_ascii_case("desktop")
                } else {
                    p.is_dir()
                }
            })
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exec_line() {
        // Standard executable
        let (clean, tokens) = parse_exec_line("firefox");
        assert_eq!(clean, "firefox");
        assert_eq!(tokens, vec!["firefox"]);

        // Executable with single file field code %u
        let (clean, tokens) = parse_exec_line("spotify --uri=%u");
        assert_eq!(clean, "spotify");
        assert_eq!(tokens, vec!["spotify"]);

        // Executable with multiple field codes
        let (clean, tokens) = parse_exec_line("micro %F");
        assert_eq!(clean, "micro");
        assert_eq!(tokens, vec!["micro"]);

        // Quoted arguments
        let (clean, tokens) = parse_exec_line("env FOO=\"bar baz\" myapp %U");
        assert_eq!(clean, "env FOO=bar baz myapp");
        assert_eq!(tokens, vec!["env", "FOO=bar baz", "myapp"]);
    }

    #[test]
    fn test_get_application_search_dirs() {
        let dirs = get_application_search_dirs();
        assert!(!dirs.is_empty());
        // System and user directories should be present
        assert!(dirs.iter().any(|d| d.ends_with("applications")));
    }

    #[test]
    fn test_terminal_detection() {
        let term = detect_terminal_emulator();
        // On our Linux environment, kitty or foot or xterm is present
        assert!(term.is_some());
    }

    #[test]
    fn test_field_code_expansions() {
        let (clean, tokens) = parse_exec_line_full(
            "mybrowser --url=%u --name=%c %i %k",
            Some("My Browser"),
            Some("my-icon"),
            Some(Path::new("/usr/share/applications/mybrowser.desktop")),
        );
        assert_eq!(clean, "mybrowser --name=My Browser --icon my-icon /usr/share/applications/mybrowser.desktop");
        assert_eq!(
            tokens,
            vec![
                "mybrowser",
                "--name=My Browser",
                "--icon",
                "my-icon",
                "/usr/share/applications/mybrowser.desktop"
            ]
        );

        // Percent literal escape
        let (clean, tokens) = parse_exec_line_full("app --ratio=100%%", None, None, None);
        assert_eq!(clean, "app --ratio=100%");
        assert_eq!(tokens, vec!["app", "--ratio=100%"]);
    }

    #[test]
    fn test_application_discovery_and_launch_readiness() {
        let _ = gtk4::init();
        let apps = discover_applications();
        assert_eq!(apps.len(), 74, "Expected exactly 74 applications discovered");

        // 1. Verify terminal-wrapped applications (Neovim, Vim, Micro, jshell)
        let term_apps: Vec<&AppEntry> = apps.iter().filter(|a| a.terminal).collect();
        assert!(!term_apps.is_empty(), "Expected terminal apps to be discovered");

        let terminal = detect_terminal_emulator().expect("Terminal emulator must be detected");
        for app in term_apps {
            assert_eq!(app.launch_command[0], terminal);
            assert_eq!(app.launch_command[1], "-e");
            assert!(app.launch_command.len() >= 3);
            let target_bin = &app.launch_command[2];
            assert!(
                is_command_available(target_bin),
                "Terminal app target binary '{}' must be available",
                target_bin
            );
        }

        // 2. Verify all 74 applications have valid executable binaries & clean arguments
        for app in &apps {
            assert!(!app.launch_command.is_empty(), "Launch command for '{}' is empty", app.name);
            let program = &app.launch_command[0];
            assert!(
                is_command_available(program),
                "Program '{}' for app '{}' ({}) not found in PATH",
                program,
                app.name,
                app.id
            );
            // Verify no leftover raw field codes
            for arg in &app.launch_command {
                assert_ne!(arg, "%f");
                assert_ne!(arg, "%F");
                assert_ne!(arg, "%u");
                assert_ne!(arg, "%U");
                assert_ne!(arg, "%d");
                assert_ne!(arg, "%n");
            }
        }
    }

    #[test]
    fn test_launch_dispatch_routing() {
        let _ = gtk4::init();
        let apps = discover_applications();

        // 1. URL resolution
        let (p_url, name_url) = resolve_kind_icon("url", "https://web.whatsapp.com", None, &apps, 36, false);
        assert!(!name_url.is_empty());
        assert!(p_url.is_some() || crate::icons::get_bundled_icon(&name_url).is_some());

        // 2. Folder resolution
        let (p_folder, name_folder) = resolve_kind_icon("folder", "/home/dev/Projects", None, &apps, 36, false);
        assert!(!name_folder.is_empty());
        assert!(p_folder.is_some() || crate::icons::get_bundled_icon(&name_folder).is_some());

        // 3. File resolution
        let (p_file, name_file) = resolve_kind_icon("file", "/home/dev/notes.txt", None, &apps, 36, false);
        assert!(!name_file.is_empty());
        assert!(p_file.is_some() || crate::icons::get_bundled_icon(&name_file).is_some());

        // 4. Binary resolution
        let (p_bin, name_bin) = resolve_kind_icon("binary", "/usr/bin/git", None, &apps, 36, false);
        assert!(!name_bin.is_empty());
        assert!(p_bin.is_some() || crate::icons::get_bundled_icon(&name_bin).is_some());

        // 5. Workspace icon resolution
        let (p_ws, name_ws) = resolve_kind_icon("workspace", "folder-code", None, &apps, 36, false);
        assert_eq!(name_ws, "folder-code");
        assert!(p_ws.is_none()); // bundled icons return None for path, resolved directly from SVG
        assert!(crate::icons::get_bundled_icon(&name_ws).is_some());
    }

    #[test]
    fn test_workspace_item_icon_isolation_regression() {
        let apps = discover_applications();
        let ws_icon_x = "folder-code";

        // Item A: in workspace X, icon explicitly set to Y ("compass")
        let icon_y = "compass";
        let (_p_a, name_a) = resolve_kind_icon("url", "https://example.com/site-a", Some(icon_y), &apps, 36, false);

        // Item B: in workspace X, no icon set (None)
        let (_p_b, name_b) = resolve_kind_icon("url", "https://example.com/site-b", None, &apps, 36, false);

        // Item C (Folder): in workspace X, no icon set (None)
        let (_p_c, name_c) = resolve_kind_icon("folder", "/home/dev/some/dir", None, &apps, 36, false);

        // Item D (File): in workspace X, no icon set (None)
        let (_p_d, name_d) = resolve_kind_icon("file", "/home/dev/file.txt", None, &apps, 36, false);

        // Item E (Binary): in workspace X, no icon set (None)
        let (_p_e, name_e) = resolve_kind_icon("binary", "/usr/bin/tool", None, &apps, 36, false);

        // 1. Assert Item A resolves to Y ("compass")
        assert_eq!(name_a, icon_y);

        // 2. Assert Item B resolves to generic URL placeholder ("globe")
        assert_eq!(name_b, "globe");

        // 3. Assert Item C resolves to generic folder placeholder ("folder")
        assert_eq!(name_c, "folder");

        // 4. Assert Item D resolves to generic file placeholder ("file-text")
        assert_eq!(name_d, "file-text");

        // 5. Assert Item E resolves to generic binary placeholder ("terminal")
        assert_eq!(name_e, "terminal");

        // 6. Assert NONE of the items resolve to X ("folder-code", the parent workspace's icon)
        assert_ne!(name_a, ws_icon_x, "Item A must not inherit parent workspace icon");
        assert_ne!(name_b, ws_icon_x, "Item B must not inherit parent workspace icon");
        assert_ne!(name_c, ws_icon_x, "Item C must not inherit parent workspace icon");
        assert_ne!(name_d, ws_icon_x, "Item D must not inherit parent workspace icon");
        assert_ne!(name_e, ws_icon_x, "Item E must not inherit parent workspace icon");
    }
}
