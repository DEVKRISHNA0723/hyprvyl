use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TriggerMode {
    #[default]
    Hold,
    Click,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SpawnPosition {
    Cursor,
    #[default]
    Center,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Whether to start Hyprvyl automatically with Hyprland
    #[serde(default)]
    pub start_with_hyprland: bool,

    /// Whether to create/maintain an XDG autostart entry
    #[serde(default)]
    pub autostart: bool,

    /// Initial spawn position of the radial wheel: "center" (default) or "cursor"
    #[serde(default)]
    pub spawn_position: SpawnPosition,

    /// Whether to close the overlay after launching an application
    #[serde(default = "default_true")]
    pub close_on_launch: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            start_with_hyprland: false,
            autostart: false,
            spawn_position: SpawnPosition::Center,
            close_on_launch: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    /// Trigger mode: "hold" or "click"
    #[serde(default)]
    pub mode: TriggerMode,

    /// Duration in milliseconds to hold before triggering overlay (for "hold" mode)
    #[serde(default = "default_hold_duration_ms")]
    pub hold_duration_ms: u64,

    /// Target button or key name (e.g. "BTN_EXTRA", "BTN_SIDE", "BTN_MIDDLE", "KEY_GRAVE")
    /// or raw evdev numeric code as string or number.
    #[serde(default = "default_button")]
    pub button: String,

    /// Optional explicit device path (e.g. "/dev/input/event5").
    /// If empty, Hyprvyl auto-detects suitable input devices.
    #[serde(default)]
    pub device: String,
}

fn default_hold_duration_ms() -> u64 {
    200
}

fn default_button() -> String {
    "BTN_EXTRA".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            mode: TriggerMode::Hold,
            hold_duration_ms: 200,
            button: "BTN_EXTRA".to_string(),
            device: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum WheelLayout {
    #[default]
    Wheel,
    Arc,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    /// Wheel layout style: "wheel" (full 360° circular wheel like Rovyl) or "arc" (radial arc)
    #[serde(default)]
    pub layout: WheelLayout,

    /// Width and height of each floating button (in pixels)
    #[serde(default = "default_button_size")]
    pub button_size: f64,

    /// Corner radius of the rounded square buttons (in pixels)
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,

    /// Distance from cursor center to buttons along the wheel/arc (in pixels)
    #[serde(default = "default_arc_radius")]
    pub arc_radius: f64,

    /// Angular span of the radial arc in degrees (default: 140.0, used in arc layout)
    #[serde(default = "default_arc_span_degrees")]
    pub arc_span_degrees: f64,

    /// Number of items displayed per page along the arc/wheel
    #[serde(default = "default_items_per_page")]
    pub items_per_page: usize,

    /// Size of the application icon inside the button (in pixels)
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,

    /// Background opacity for the floating buttons (0.0 - 1.0)
    #[serde(default = "default_opacity")]
    pub opacity: f64,

    /// Color theme variant ("dark", "oled")
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_button_size() -> f64 {
    56.0
}

fn default_corner_radius() -> f64 {
    14.0
}

fn default_arc_radius() -> f64 {
    140.0
}

fn default_arc_span_degrees() -> f64 {
    140.0
}

fn default_items_per_page() -> usize {
    8
}

fn default_icon_size() -> i32 {
    36
}

fn default_opacity() -> f64 {
    0.90
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            layout: WheelLayout::Wheel,
            button_size: default_button_size(),
            corner_radius: default_corner_radius(),
            arc_radius: default_arc_radius(),
            arc_span_degrees: default_arc_span_degrees(),
            items_per_page: default_items_per_page(),
            icon_size: default_icon_size(),
            opacity: default_opacity(),
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacesConfig {
    /// Spawn overlay on the monitor where the cursor currently resides
    #[serde(default = "default_true")]
    pub spawn_at_cursor: bool,

    /// Follow active workspace focus
    #[serde(default = "default_true")]
    pub follow_workspace: bool,

    /// Workspace switching mode: "picker" (visual wheel picker) or "keys" (number keys)
    #[serde(default = "default_workspace_switching")]
    pub workspace_switching: String,
}

fn default_workspace_switching() -> String {
    "picker".to_string()
}

impl Default for WorkspacesConfig {
    fn default() -> Self {
        Self {
            spawn_at_cursor: true,
            follow_workspace: true,
            workspace_switching: default_workspace_switching(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
#[allow(dead_code)]
pub enum ItemKind {
    #[default]
    App,
    Url,
    Folder,
    File,
    Binary,
}

#[allow(dead_code)]
impl ItemKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ItemKind::App => "app",
            ItemKind::Url => "url",
            ItemKind::Folder => "folder",
            ItemKind::File => "file",
            ItemKind::Binary => "binary",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "app" => ItemKind::App,
            "url" => ItemKind::Url,
            "folder" => ItemKind::Folder,
            "file" => ItemKind::File,
            "binary" | "bin" | "executable" => ItemKind::Binary,
            _ => ItemKind::App,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceItemConfig {
    pub name: String,
    /// "app" | "url" | "folder" | "file" | "binary"
    pub kind: String,
    /// target desktop_id (for app), URL (for url), directory (for folder), file path (for file), binary path (for binary)
    pub target: String,
    #[serde(default)]
    pub order: usize,
    #[serde(default)]
    pub icon: Option<String>,
}

#[allow(dead_code)]
impl WorkspaceItemConfig {
    pub fn item_kind(&self) -> ItemKind {
        ItemKind::from_str_loose(&self.kind)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub id: String,
    pub name: String,
    #[serde(default = "default_workspace_icon")]
    pub icon: String,
    #[serde(default)]
    pub order: usize,
    #[serde(default, rename = "item")]
    pub items: Vec<WorkspaceItemConfig>,
}

fn default_workspace_icon() -> String {
    "folder".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// Custom terminal emulator override (e.g. "kitty", "foot", "alacritty", or empty for auto-detect)
    #[serde(default)]
    pub terminal_override: String,

    /// Custom file manager override (e.g. "nautilus", "dolphin", "thunar", "pcmanfm", or empty for system default)
    #[serde(default)]
    pub file_manager_override: String,

    /// Automatically fetch and cache favicons for URL items
    #[serde(default = "default_true")]
    pub auto_fetch_favicons: bool,

    /// Enable debug logging in stdout/stderr
    #[serde(default)]
    pub debug_logging: bool,

    /// Enable live inotify watching of .desktop file directories
    #[serde(default = "default_true")]
    pub live_watcher: bool,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            terminal_override: String::new(),
            file_manager_override: String::new(),
            auto_fetch_favicons: true,
            debug_logging: false,
            live_watcher: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub trigger: TriggerConfig,

    #[serde(default)]
    pub appearance: AppearanceConfig,

    #[serde(default)]
    pub workspaces: WorkspacesConfig,

    #[serde(default)]
    pub advanced: AdvancedConfig,

    #[serde(default, rename = "workspace")]
    pub workspace: Vec<WorkspaceConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            trigger: TriggerConfig::default(),
            appearance: AppearanceConfig::default(),
            workspaces: WorkspacesConfig::default(),
            advanced: AdvancedConfig::default(),
            workspace: default_workspaces(),
        }
    }
}

pub fn default_workspaces() -> Vec<WorkspaceConfig> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/dev".to_string());
    let projects_dir = format!("{}/Projects", home);

    vec![
        WorkspaceConfig {
            id: "main".to_string(),
            name: "Main".to_string(),
            icon: "folder".to_string(),
            order: 0,
            items: vec![
                WorkspaceItemConfig {
                    name: "Chrome".to_string(),
                    kind: "app".to_string(),
                    target: "google-chrome.desktop".to_string(),
                    order: 0,
                    icon: None,
                },
                WorkspaceItemConfig {
                    name: "WhatsApp".to_string(),
                    kind: "url".to_string(),
                    target: "https://web.whatsapp.com".to_string(),
                    order: 1,
                    icon: None,
                },
                WorkspaceItemConfig {
                    name: "Notepad".to_string(),
                    kind: "app".to_string(),
                    target: "org.gnome.TextEditor.desktop".to_string(),
                    order: 2,
                    icon: None,
                },
                WorkspaceItemConfig {
                    name: "Calculator".to_string(),
                    kind: "app".to_string(),
                    target: "org.gnome.Calculator.desktop".to_string(),
                    order: 3,
                    icon: None,
                },
            ],
        },
        WorkspaceConfig {
            id: "work".to_string(),
            name: "Work".to_string(),
            icon: "folder".to_string(),
            order: 1,
            items: vec![
                WorkspaceItemConfig {
                    name: "Firefox".to_string(),
                    kind: "app".to_string(),
                    target: "firefox.desktop".to_string(),
                    order: 0,
                    icon: None,
                },
                WorkspaceItemConfig {
                    name: "Anthropic".to_string(),
                    kind: "url".to_string(),
                    target: "https://anthropic.com".to_string(),
                    order: 1,
                    icon: None,
                },
                WorkspaceItemConfig {
                    name: "Projects".to_string(),
                    kind: "folder".to_string(),
                    target: projects_dir,
                    order: 2,
                    icon: None,
                },
            ],
        },
    ]
}

impl Config {
    /// Returns the standard config path: ~/.config/hyprvyl/config.toml
    pub fn config_path() -> PathBuf {
        let base = match std::env::var("XDG_CONFIG_HOME") {
            Ok(val) if !val.is_empty() => PathBuf::from(val),
            _ => {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".config")
            }
        };
        base.join("hyprvyl").join("config.toml")
    }

    /// Loads the configuration from disk, or creates a default one if it doesn't exist.
    pub fn load_or_create() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match toml::from_str::<Config>(&content) {
                    Ok(mut cfg) => {
                        println!("[Hyprvyl Config] Loaded config from {}", path.display());
                        if cfg.workspace.is_empty() {
                            cfg.workspace = default_workspaces();
                            let _ = cfg.save();
                        }
                        return cfg;
                    }
                    Err(e) => {
                        eprintln!("[Hyprvyl Config] Error parsing {}: {}. Using defaults.", path.display(), e);
                    }
                },
                Err(e) => {
                    eprintln!("[Hyprvyl Config] Error reading {}: {}. Using defaults.", path.display(), e);
                }
            }
        } else {
            let default_cfg = Config::default();
            if let Err(e) = default_cfg.save() {
                eprintln!("[Hyprvyl Config] Note: Could not write default config to {}: {}", path.display(), e);
            } else {
                println!("[Hyprvyl Config] Created default configuration template at {}", path.display());
            }
            return default_cfg;
        }

        Config::default()
    }

    /// Saves the current configuration to disk as TOML.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&path, toml_str)
    }

    /// Synchronizes Hyprland autostart in ~/.config/hypr/hyprland.conf and installs binary to ~/.local/bin
    pub fn sync_hyprland_autostart(&self) -> Result<(), std::io::Error> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/dev".to_string());
        let hypr_dir = PathBuf::from(&home).join(".config/hypr");
        let hypr_conf = hypr_dir.join("hyprland.conf");
        let local_bin_dir = PathBuf::from(&home).join(".local/bin");
        let target_bin = local_bin_dir.join("hyprvyl");
        let target_bin_str = target_bin.to_string_lossy().to_string();

        if self.general.start_with_hyprland {
            // Ensure the hyprvyl binary is installed into ~/.local/bin (which is in PATH)
            if let Ok(curr_exe) = std::env::current_exe() {
                let _ = fs::create_dir_all(&local_bin_dir);
                let _ = fs::copy(&curr_exe, &target_bin);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&target_bin, fs::Permissions::from_mode(0o755));
                }
            }

            // Ensure ~/.config/hypr directory exists
            let _ = fs::create_dir_all(&hypr_dir);

            let autostart_cmd = format!("exec-once = {}", target_bin_str);
            if !hypr_conf.exists() {
                let content = format!("# Hyprland Configuration\n# Hyprvyl Radial Launcher Autostart\n{}\n", autostart_cmd);
                fs::write(&hypr_conf, content)?;
                println!("[Hyprvyl Config] Created {} with autostart entry", hypr_conf.display());
            } else {
                let content = fs::read_to_string(&hypr_conf)?;
                let has_exact = content.lines().any(|l| l.trim() == autostart_cmd);
                let has_generic = content.lines().any(|l| {
                    let t = l.trim();
                    t.starts_with("exec-once =") && t.contains("hyprvyl")
                });

                if !has_exact {
                    if has_generic {
                        // Upgrade any relative exec-once line to absolute path
                        let updated_lines: Vec<String> = content
                            .lines()
                            .map(|l| {
                                let t = l.trim();
                                if t.starts_with("exec-once =") && t.contains("hyprvyl") {
                                    autostart_cmd.clone()
                                } else {
                                    l.to_string()
                                }
                            })
                            .collect();
                        let new_content = updated_lines.join("\n") + "\n";
                        fs::write(&hypr_conf, new_content)?;
                        println!("[Hyprvyl Config] Updated autostart entry with absolute path in {}", hypr_conf.display());
                    } else {
                        let mut new_content = content;
                        if !new_content.ends_with('\n') {
                            new_content.push('\n');
                        }
                        new_content.push_str(&format!("\n# Hyprvyl Radial Launcher Autostart\n{}\n", autostart_cmd));
                        fs::write(&hypr_conf, new_content)?;
                        println!("[Hyprvyl Config] Added '{}' to {}", autostart_cmd, hypr_conf.display());
                    }
                }
            }

            // Also synchronize XDG desktop autostart entry for maximum reliability
            let _ = self.sync_xdg_autostart();
        } else if hypr_conf.exists() {
            let content = fs::read_to_string(&hypr_conf)?;
            let has_line = content.lines().any(|l| {
                let t = l.trim();
                t.starts_with("exec-once =") && t.contains("hyprvyl")
            });

            if has_line {
                let filtered: Vec<&str> = content
                    .lines()
                    .filter(|l| {
                        let t = l.trim();
                        !t.contains("hyprvyl") && t != "# Hyprvyl Radial Launcher Autostart" && t != "# Auto-generated by Hyprvyl Settings"
                    })
                    .collect();
                let new_content = filtered.join("\n") + "\n";
                fs::write(&hypr_conf, new_content)?;
                println!("[Hyprvyl Config] Removed autostart entry from {}", hypr_conf.display());
            }

            if !self.general.autostart {
                let _ = self.sync_xdg_autostart();
            }
        }

        Ok(())
    }

    /// Synchronizes XDG autostart in ~/.config/autostart/hyprvyl.desktop
    pub fn sync_xdg_autostart(&self) -> Result<(), std::io::Error> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/dev".to_string());
        let autostart_dir = PathBuf::from(&home).join(".config/autostart");
        let desktop_file = autostart_dir.join("hyprvyl.desktop");
        let local_bin_dir = PathBuf::from(&home).join(".local/bin");
        let target_bin = local_bin_dir.join("hyprvyl");
        let target_bin_str = target_bin.to_string_lossy().to_string();

        if self.general.autostart || self.general.start_with_hyprland {
            fs::create_dir_all(&autostart_dir)?;
            let content = format!(
                "[Desktop Entry]\nType=Application\nName=Hyprvyl\nComment=Wayland Application Launcher Daemon\nExec={}\nIcon=preferences-system\nTerminal=false\nCategories=Utility;\n",
                target_bin_str
            );
            fs::write(&desktop_file, content)?;
            println!("[Hyprvyl Config] Created autostart desktop entry at {}", desktop_file.display());
        } else if desktop_file.exists() {
            let _ = fs::remove_file(&desktop_file);
            println!("[Hyprvyl Config] Removed autostart desktop entry at {}", desktop_file.display());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_toml_roundtrip() {
        let toml_sample = r#"
[general]
start_with_hyprland = true
autostart = false
close_on_launch = true

[trigger]
mode = "hold"
hold_duration_ms = 250
button = "BTN_EXTRA"
device = ""

[appearance]
button_size = 56.0
corner_radius = 10.0
arc_radius = 160.0
arc_span_degrees = 140.0
items_per_page = 8
icon_size = 36
opacity = 0.85
theme = "dark"

[workspaces]
spawn_at_cursor = true
follow_workspace = true

[advanced]
terminal_override = "kitty"
file_manager_override = "nautilus"
auto_fetch_favicons = true
debug_logging = false
live_watcher = true

[[workspace]]
id = "main"
name = "Main"
icon = "folder-code"
order = 0

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

  [[workspace.item]]
  name = "Notes"
  kind = "file"
  target = "/home/dev/Documents/notes.txt"
  order = 3

  [[workspace.item]]
  name = "Custom Script"
  kind = "binary"
  target = "/home/dev/.local/bin/my_tool"
  order = 4
"#;

        let parsed: Config = toml::from_str(toml_sample).expect("Failed to parse TOML sample");
        assert_eq!(parsed.general.start_with_hyprland, true);
        assert_eq!(parsed.appearance.arc_radius, 160.0);
        assert_eq!(parsed.appearance.arc_span_degrees, 140.0);
        assert_eq!(parsed.advanced.terminal_override, "kitty");
        assert_eq!(parsed.advanced.file_manager_override, "nautilus");
        assert_eq!(parsed.advanced.auto_fetch_favicons, true);
        assert_eq!(parsed.workspace.len(), 1);
        assert_eq!(parsed.workspace[0].name, "Main");
        assert_eq!(parsed.workspace[0].icon, "folder-code");
        assert_eq!(parsed.workspace[0].items.len(), 5);
        assert_eq!(parsed.workspace[0].items[0].kind, "app");
        assert_eq!(parsed.workspace[0].items[0].item_kind(), ItemKind::App);
        assert_eq!(parsed.workspace[0].items[1].kind, "url");
        assert_eq!(parsed.workspace[0].items[1].item_kind(), ItemKind::Url);
        assert_eq!(parsed.workspace[0].items[2].kind, "folder");
        assert_eq!(parsed.workspace[0].items[2].item_kind(), ItemKind::Folder);
        assert_eq!(parsed.workspace[0].items[3].kind, "file");
        assert_eq!(parsed.workspace[0].items[3].item_kind(), ItemKind::File);
        assert_eq!(parsed.workspace[0].items[4].kind, "binary");
        assert_eq!(parsed.workspace[0].items[4].item_kind(), ItemKind::Binary);

        let serialized = toml::to_string_pretty(&parsed).expect("Failed to serialize Config");
        let reparsed: Config = toml::from_str(&serialized).expect("Failed to reparse serialized TOML");
        assert_eq!(reparsed.workspace.len(), 1);
        assert_eq!(reparsed.workspace[0].items.len(), 5);
        assert_eq!(reparsed.advanced.file_manager_override, "nautilus");
        assert_eq!(reparsed.advanced.auto_fetch_favicons, true);
    }

    #[test]
    fn test_item_kind_serialization() {
        assert_eq!(ItemKind::App.as_str(), "app");
        assert_eq!(ItemKind::Url.as_str(), "url");
        assert_eq!(ItemKind::Folder.as_str(), "folder");
        assert_eq!(ItemKind::File.as_str(), "file");
        assert_eq!(ItemKind::Binary.as_str(), "binary");

        assert_eq!(ItemKind::from_str_loose("app"), ItemKind::App);
        assert_eq!(ItemKind::from_str_loose("URL"), ItemKind::Url);
        assert_eq!(ItemKind::from_str_loose("folder"), ItemKind::Folder);
        assert_eq!(ItemKind::from_str_loose("file"), ItemKind::File);
        assert_eq!(ItemKind::from_str_loose("binary"), ItemKind::Binary);
        assert_eq!(ItemKind::from_str_loose("bin"), ItemKind::Binary);
        assert_eq!(ItemKind::from_str_loose("executable"), ItemKind::Binary);
    }

    #[test]
    fn test_multi_workspace_operations() {
        let mut cfg = Config::default();
        let init_len = cfg.workspace.len();

        // Add extra workspace
        cfg.workspace.push(WorkspaceConfig {
            id: "media".to_string(),
            name: "Media".to_string(),
            icon: "multimedia".to_string(),
            order: init_len,
            items: vec![
                WorkspaceItemConfig {
                    name: "Spotify".to_string(),
                    kind: "app".to_string(),
                    target: "spotify.desktop".to_string(),
                    order: 0,
                    icon: None,
                },
                WorkspaceItemConfig {
                    name: "Music Folder".to_string(),
                    kind: "folder".to_string(),
                    target: "/home/dev/Music".to_string(),
                    order: 1,
                    icon: None,
                },
            ],
        });

        assert_eq!(cfg.workspace.len(), init_len + 1);
        let media_idx = cfg.workspace.len() - 1;
        assert_eq!(cfg.workspace[media_idx].items.len(), 2);

        // Reorder workspaces
        cfg.workspace.swap(0, media_idx);
        assert_eq!(cfg.workspace[0].id, "media");

        let serialized = toml::to_string_pretty(&cfg).expect("Failed to serialize Config");
        let reparsed: Config = toml::from_str(&serialized).expect("Failed to reparse serialized TOML");
        assert_eq!(reparsed.workspace.len(), init_len + 1);
        assert_eq!(reparsed.workspace[0].id, "media");
    }

    #[test]
    fn test_autostart_sync() {
        let mut cfg = Config::default();
        cfg.general.start_with_hyprland = true;
        cfg.general.autostart = true;
        assert!(cfg.sync_hyprland_autostart().is_ok());
        assert!(cfg.sync_xdg_autostart().is_ok());

        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/dev".to_string());
        let desktop_file = std::path::PathBuf::from(&home).join(".config/autostart/hyprvyl.desktop");
        assert!(desktop_file.exists());
        let desktop_content = std::fs::read_to_string(&desktop_file).unwrap();
        assert!(desktop_content.contains("Exec=/home/dev/.local/bin/hyprvyl"));
        assert!(desktop_content.contains("Terminal=false"));
        assert!(!desktop_content.contains("Terminal=falsep"));
    }
}
