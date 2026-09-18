//! Dedicated Settings Application Window for Hyprvyl (Rovyl-Matched).
//!
//! Provides a modern, dark Fluent-styled settings interface matching Rovyl:
//! - Titlebar with Back/Forward navigation controls and status feedback badge.
//! - Left Sidebar (~270px wide, `#0d0d0f` background, search bar, category navigation, version footer).
//! - Main Content Panel (Section title, subtitle, grouped card rows with segmented buttons and toggles).
//! - Workspaces Management: Full CRUD and reordering for Workspaces and Items (App, URL, Folder).
//! - Live persistence to ~/.config/hyprvyl/config.toml, Hyprland autostart sync, and daemon IPC notifications.

use crate::apps::{self, AppEntry};
use crate::config::{Config, TriggerMode, WheelLayout, WorkspaceConfig, WorkspaceItemConfig};
use crate::ipc;
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, DropDown,
    Entry, FlowBox, FlowBoxChild, HeaderBar, Image, Label, ListBox, ListBoxRow, Orientation,
    ScrolledWindow, SearchEntry, SpinButton, Stack, StringList, Switch, Window,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

const CSS_STYLING: &str = r#"
window.settings-window {
    background-color: #111113;
    color: #e4e4e7;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
}

headerbar {
    background-color: #0d0d0f;
    border-bottom: 1px solid #1a1a1d;
    min-height: 44px;
    padding: 0 8px;
}

.nav-arrow-btn {
    background: transparent;
    border: none;
    border-radius: 6px;
    color: #71717a;
    font-size: 15px;
    font-weight: 600;
    padding: 4px 8px;
    margin-right: 4px;
}

.nav-arrow-btn:hover {
    background-color: #1c1c20;
    color: #ffffff;
}

.sidebar {
    background-color: #0d0d0f;
    border-right: 1px solid #1a1a1d;
    padding: 16px 12px;
}

.sidebar-title {
    font-size: 20px;
    font-weight: 700;
    color: #ffffff;
    padding-left: 8px;
    margin-bottom: 14px;
    letter-spacing: -0.3px;
}

.sidebar-search {
    background-color: #18181b;
    border: 1px solid #27272a;
    border-radius: 8px;
    color: #ffffff;
    padding: 6px 10px;
    margin-bottom: 16px;
}

.sidebar-search:focus {
    border-color: #3f3f46;
}

.category-list {
    background-color: transparent;
}

.category-row {
    background-color: transparent;
    border-radius: 8px;
    padding: 9px 12px;
    margin-bottom: 3px;
    color: #a1a1aa;
    font-size: 13.5px;
    font-weight: 500;
    transition: all 120ms ease;
}

.category-row:hover {
    background-color: #18181c;
    color: #f4f4f5;
}

.category-row:selected, .category-row.active {
    background-color: #242428;
    color: #ffffff;
    font-weight: 600;
}

.sidebar-footer {
    padding: 14px 8px 4px 8px;
    border-top: 1px solid #18181b;
}

.sidebar-footer-app {
    font-size: 13px;
    font-weight: 600;
    color: #888890;
}

.sidebar-footer-version {
    font-size: 12px;
    color: #52525b;
}

.content-panel {
    background-color: #111113;
    padding: 28px 40px;
}

.section-title {
    font-size: 26px;
    font-weight: 700;
    color: #ffffff;
    margin-bottom: 3px;
    letter-spacing: -0.4px;
}

.section-desc {
    font-size: 13px;
    color: #71717a;
    margin-bottom: 24px;
}

.group-heading {
    font-size: 11px;
    font-weight: 700;
    color: #71717a;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    margin-top: 20px;
    margin-bottom: 8px;
    padding-left: 2px;
}

.settings-card {
    background-color: #161619;
    border: 1px solid #222226;
    border-radius: 10px;
    margin-bottom: 18px;
}

.setting-row {
    padding: 14px 18px;
    border-bottom: 1px solid #1c1c20;
}

.setting-row:last-child {
    border-bottom: none;
}

.setting-label {
    font-size: 14px;
    font-weight: 600;
    color: #f4f4f5;
}

.setting-sublabel {
    font-size: 12px;
    color: #71717a;
    margin-top: 2px;
}

.status-badge {
    font-size: 11.5px;
    font-weight: 600;
    color: #f4f4f5;
    padding: 3px 10px;
    background-color: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 6px;
}

/* Segmented Button (Picker / Keys) */
.segmented-box {
    background-color: #161619;
    border: 1px solid #27272a;
    border-radius: 8px;
    padding: 3px;
}

.segmented-btn {
    background: transparent;
    border: none;
    border-radius: 6px;
    color: #8e8e93;
    font-size: 12.5px;
    font-weight: 500;
    padding: 5px 14px;
    transition: all 120ms ease;
}

.segmented-btn:hover {
    color: #ffffff;
    background-color: rgba(255, 255, 255, 0.05);
}

.segmented-btn.active {
    background-color: #27272a;
    color: #ffffff;
    font-weight: 600;
}

switch {
    border-radius: 14px;
}

entry, spinbutton, dropdown button {
    background-color: #161619;
    border: 1px solid #27272a;
    border-radius: 6px;
    color: #ffffff;
    padding: 6px 10px;
}

button.action-btn {
    background-color: #161619;
    border: 1px solid #27272a;
    border-radius: 6px;
    color: #ffffff;
    padding: 7px 14px;
    font-weight: 500;
}

button.action-btn:hover {
    background-color: #222226;
}

button.action-btn-primary {
    background-color: #ffffff;
    border: 1px solid #ffffff;
    border-radius: 6px;
    color: #09090b;
    padding: 7px 14px;
    font-weight: 600;
}

button.action-btn-primary:hover {
    background-color: #e4e4e7;
    border-color: #e4e4e7;
}

button.action-btn-danger {
    background-color: #161619;
    border: 1px solid #3f3f46;
    border-radius: 6px;
    color: #e4e4e7;
    padding: 7px 14px;
    font-weight: 500;
}

button.action-btn-danger:hover {
    background-color: #27272a;
}

.workspace-list-box {
    background-color: #141417;
    border: 1px solid #222226;
    border-radius: 8px;
    margin: 8px 0;
}

.item-badge-app {
    background-color: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.14);
    color: #f4f4f5;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
}

.item-badge-url {
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.10);
    color: #d4d4d8;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
}

.item-badge-folder {
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.10);
    color: #a1a1aa;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
}

.item-badge-file {
    background-color: rgba(56, 189, 248, 0.12);
    border: 1px solid rgba(56, 189, 248, 0.25);
    color: #38bdf8;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
}

.item-badge-binary {
    background-color: rgba(245, 158, 11, 0.12);
    border: 1px solid rgba(245, 158, 11, 0.25);
    color: #fbbf24;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
}

.icon-preview-btn {
    background-color: #18181b;
    border: 1px solid #27272a;
    border-radius: 8px;
    padding: 5px 10px;
    color: #ffffff;
    font-size: 12.5px;
    font-weight: 500;
}

.icon-preview-btn:hover {
    background-color: #27272a;
    border-color: #3f3f46;
}

.icon-picker-btn {
    background-color: #18181b;
    border: 1px solid #27272a;
    border-radius: 8px;
    padding: 8px;
    min-width: 44px;
    min-height: 44px;
}

.icon-picker-btn:hover {
    background-color: #27272a;
    border-color: #52525b;
}

.icon-picker-btn.active {
    background-color: #ffffff;
    border-color: #ffffff;
}

.category-chip {
    background-color: #18181b;
    border: 1px solid #27272a;
    border-radius: 14px;
    padding: 4px 12px;
    color: #a1a1aa;
    font-size: 12px;
    font-weight: 500;
}

.category-chip:hover {
    background-color: #27272a;
    color: #ffffff;
}

.category-chip.active {
    background-color: #ffffff;
    border-color: #ffffff;
    color: #09090b;
    font-weight: 600;
}
"#;

pub struct SettingsWindow {
    pub window: ApplicationWindow,
    pub config: Rc<RefCell<Config>>,
    pub status_label: Label,
    pub cached_apps: Rc<RefCell<Vec<AppEntry>>>,
}

impl SettingsWindow {
    pub fn new(app: &Application) -> Rc<Self> {
        let config = Rc::new(RefCell::new(Config::load_or_create()));
        let cached_apps = Rc::new(RefCell::new(apps::discover_applications()));

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Settings")
            .default_width(940)
            .default_height(660)
            .build();

        window.add_css_class("settings-window");

        // Hide window on close request rather than destroying it
        window.connect_close_request(|win| {
            win.set_visible(false);
            glib::Propagation::Stop
        });

        // Attach custom dark theme CSS
        let css_provider = CssProvider::new();
        css_provider.load_from_data(CSS_STYLING);
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &css_provider,
                STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let header = HeaderBar::builder()
            .show_title_buttons(true)
            .build();

        // Nav arrow buttons on headerbar left (Image 3 style)
        let nav_box = GtkBox::new(Orientation::Horizontal, 2);
        let back_btn = Button::with_label("←");
        back_btn.add_css_class("nav-arrow-btn");
        let fwd_btn = Button::with_label("→");
        fwd_btn.add_css_class("nav-arrow-btn");
        nav_box.append(&back_btn);
        nav_box.append(&fwd_btn);
        header.pack_start(&nav_box);

        window.set_titlebar(Some(&header));

        let status_label = Label::new(Some("All changes saved"));
        status_label.add_css_class("status-badge");
        status_label.set_visible(false);
        header.pack_end(&status_label);

        let root_box = GtkBox::new(Orientation::Horizontal, 0);

        // 1. Left Sidebar
        let (sidebar_box, stack, category_list) = Self::build_sidebar();
        root_box.append(&sidebar_box);

        // 2. Main Content Stack
        let settings_win = Rc::new(Self {
            window: window.clone(),
            config: config.clone(),
            status_label: status_label.clone(),
            cached_apps,
        });

        Self::build_content_pages(&settings_win, &stack);

        let content_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .child(&stack)
            .build();
        content_scroll.add_css_class("content-panel");
        root_box.append(&content_scroll);

        // Wire category list selection to switch stack pages
        let stack_for_list = stack.clone();
        category_list.connect_row_selected(move |_list, row| {
            if let Some(row) = row {
                let idx = row.index();
                let page_name = match idx {
                    0 => "general",
                    1 => "activation",
                    2 => "appearance",
                    3 => "workspaces",
                    4 => "advanced",
                    _ => "general",
                };
                stack_for_list.set_visible_child_name(page_name);
            }
        });

        // Select first category by default
        if let Some(row0) = category_list.row_at_index(0) {
            category_list.select_row(Some(&row0));
        }

        window.set_child(Some(&root_box));
        settings_win
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn notify_saved(&self, msg: &str) {
        if let Err(e) = self.config.borrow().save() {
            eprintln!("[Hyprvyl Settings] Failed to save config to disk: {}", e);
        } else {
            println!("[Hyprvyl Settings] Configuration successfully saved to {}", Config::config_path().display());
        }

        let _ = self.config.borrow().sync_hyprland_autostart();
        let _ = self.config.borrow().sync_xdg_autostart();

        // Notify running daemon if available
        match ipc::send_command("RELOAD") {
            Ok(resp) => {
                println!("[Hyprvyl Settings] Notified daemon: {}", resp.trim());
            }
            Err(_) => {
                // Daemon not currently running, changes will be loaded on next startup
            }
        }

        self.status_label.set_text(msg);
        self.status_label.set_visible(true);

        let status_lbl = self.status_label.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(2200), move || {
            status_lbl.set_visible(false);
        });
    }

    fn build_sidebar() -> (GtkBox, Stack, ListBox) {
        let sidebar = GtkBox::new(Orientation::Vertical, 0);
        sidebar.set_size_request(270, -1);
        sidebar.add_css_class("sidebar");

        let title = Label::new(Some("Settings"));
        title.set_halign(Align::Start);
        title.add_css_class("sidebar-title");
        sidebar.append(&title);

        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search"));
        search_entry.add_css_class("sidebar-search");
        sidebar.append(&search_entry);

        let list = ListBox::new();
        list.add_css_class("category-list");
        list.set_selection_mode(gtk4::SelectionMode::Single);

        let categories = [
            ("General", "general"),
            ("Activation", "activation"),
            ("Appearance", "appearance"),
            ("Workspaces", "workspaces"),
            ("Advanced", "advanced"),
        ];

        for (name, _id) in categories {
            let row = ListBoxRow::new();
            row.add_css_class("category-row");
            let row_box = GtkBox::new(Orientation::Horizontal, 10);
            let lbl = Label::new(Some(name));
            lbl.set_halign(Align::Start);
            lbl.set_hexpand(true);
            row_box.append(&lbl);
            row.set_child(Some(&row_box));
            list.append(&row);
        }

        sidebar.append(&list);

        let spacer = GtkBox::new(Orientation::Vertical, 0);
        spacer.set_vexpand(true);
        sidebar.append(&spacer);

        // Sidebar Footer (Image 3 style: "Hyprvyl" on left, "0.1.0" on right)
        let footer = GtkBox::new(Orientation::Horizontal, 0);
        footer.add_css_class("sidebar-footer");
        let app_lbl = Label::new(Some("Hyprvyl"));
        app_lbl.set_halign(Align::Start);
        app_lbl.set_hexpand(true);
        app_lbl.add_css_class("sidebar-footer-app");

        let ver_lbl = Label::new(Some("0.1.0"));
        ver_lbl.set_halign(Align::End);
        ver_lbl.add_css_class("sidebar-footer-version");

        footer.append(&app_lbl);
        footer.append(&ver_lbl);
        sidebar.append(&footer);

        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        stack.set_transition_duration(120);

        // Sidebar search filter logic
        let list_for_search = list.clone();
        search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_lowercase();
            let mut i = 0;
            while let Some(row) = list_for_search.row_at_index(i) {
                if query.is_empty() {
                    row.set_visible(true);
                } else {
                    let cat_name = match i {
                        0 => "general startup autostart login windows hyprland switching",
                        1 => "activation trigger hold click mouse key button device timing",
                        2 => "appearance layout wheel arc radius span size theme icons corner",
                        3 => "workspaces apps urls folders items navigation",
                        4 => "advanced debug logging terminal watcher rescan cache",
                        _ => "",
                    };
                    row.set_visible(cat_name.contains(&query));
                }
                i += 1;
            }
        });

        (sidebar, stack, list)
    }

    fn build_content_pages(settings_win: &Rc<Self>, stack: &Stack) {
        // Page 1: General
        let general_page = Self::build_general_page(settings_win);
        stack.add_named(&general_page, Some("general"));

        // Page 2: Activation
        let activation_page = Self::build_activation_page(settings_win);
        stack.add_named(&activation_page, Some("activation"));

        // Page 3: Appearance
        let appearance_page = Self::build_appearance_page(settings_win);
        stack.add_named(&appearance_page, Some("appearance"));

        // Page 4: Workspaces
        let workspaces_page = Self::build_workspaces_page(settings_win);
        stack.add_named(&workspaces_page, Some("workspaces"));

        // Page 5: Advanced
        let advanced_page = Self::build_advanced_page(settings_win);
        stack.add_named(&advanced_page, Some("advanced"));
    }

    fn create_header_section(title: &str, subtitle: &str) -> GtkBox {
        let container = GtkBox::new(Orientation::Vertical, 2);
        let t = Label::new(Some(title));
        t.set_halign(Align::Start);
        t.add_css_class("section-title");
        let s = Label::new(Some(subtitle));
        s.set_halign(Align::Start);
        s.add_css_class("section-desc");
        container.append(&t);
        container.append(&s);
        container
    }

    fn create_card_row(title: &str, desc: &str, widget: &impl IsA<gtk4::Widget>) -> GtkBox {
        let row = GtkBox::new(Orientation::Horizontal, 16);
        row.add_css_class("setting-row");

        let text_box = GtkBox::new(Orientation::Vertical, 2);
        text_box.set_hexpand(true);
        text_box.set_halign(Align::Start);

        let lbl = Label::new(Some(title));
        lbl.set_halign(Align::Start);
        lbl.add_css_class("setting-label");
        text_box.append(&lbl);

        if !desc.is_empty() {
            let sub = Label::new(Some(desc));
            sub.set_halign(Align::Start);
            sub.add_css_class("setting-sublabel");
            text_box.append(&sub);
        }

        row.append(&text_box);
        widget.set_halign(Align::End);
        widget.set_valign(Align::Center);
        row.append(widget);
        row
    }

    /// Creates a two-segment button widget: [ Option 1 | Option 2 ] (Image 3 style)
    fn create_segmented_button<F>(
        opt1_label: &str,
        opt2_label: &str,
        init_is_opt1: bool,
        on_change: F,
    ) -> GtkBox
    where
        F: FnMut(bool) + 'static,
    {
        let container = GtkBox::new(Orientation::Horizontal, 2);
        container.add_css_class("segmented-box");

        let btn1 = Button::with_label(opt1_label);
        btn1.add_css_class("segmented-btn");
        let btn2 = Button::with_label(opt2_label);
        btn2.add_css_class("segmented-btn");

        if init_is_opt1 {
            btn1.add_css_class("active");
        } else {
            btn2.add_css_class("active");
        }

        let on_change = Rc::new(RefCell::new(on_change));

        {
            let b1 = btn1.clone();
            let b2 = btn2.clone();
            let cb = on_change.clone();
            btn1.connect_clicked(move |_| {
                b1.add_css_class("active");
                b2.remove_css_class("active");
                (cb.borrow_mut())(true);
            });
        }

        {
            let b1 = btn1.clone();
            let b2 = btn2.clone();
            let cb = on_change;
            btn2.connect_clicked(move |_| {
                b2.add_css_class("active");
                b1.remove_css_class("active");
                (cb.borrow_mut())(false);
            });
        }

        container.append(&btn1);
        container.append(&btn2);
        container
    }

    fn build_general_page(settings_win: &Rc<Self>) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 0);
        page.append(&Self::create_header_section("General", "Core Hyprvyl behavior."));

        // Group: Startup (Image 3 style)
        let group1_lbl = Label::new(Some("Startup"));
        group1_lbl.set_halign(Align::Start);
        group1_lbl.add_css_class("group-heading");
        page.append(&group1_lbl);

        let card1 = GtkBox::new(Orientation::Vertical, 0);
        card1.add_css_class("settings-card");

        // Start with Hyprland toggle
        let sw_hypr = Switch::builder()
            .active(settings_win.config.borrow().general.start_with_hyprland)
            .build();
        {
            let win = settings_win.clone();
            sw_hypr.connect_active_notify(move |sw| {
                win.config.borrow_mut().general.start_with_hyprland = sw.is_active();
                win.notify_saved("Startup setting saved");
            });
        }
        card1.append(&Self::create_card_row(
            "Start with Hyprland",
            "Hyprvyl is ready as soon as you sign in to Hyprland.",
            &sw_hypr,
        ));
        page.append(&card1);

        // Group: Workspaces (Image 3 style)
        let group2_lbl = Label::new(Some("Workspaces"));
        group2_lbl.set_halign(Align::Start);
        group2_lbl.add_css_class("group-heading");
        page.append(&group2_lbl);

        let card2 = GtkBox::new(Orientation::Vertical, 0);
        card2.add_css_class("settings-card");

        // Workspace switching [ Picker | Keys ]
        let init_is_picker = settings_win.config.borrow().workspaces.workspace_switching != "keys";
        let win = settings_win.clone();
        let segmented_switching = Self::create_segmented_button(
            "Picker",
            "Keys",
            init_is_picker,
            move |is_picker| {
                win.config.borrow_mut().workspaces.workspace_switching = if is_picker {
                    "picker".to_string()
                } else {
                    "keys".to_string()
                };
                win.notify_saved("Workspace switching mode updated");
            },
        );

        card2.append(&Self::create_card_row(
            "Workspace switching",
            "Use the visual wheel picker or number keys.",
            &segmented_switching,
        ));

        // Close on launch
        let sw_close = Switch::builder()
            .active(settings_win.config.borrow().general.close_on_launch)
            .build();
        {
            let win = settings_win.clone();
            sw_close.connect_active_notify(move |sw| {
                win.config.borrow_mut().general.close_on_launch = sw.is_active();
                win.notify_saved("Behavior setting saved");
            });
        }
        card2.append(&Self::create_card_row(
            "Close on launch",
            "Dismiss the wheel menu after launching an application, URL, or folder.",
            &sw_close,
        ));

        // Reload Hyprvyl daemon button
        let reload_btn = Button::with_label("Reload Hyprvyl");
        reload_btn.add_css_class("action-btn");
        {
            let win = settings_win.clone();
            reload_btn.connect_clicked(move |_| {
                let fresh_apps = apps::discover_applications();
                *win.cached_apps.borrow_mut() = fresh_apps;
                match ipc::send_command("RELOAD") {
                    Ok(resp) => {
                        win.notify_saved(&format!("Hyprvyl reloaded ({})", resp.trim()));
                    }
                    Err(_) => {
                        win.notify_saved("Config & application cache reloaded");
                    }
                }
            });
        }
        card2.append(&Self::create_card_row(
            "Reload Hyprvyl",
            "Apply configuration changes, reload daemon, and rescan applications immediately.",
            &reload_btn,
        ));

        page.append(&card2);

        page
    }

    fn build_activation_page(settings_win: &Rc<Self>) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 0);
        page.append(&Self::create_header_section("Activation", "Trigger configuration, timing, and device binding."));

        let group1_lbl = Label::new(Some("Trigger Mode"));
        group1_lbl.set_halign(Align::Start);
        group1_lbl.add_css_class("group-heading");
        page.append(&group1_lbl);

        let card1 = GtkBox::new(Orientation::Vertical, 0);
        card1.add_css_class("settings-card");

        // Mode Segmented Button (Hold vs Click)
        let is_hold = settings_win.config.borrow().trigger.mode == TriggerMode::Hold;
        let win = settings_win.clone();
        let segmented_mode = Self::create_segmented_button(
            "Hold",
            "Click",
            is_hold,
            move |is_hold_selected| {
                win.config.borrow_mut().trigger.mode = if is_hold_selected {
                    TriggerMode::Hold
                } else {
                    TriggerMode::Click
                };
                win.notify_saved("Trigger mode updated");
            },
        );
        card1.append(&Self::create_card_row(
            "Activation Mode",
            "Hold: opens after holding button; releasing over item launches it. Click: toggle open/close.",
            &segmented_mode,
        ));

        // Hold Duration SpinButton
        let hold_spin = SpinButton::with_range(50.0, 1000.0, 25.0);
        hold_spin.set_value(settings_win.config.borrow().trigger.hold_duration_ms as f64);
        {
            let win = settings_win.clone();
            hold_spin.connect_value_changed(move |sb| {
                win.config.borrow_mut().trigger.hold_duration_ms = sb.value() as u64;
                win.notify_saved("Hold duration updated");
            });
        }
        card1.append(&Self::create_card_row(
            "Hold Duration (ms)",
            "Duration in milliseconds before wheel blooms under cursor (default: 200ms)",
            &hold_spin,
        ));
        page.append(&card1);

        let group2_lbl = Label::new(Some("Input Binding"));
        group2_lbl.set_halign(Align::Start);
        group2_lbl.add_css_class("group-heading");
        page.append(&group2_lbl);

        let card2 = GtkBox::new(Orientation::Vertical, 0);
        card2.add_css_class("settings-card");

        // Trigger Button DropDown (prevents typos)
        let trigger_options: &[(&str, &str)] = &[
            ("Mouse Forward (Button 5 / BTN_EXTRA)", "BTN_EXTRA"),
            ("Mouse Back (Button 4 / BTN_SIDE)", "BTN_SIDE"),
            ("Middle Click (Button 3 / BTN_MIDDLE)", "BTN_MIDDLE"),
            ("Right Click (Button 2 / BTN_RIGHT)", "BTN_RIGHT"),
            ("Left Click (Button 1 / BTN_LEFT)", "BTN_LEFT"),
            ("Task Button (BTN_TASK)", "BTN_TASK"),
            ("Grave / Tilde (` / ~)", "KEY_GRAVE"),
            ("Spacebar (SPACE)", "KEY_SPACE"),
            ("Tab Key (TAB)", "KEY_TAB"),
            ("Caps Lock", "KEY_CAPSLOCK"),
            ("Super / Windows Key (Left)", "KEY_LEFTMETA"),
            ("Alt Key (Left)", "KEY_LEFTALT"),
            ("Control Key (Left)", "KEY_LEFTCTRL"),
            ("Shift Key (Left)", "KEY_LEFTSHIFT"),
            ("Escape (ESC)", "KEY_ESC"),
            ("F1", "KEY_F1"),
            ("F2", "KEY_F2"),
            ("F3", "KEY_F3"),
            ("F4", "KEY_F4"),
            ("F5", "KEY_F5"),
            ("F6", "KEY_F6"),
            ("F7", "KEY_F7"),
            ("F8", "KEY_F8"),
            ("F9", "KEY_F9"),
            ("F10", "KEY_F10"),
            ("F11", "KEY_F11"),
            ("F12", "KEY_F12"),
        ];

        let labels: Vec<&str> = trigger_options.iter().map(|(label, _)| *label).collect();
        let string_list = StringList::new(&labels);
        let btn_dropdown = DropDown::new(Some(string_list), None::<gtk4::Expression>);

        // Find initial selection index
        let cur_button = settings_win.config.borrow().trigger.button.clone();
        let initial_idx = trigger_options
            .iter()
            .position(|(_, code)| code.eq_ignore_ascii_case(&cur_button))
            .unwrap_or(0);
        btn_dropdown.set_selected(initial_idx as u32);

        {
            let win = settings_win.clone();
            btn_dropdown.connect_selected_notify(move |dd| {
                let idx = dd.selected() as usize;
                if let Some((_, code)) = trigger_options.get(idx) {
                    win.config.borrow_mut().trigger.button = code.to_string();
                    win.notify_saved("Trigger button updated");
                }
            });
        }
        card2.append(&Self::create_card_row(
            "Trigger Button / Key",
            "Select the mouse button or keyboard key used to bloom and trigger the radial menu",
            &btn_dropdown,
        ));

        // Device path override
        let dev_entry = Entry::new();
        dev_entry.set_text(&settings_win.config.borrow().trigger.device);
        dev_entry.set_placeholder_text(Some("Auto-detect (leave empty)"));
        dev_entry.set_width_chars(22);
        {
            let win = settings_win.clone();
            dev_entry.connect_changed(move |e| {
                win.config.borrow_mut().trigger.device = e.text().to_string();
                win.notify_saved("Device override updated");
            });
        }
        card2.append(&Self::create_card_row(
            "Explicit Input Device",
            "Optional /dev/input/event* path. Leave blank to auto-detect mouse/keyboard devices.",
            &dev_entry,
        ));
        page.append(&card2);

        page
    }

    fn build_appearance_page(settings_win: &Rc<Self>) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 0);
        page.append(&Self::create_header_section("Appearance", "Radial wheel geometry, button sizing, and styling."));

        let group1_lbl = Label::new(Some("Wheel Layout"));
        group1_lbl.set_halign(Align::Start);
        group1_lbl.add_css_class("group-heading");
        page.append(&group1_lbl);

        let card1 = GtkBox::new(Orientation::Vertical, 0);
        card1.add_css_class("settings-card");

        // Layout Segmented Button: [ Wheel | Arc ]
        let is_wheel = settings_win.config.borrow().appearance.layout == WheelLayout::Wheel;
        let win = settings_win.clone();
        let segmented_layout = Self::create_segmented_button(
            "Wheel (360°)",
            "Arc",
            is_wheel,
            move |is_wheel_selected| {
                win.config.borrow_mut().appearance.layout = if is_wheel_selected {
                    WheelLayout::Wheel
                } else {
                    WheelLayout::Arc
                };
                win.notify_saved("Layout style updated");
            },
        );
        card1.append(&Self::create_card_row(
            "Wheel Layout Style",
            "Full 360° circular radial wheel (Hyprvyl default) or vertical-leaning radial arc",
            &segmented_layout,
        ));

        // Arc Radius
        let arc_r_spin = SpinButton::with_range(100.0, 260.0, 10.0);
        arc_r_spin.set_value(settings_win.config.borrow().appearance.arc_radius);
        {
            let win = settings_win.clone();
            arc_r_spin.connect_value_changed(move |sb| {
                win.config.borrow_mut().appearance.arc_radius = sb.value();
                win.notify_saved("Radius updated");
            });
        }
        card1.append(&Self::create_card_row(
            "Wheel Radius (px)",
            "Distance from cursor center to floating radial buttons (default: 140px)",
            &arc_r_spin,
        ));

        // Items per page
        let items_spin = SpinButton::with_range(4.0, 16.0, 1.0);
        items_spin.set_value(settings_win.config.borrow().appearance.items_per_page as f64);
        {
            let win = settings_win.clone();
            items_spin.connect_value_changed(move |sb| {
                win.config.borrow_mut().appearance.items_per_page = sb.value() as usize;
                win.notify_saved("Items per page updated");
            });
        }
        card1.append(&Self::create_card_row(
            "Items per Page",
            "Maximum number of buttons displayed simultaneously along the wheel (default: 8)",
            &items_spin,
        ));
        page.append(&card1);

        let group2_lbl = Label::new(Some("Button Sizing & Theme"));
        group2_lbl.set_halign(Align::Start);
        group2_lbl.add_css_class("group-heading");
        page.append(&group2_lbl);

        let card2 = GtkBox::new(Orientation::Vertical, 0);
        card2.add_css_class("settings-card");

        // Button Size
        let size_spin = SpinButton::with_range(40.0, 80.0, 2.0);
        size_spin.set_value(settings_win.config.borrow().appearance.button_size);
        {
            let win = settings_win.clone();
            size_spin.connect_value_changed(move |sb| {
                win.config.borrow_mut().appearance.button_size = sb.value();
                win.notify_saved("Button size updated");
            });
        }
        card2.append(&Self::create_card_row(
            "Button Size (px)",
            "Width and height of floating rounded square buttons (default: 56px)",
            &size_spin,
        ));

        // Corner Radius
        let radius_spin = SpinButton::with_range(4.0, 24.0, 1.0);
        radius_spin.set_value(settings_win.config.borrow().appearance.corner_radius);
        {
            let win = settings_win.clone();
            radius_spin.connect_value_changed(move |sb| {
                win.config.borrow_mut().appearance.corner_radius = sb.value();
                win.notify_saved("Corner radius updated");
            });
        }
        card2.append(&Self::create_card_row(
            "Corner Radius (px)",
            "Curvature of button corners (default: 14px)",
            &radius_spin,
        ));

        // Icon Size
        let icon_spin = SpinButton::with_range(24.0, 48.0, 2.0);
        icon_spin.set_value(settings_win.config.borrow().appearance.icon_size as f64);
        {
            let win = settings_win.clone();
            icon_spin.connect_value_changed(move |sb| {
                win.config.borrow_mut().appearance.icon_size = sb.value() as i32;
                win.notify_saved("Icon size updated");
            });
        }
        card2.append(&Self::create_card_row(
            "Icon Size (px)",
            "Rendered size of application and kind icons centered inside buttons (default: 36px)",
            &icon_spin,
        ));

        // Theme Segmented Button [ Dark | OLED ]
        let is_dark_theme = settings_win.config.borrow().appearance.theme != "oled";
        let win = settings_win.clone();
        let segmented_theme = Self::create_segmented_button(
            "Dark",
            "OLED",
            is_dark_theme,
            move |is_dark| {
                win.config.borrow_mut().appearance.theme = if is_dark {
                    "dark".to_string()
                } else {
                    "oled".to_string()
                };
                win.notify_saved("Theme variant updated");
            },
        );
        card2.append(&Self::create_card_row(
            "Color Theme",
            "Choose between Dark Grey and pure black OLED aesthetic",
            &segmented_theme,
        ));

        page.append(&card2);

        page
    }

    fn build_workspaces_page(settings_win: &Rc<Self>) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 0);
        page.append(&Self::create_header_section(
            "Workspaces",
            "Configure workspaces and item launchers (Apps, URLs, Folders).",
        ));

        let main_box = GtkBox::new(Orientation::Vertical, 12);

        let selected_ws_idx = Rc::new(RefCell::new(0usize));

        let ws_list_box = ListBox::new();
        ws_list_box.add_css_class("workspace-list-box");
        ws_list_box.set_selection_mode(gtk4::SelectionMode::Single);

        let items_list_box = ListBox::new();
        items_list_box.add_css_class("workspace-list-box");
        items_list_box.set_selection_mode(gtk4::SelectionMode::Single);

        let items_title_lbl = Label::new(Some("Workspace Items"));
        items_title_lbl.set_halign(Align::Start);
        items_title_lbl.add_css_class("group-heading");

        let is_refreshing = Rc::new(RefCell::new(false));

        // Refresh items closure
        let refresh_items_ui = {
            let win = settings_win.clone();
            let it_box = items_list_box.clone();
            let it_title = items_title_lbl.clone();
            let sel_idx = selected_ws_idx.clone();

            Rc::new(move || {
                let cur = *sel_idx.borrow();
                let cfg = win.config.borrow();

                while let Some(child) = it_box.first_child() {
                    it_box.remove(&child);
                }

                if let Some(ws) = cfg.workspace.get(cur) {
                    it_title.set_text(&format!("Items in '{}' ({} items)", ws.name, ws.items.len()));
                    for it in &ws.items {
                        let row = ListBoxRow::new();
                        row.add_css_class("category-row");

                        let row_box = GtkBox::new(Orientation::Horizontal, 12);
                        row_box.set_hexpand(true);

                        let badge_lbl = Label::new(Some(&it.kind.to_uppercase()));
                        match it.kind.as_str() {
                            "app" => badge_lbl.add_css_class("item-badge-app"),
                            "url" => badge_lbl.add_css_class("item-badge-url"),
                            "folder" => badge_lbl.add_css_class("item-badge-folder"),
                            "file" => badge_lbl.add_css_class("item-badge-file"),
                            "binary" | "bin" | "executable" => badge_lbl.add_css_class("item-badge-binary"),
                            _ => badge_lbl.add_css_class("item-badge-app"),
                        }
                        row_box.append(&badge_lbl);

                        let name_lbl = Label::new(Some(&it.name));
                        name_lbl.set_halign(Align::Start);
                        name_lbl.set_hexpand(true);
                        row_box.append(&name_lbl);

                        let target_lbl = Label::new(Some(&it.target));
                        target_lbl.add_css_class("setting-sublabel");
                        target_lbl.set_max_width_chars(30);
                        target_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                        row_box.append(&target_lbl);

                        row.set_child(Some(&row_box));
                        it_box.append(&row);
                    }
                }
            })
        };

        // Refresh workspaces closure
        let refresh_workspaces_ui = {
            let win = settings_win.clone();
            let ws_box = ws_list_box.clone();
            let sel_idx = selected_ws_idx.clone();
            let is_ref = is_refreshing.clone();
            let refresh_items = refresh_items_ui.clone();

            Rc::new(move || {
                *is_ref.borrow_mut() = true;

                while let Some(child) = ws_box.first_child() {
                    ws_box.remove(&child);
                }

                let cfg = win.config.borrow();
                let ws_count = cfg.workspace.len();

                for ws in cfg.workspace.iter() {
                    let row = ListBoxRow::new();
                    row.add_css_class("category-row");

                    let row_box = GtkBox::new(Orientation::Horizontal, 12);
                    row_box.set_hexpand(true);

                    // Workspace icon preview
                    let icon_box = GtkBox::new(Orientation::Horizontal, 4);
                    if let Some(pixbuf) = crate::icons::get_bundled_icon_pixbuf(&ws.icon, 20) {
                        let texture = gdk::Texture::for_pixbuf(&pixbuf);
                        let img = Image::from_paintable(Some(&texture));
                        icon_box.append(&img);
                    }
                    row_box.append(&icon_box);

                    let name_lbl = Label::new(Some(&ws.name));
                    name_lbl.set_halign(Align::Start);
                    name_lbl.set_hexpand(true);
                    row_box.append(&name_lbl);

                    let icon_name_lbl = Label::new(Some(&format!("[{}]", ws.icon)));
                    icon_name_lbl.add_css_class("setting-sublabel");
                    row_box.append(&icon_name_lbl);

                    let count_lbl = Label::new(Some(&format!("{} items", ws.items.len())));
                    count_lbl.add_css_class("setting-sublabel");
                    row_box.append(&count_lbl);

                    row.set_child(Some(&row_box));
                    ws_box.append(&row);
                }

                let current_sel = *sel_idx.borrow();
                let safe_sel = if current_sel >= ws_count && ws_count > 0 {
                    ws_count - 1
                } else {
                    current_sel
                };
                *sel_idx.borrow_mut() = safe_sel;

                if let Some(row) = ws_box.row_at_index(safe_sel as i32) {
                    ws_box.select_row(Some(&row));
                }

                *is_ref.borrow_mut() = false;
                refresh_items();
            })
        };

        // Workspaces Master Section
        let ws_header_box = GtkBox::new(Orientation::Horizontal, 8);
        let ws_heading = Label::new(Some("Workspaces"));
        ws_heading.set_halign(Align::Start);
        ws_heading.set_hexpand(true);
        ws_heading.add_css_class("group-heading");
        ws_header_box.append(&ws_heading);

        let add_ws_btn = Button::with_label("+ Add Workspace");
        add_ws_btn.add_css_class("action-btn-primary");
        {
            let win = settings_win.clone();
            let refresh = refresh_workspaces_ui.clone();
            add_ws_btn.connect_clicked(move |_| {
                Self::show_add_workspace_dialog(&win, refresh.clone());
            });
        }
        ws_header_box.append(&add_ws_btn);

        let edit_icon_btn = Button::with_label("Change Icon");
        edit_icon_btn.add_css_class("action-btn");
        {
            let win = settings_win.clone();
            let sel_idx = selected_ws_idx.clone();
            let refresh = refresh_workspaces_ui.clone();
            edit_icon_btn.connect_clicked(move |_| {
                let cur = *sel_idx.borrow();
                let cur_icon = {
                    let cfg = win.config.borrow();
                    cfg.workspace.get(cur).map(|w| w.icon.clone()).unwrap_or_else(|| "folder".to_string())
                };
                let win_clone = win.clone();
                let refresh_clone = refresh.clone();
                Self::show_icon_picker_dialog(&win.window, &cur_icon, Rc::new(move |new_icon| {
                    let mut cfg = win_clone.config.borrow_mut();
                    if let Some(w) = cfg.workspace.get_mut(cur) {
                        w.icon = new_icon;
                        drop(cfg);
                        win_clone.notify_saved("Workspace icon updated");
                        refresh_clone();
                    }
                }));
            });
        }
        ws_header_box.append(&edit_icon_btn);

        let ws_up_btn = Button::with_label("↑");
        ws_up_btn.add_css_class("action-btn");
        {
            let win = settings_win.clone();
            let sel_idx = selected_ws_idx.clone();
            let refresh = refresh_workspaces_ui.clone();
            ws_up_btn.connect_clicked(move |_| {
                let cur = *sel_idx.borrow();
                if cur > 0 {
                    let mut cfg = win.config.borrow_mut();
                    cfg.workspace.swap(cur, cur - 1);
                    for (i, w) in cfg.workspace.iter_mut().enumerate() {
                        w.order = i;
                    }
                    drop(cfg);
                    *sel_idx.borrow_mut() = cur - 1;
                    win.notify_saved("Workspace reordered");
                    refresh();
                }
            });
        }
        ws_header_box.append(&ws_up_btn);

        let ws_down_btn = Button::with_label("↓");
        ws_down_btn.add_css_class("action-btn");
        {
            let win = settings_win.clone();
            let sel_idx = selected_ws_idx.clone();
            let refresh = refresh_workspaces_ui.clone();
            ws_down_btn.connect_clicked(move |_| {
                let cur = *sel_idx.borrow();
                let len = win.config.borrow().workspace.len();
                if cur + 1 < len {
                    let mut cfg = win.config.borrow_mut();
                    cfg.workspace.swap(cur, cur + 1);
                    for (i, w) in cfg.workspace.iter_mut().enumerate() {
                        w.order = i;
                    }
                    drop(cfg);
                    *sel_idx.borrow_mut() = cur + 1;
                    win.notify_saved("Workspace reordered");
                    refresh();
                }
            });
        }
        ws_header_box.append(&ws_down_btn);

        let del_ws_btn = Button::with_label("Delete");
        del_ws_btn.add_css_class("action-btn-danger");
        {
            let win = settings_win.clone();
            let sel_idx = selected_ws_idx.clone();
            let refresh = refresh_workspaces_ui.clone();
            del_ws_btn.connect_clicked(move |_| {
                let mut cfg = win.config.borrow_mut();
                if cfg.workspace.len() > 1 {
                    let cur = *sel_idx.borrow();
                    if cur < cfg.workspace.len() {
                        cfg.workspace.remove(cur);
                        for (i, w) in cfg.workspace.iter_mut().enumerate() {
                            w.order = i;
                        }
                        drop(cfg);
                        win.notify_saved("Workspace deleted");
                        refresh();
                    }
                }
            });
        }
        ws_header_box.append(&del_ws_btn);

        main_box.append(&ws_header_box);
        main_box.append(&ws_list_box);

        // Connect Workspace selection changes
        {
            let sel_idx = selected_ws_idx.clone();
            let is_ref = is_refreshing.clone();
            let refresh_items = refresh_items_ui.clone();
            ws_list_box.connect_row_selected(move |_list, row| {
                if *is_ref.borrow() {
                    return;
                }
                if let Some(row) = row {
                    *sel_idx.borrow_mut() = row.index() as usize;
                    refresh_items();
                }
            });
        }

        // Items Detail Section
        let it_header_box = GtkBox::new(Orientation::Horizontal, 8);
        items_title_lbl.set_hexpand(true);
        it_header_box.append(&items_title_lbl);

        let add_it_btn = Button::with_label("+ Add Item");
        add_it_btn.add_css_class("action-btn-primary");
        {
            let win = settings_win.clone();
            let sel_idx = selected_ws_idx.clone();
            let refresh = refresh_workspaces_ui.clone();
            add_it_btn.connect_clicked(move |_| {
                let ws_idx = *sel_idx.borrow();
                Self::show_add_item_dialog(&win, ws_idx, refresh.clone());
            });
        }
        it_header_box.append(&add_it_btn);

        let it_up_btn = Button::with_label("↑");
        it_up_btn.add_css_class("action-btn");
        {
            let win = settings_win.clone();
            let sel_ws = selected_ws_idx.clone();
            let it_box = items_list_box.clone();
            let refresh = refresh_workspaces_ui.clone();
            it_up_btn.connect_clicked(move |_| {
                if let Some(sel_row) = it_box.selected_row() {
                    let cur_it = sel_row.index() as usize;
                    let ws_idx = *sel_ws.borrow();
                    if cur_it > 0 {
                        let mut cfg = win.config.borrow_mut();
                        if let Some(ws) = cfg.workspace.get_mut(ws_idx) {
                            ws.items.swap(cur_it, cur_it - 1);
                            for (i, it) in ws.items.iter_mut().enumerate() {
                                it.order = i;
                            }
                            drop(cfg);
                            win.notify_saved("Item reordered");
                            refresh();
                            if let Some(new_row) = it_box.row_at_index((cur_it - 1) as i32) {
                                it_box.select_row(Some(&new_row));
                            }
                        }
                    }
                }
            });
        }
        it_header_box.append(&it_up_btn);

        let it_down_btn = Button::with_label("↓");
        it_down_btn.add_css_class("action-btn");
        {
            let win = settings_win.clone();
            let sel_ws = selected_ws_idx.clone();
            let it_box = items_list_box.clone();
            let refresh = refresh_workspaces_ui.clone();
            it_down_btn.connect_clicked(move |_| {
                if let Some(sel_row) = it_box.selected_row() {
                    let cur_it = sel_row.index() as usize;
                    let ws_idx = *sel_ws.borrow();
                    let mut cfg = win.config.borrow_mut();
                    if let Some(ws) = cfg.workspace.get_mut(ws_idx)
                        && cur_it + 1 < ws.items.len() {
                            ws.items.swap(cur_it, cur_it + 1);
                            for (i, it) in ws.items.iter_mut().enumerate() {
                                it.order = i;
                            }
                            drop(cfg);
                            win.notify_saved("Item reordered");
                            refresh();
                            if let Some(new_row) = it_box.row_at_index((cur_it + 1) as i32) {
                                it_box.select_row(Some(&new_row));
                            }
                        }
                }
            });
        }
        it_header_box.append(&it_down_btn);

        let del_it_btn = Button::with_label("Delete");
        del_it_btn.add_css_class("action-btn-danger");
        {
            let win = settings_win.clone();
            let sel_ws = selected_ws_idx.clone();
            let it_box = items_list_box.clone();
            let refresh = refresh_workspaces_ui.clone();
            del_it_btn.connect_clicked(move |_| {
                if let Some(sel_row) = it_box.selected_row() {
                    let cur_it = sel_row.index() as usize;
                    let ws_idx = *sel_ws.borrow();
                    let mut cfg = win.config.borrow_mut();
                    if let Some(ws) = cfg.workspace.get_mut(ws_idx)
                        && cur_it < ws.items.len() {
                            ws.items.remove(cur_it);
                            for (i, it) in ws.items.iter_mut().enumerate() {
                                it.order = i;
                            }
                            drop(cfg);
                            win.notify_saved("Item deleted");
                            refresh();
                        }
                }
            });
        }
        it_header_box.append(&del_it_btn);

        main_box.append(&it_header_box);
        main_box.append(&items_list_box);

        refresh_workspaces_ui();

        page.append(&main_box);
        page
    }

    /// Searchable Icon Picker Dialog with 180+ bundled SVG icons and category filter chips.
    fn show_icon_picker_dialog(parent_win: &impl glib::object::IsA<gtk4::Window>, current_icon: &str, on_selected: Rc<dyn Fn(String)>) {
        let dialog = Window::builder()
            .title("Select Workspace Icon")
            .transient_for(parent_win)
            .modal(true)
            .default_width(580)
            .default_height(500)
            .build();
        dialog.add_css_class("settings-window");

        let box_root = GtkBox::new(Orientation::Vertical, 14);
        box_root.set_margin_top(18);
        box_root.set_margin_bottom(18);
        box_root.set_margin_start(18);
        box_root.set_margin_end(18);

        // Search Bar
        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search 180+ icons (e.g. folder, code, web, terminal)..."));
        box_root.append(&search_entry);

        // Category Filter Chips
        let categories = crate::icons::get_icon_categories();
        let cat_box = GtkBox::new(Orientation::Horizontal, 6);
        let selected_cat = Rc::new(RefCell::new("All".to_string()));
        let chip_buttons: Rc<RefCell<Vec<Button>>> = Rc::new(RefCell::new(Vec::new()));

        let cat_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .vscrollbar_policy(gtk4::PolicyType::Never)
            .child(&cat_box)
            .build();
        box_root.append(&cat_scroll);

        // FlowBox for Icon Grid
        let flow_box = FlowBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .max_children_per_line(8)
            .min_children_per_line(4)
            .row_spacing(8)
            .column_spacing(8)
            .homogeneous(true)
            .build();

        struct IconPickerItem {
            child: FlowBoxChild,
            id: &'static str,
            name: &'static str,
            category: &'static str,
        }

        let all_icons = crate::icons::get_all_bundled_icons();
        let icon_elements: Rc<RefCell<Vec<IconPickerItem>>> = Rc::new(RefCell::new(Vec::new()));

        for icon in all_icons {
            let child = FlowBoxChild::new();
            let btn = Button::new();
            btn.add_css_class("icon-picker-btn");
            if icon.id.eq_ignore_ascii_case(current_icon) {
                btn.add_css_class("active");
            }
            btn.set_tooltip_text(Some(&format!("{} ({})", icon.name, icon.id)));

            if let Some(pixbuf) = crate::icons::get_bundled_icon_pixbuf(icon.id, 24) {
                let texture = gdk::Texture::for_pixbuf(&pixbuf);
                let img = Image::from_paintable(Some(&texture));
                btn.set_child(Some(&img));
            } else {
                let lbl = Label::new(Some(icon.id));
                btn.set_child(Some(&lbl));
            }

            let on_sel = on_selected.clone();
            let icon_id = icon.id.to_string();
            let dlg = dialog.clone();
            btn.connect_clicked(move |_| {
                on_sel(icon_id.clone());
                dlg.close();
            });

            child.set_child(Some(&btn));
            flow_box.insert(&child, -1);
            icon_elements.borrow_mut().push(IconPickerItem {
                child,
                id: icon.id,
                name: icon.name,
                category: icon.category,
            });
        }

        let grid_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .child(&flow_box)
            .build();
        box_root.append(&grid_scroll);

        // Filter helper
        let filter_icons = {
            let elements = icon_elements.clone();
            let sel_cat = selected_cat.clone();
            let s_entry = search_entry.clone();

            Rc::new(move || {
                let q = s_entry.text().to_lowercase();
                let cat = sel_cat.borrow().clone();

                for item in elements.borrow().iter() {
                    let matches_q = q.is_empty()
                        || item.id.to_lowercase().contains(&q)
                        || item.name.to_lowercase().contains(&q);
                    let matches_cat = cat == "All" || item.category == cat;
                    item.child.set_visible(matches_q && matches_cat);
                }
            })
        };

        // Wire category chips
        for &cat_name in categories {
            let chip = Button::with_label(cat_name);
            chip.add_css_class("category-chip");
            if cat_name == "All" {
                chip.add_css_class("active");
            }

            let sel_cat = selected_cat.clone();
            let filter = filter_icons.clone();
            let chips_ref = chip_buttons.clone();
            let cat_str = cat_name.to_string();

            chip.connect_clicked(move |btn| {
                *sel_cat.borrow_mut() = cat_str.clone();
                for c in chips_ref.borrow().iter() {
                    c.remove_css_class("active");
                }
                btn.add_css_class("active");
                filter();
            });

            cat_box.append(&chip);
            chip_buttons.borrow_mut().push(chip);
        }

        // Wire search entry
        {
            let filter = filter_icons.clone();
            search_entry.connect_search_changed(move |_| {
                filter();
            });
        }

        // Close button at bottom
        let btn_box = GtkBox::new(Orientation::Horizontal, 8);
        btn_box.set_halign(Align::End);
        let cancel_btn = Button::with_label("Cancel");
        cancel_btn.add_css_class("action-btn");
        {
            let dlg = dialog.clone();
            cancel_btn.connect_clicked(move |_| dlg.close());
        }
        btn_box.append(&cancel_btn);
        box_root.append(&btn_box);

        dialog.set_child(Some(&box_root));
        dialog.present();
    }

    /// Dialog to create a new workspace.
    fn show_add_workspace_dialog(win: &Rc<Self>, on_saved: Rc<dyn Fn()>) {
        let dialog = Window::builder()
            .title("Add New Workspace")
            .transient_for(&win.window)
            .modal(true)
            .default_width(420)
            .default_height(260)
            .build();
        dialog.add_css_class("settings-window");

        let box_root = GtkBox::new(Orientation::Vertical, 14);
        box_root.set_margin_top(20);
        box_root.set_margin_bottom(20);
        box_root.set_margin_start(20);
        box_root.set_margin_end(20);

        let title_lbl = Label::new(Some("Workspace Name"));
        title_lbl.set_halign(Align::Start);
        title_lbl.add_css_class("setting-label");
        box_root.append(&title_lbl);

        let name_entry = Entry::new();
        name_entry.set_placeholder_text(Some("e.g. Work, Gaming, Media, Personal"));
        box_root.append(&name_entry);

        // Icon picker selector
        let icon_val = Rc::new(RefCell::new("folder".to_string()));
        let icon_row_box = GtkBox::new(Orientation::Horizontal, 10);
        let icon_lbl = Label::new(Some("Workspace Icon:"));
        icon_lbl.add_css_class("setting-label");
        icon_row_box.append(&icon_lbl);

        let icon_btn = Button::new();
        icon_btn.add_css_class("icon-preview-btn");
        let update_icon_btn_display = {
            let icon_btn = icon_btn.clone();
            let icon_val = icon_val.clone();
            Rc::new(move || {
                let cur = icon_val.borrow().clone();
                let icon_content_box = GtkBox::new(Orientation::Horizontal, 6);
                if let Some(pixbuf) = crate::icons::get_bundled_icon_pixbuf(&cur, 18) {
                    let texture = gdk::Texture::for_pixbuf(&pixbuf);
                    let img = Image::from_paintable(Some(&texture));
                    icon_content_box.append(&img);
                }
                let lbl = Label::new(Some(&format!("{} (Change...)", cur)));
                icon_content_box.append(&lbl);
                icon_btn.set_child(Some(&icon_content_box));
            })
        };
        update_icon_btn_display();

        {
            let dlg = dialog.clone();
            let icon_val = icon_val.clone();
            let update_display = update_icon_btn_display.clone();
            icon_btn.connect_clicked(move |_| {
                let cur = icon_val.borrow().clone();
                let icon_val_for_sel = icon_val.clone();
                let update_for_sel = update_display.clone();
                Self::show_icon_picker_dialog(&dlg, &cur, Rc::new(move |chosen| {
                    *icon_val_for_sel.borrow_mut() = chosen;
                    update_for_sel();
                }));
            });
        }
        icon_row_box.append(&icon_btn);
        box_root.append(&icon_row_box);

        let btn_box = GtkBox::new(Orientation::Horizontal, 10);
        btn_box.set_halign(Align::End);

        let cancel_btn = Button::with_label("Cancel");
        cancel_btn.add_css_class("action-btn");
        {
            let dlg = dialog.clone();
            cancel_btn.connect_clicked(move |_| dlg.close());
        }
        btn_box.append(&cancel_btn);

        let create_btn = Button::with_label("Create Workspace");
        create_btn.add_css_class("action-btn-primary");
        {
            let win = win.clone();
            let dlg = dialog.clone();
            let entry = name_entry.clone();
            let icon_val = icon_val.clone();
            create_btn.connect_clicked(move |_| {
                let name = entry.text().trim().to_string();
                let chosen_icon = icon_val.borrow().clone();
                if !name.is_empty() {
                    let mut cfg = win.config.borrow_mut();
                    let base_id = name.to_lowercase().replace(' ', "-");
                    let mut id = base_id.clone();
                    let mut counter = 2;
                    while cfg.workspace.iter().any(|w| w.id == id) {
                        id = format!("{}-{}", base_id, counter);
                        counter += 1;
                    }
                    let order = cfg.workspace.len();
                    cfg.workspace.push(WorkspaceConfig {
                        id,
                        name,
                        icon: chosen_icon,
                        order,
                        items: Vec::new(),
                    });
                    drop(cfg);
                    win.notify_saved("New workspace created");
                    on_saved();
                    dlg.close();
                }
            });
        }
        btn_box.append(&create_btn);
        box_root.append(&btn_box);

        dialog.set_child(Some(&box_root));
        dialog.present();
    }

    /// Dialog to add an Item to a workspace (App, URL, Folder, File, Binary).
    fn show_add_item_dialog(win: &Rc<Self>, ws_idx: usize, on_saved: Rc<dyn Fn()>) {
        let dialog = Window::builder()
            .title("Add Workspace Item")
            .transient_for(&win.window)
            .modal(true)
            .default_width(540)
            .default_height(460)
            .build();
        dialog.add_css_class("settings-window");

        let box_root = GtkBox::new(Orientation::Vertical, 16);
        box_root.set_margin_top(20);
        box_root.set_margin_bottom(20);
        box_root.set_margin_start(20);
        box_root.set_margin_end(20);

        // Kind selection (App, URL, Folder, File, Binary)
        let kind_box = GtkBox::new(Orientation::Horizontal, 12);
        let kind_lbl = Label::new(Some("Item Type:"));
        kind_lbl.add_css_class("setting-label");
        kind_box.append(&kind_lbl);

        let kind_list = StringList::new(&[
            "Installed Application",
            "Web URL",
            "Folder / Directory",
            "File / Document",
            "Executable Binary",
        ]);
        let kind_dropdown = DropDown::new(Some(kind_list), None::<gtk4::Expression>);
        kind_box.append(&kind_dropdown);
        box_root.append(&kind_box);

        let stack = Stack::new();

        // 1. App Tab
        let app_page = GtkBox::new(Orientation::Vertical, 10);
        let app_search = SearchEntry::new();
        app_search.set_placeholder_text(Some("Search installed applications..."));
        app_page.append(&app_search);

        let app_list_box = ListBox::new();
        app_list_box.add_css_class("workspace-list-box");
        app_list_box.set_selection_mode(gtk4::SelectionMode::Single);

        let cached_apps = win.cached_apps.borrow().clone();
        for app in &cached_apps {
            let row = ListBoxRow::new();
            row.add_css_class("category-row");

            let r_box = GtkBox::new(Orientation::Horizontal, 10);
            let n_lbl = Label::new(Some(&app.name));
            n_lbl.set_halign(Align::Start);
            n_lbl.set_hexpand(true);
            r_box.append(&n_lbl);

            let id_lbl = Label::new(Some(&app.id));
            id_lbl.add_css_class("setting-sublabel");
            r_box.append(&id_lbl);

            row.set_child(Some(&r_box));
            app_list_box.append(&row);
        }

        let app_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .height_request(220)
            .child(&app_list_box)
            .build();
        app_page.append(&app_scroll);

        // Filter apps search
        let app_list_box_for_filter = app_list_box.clone();
        let cached_apps_for_filter = cached_apps.clone();
        app_search.connect_search_changed(move |entry| {
            let q = entry.text().to_lowercase();
            for (idx, app) in cached_apps_for_filter.iter().enumerate() {
                if let Some(row) = app_list_box_for_filter.row_at_index(idx as i32) {
                    if q.is_empty() || app.name.to_lowercase().contains(&q) || app.id.to_lowercase().contains(&q) {
                        row.set_visible(true);
                    } else {
                        row.set_visible(false);
                    }
                }
            }
        });

        stack.add_named(&app_page, Some("app"));

        // 2. URL Tab
        let url_page = GtkBox::new(Orientation::Vertical, 12);
        let url_name_lbl = Label::new(Some("Display Name"));
        url_name_lbl.set_halign(Align::Start);
        url_name_lbl.add_css_class("setting-label");
        url_page.append(&url_name_lbl);
        let url_name_entry = Entry::new();
        url_name_entry.set_placeholder_text(Some("e.g. WhatsApp, GitHub, YouTube"));
        url_page.append(&url_name_entry);

        let url_target_lbl = Label::new(Some("URL Address"));
        url_target_lbl.set_halign(Align::Start);
        url_target_lbl.add_css_class("setting-label");
        url_page.append(&url_target_lbl);
        let url_target_entry = Entry::new();
        url_target_entry.set_placeholder_text(Some("https://web.whatsapp.com"));
        url_page.append(&url_target_entry);

        stack.add_named(&url_page, Some("url"));

        // 3. Folder Tab
        let folder_page = GtkBox::new(Orientation::Vertical, 12);
        let folder_name_lbl = Label::new(Some("Display Name"));
        folder_name_lbl.set_halign(Align::Start);
        folder_name_lbl.add_css_class("setting-label");
        folder_page.append(&folder_name_lbl);
        let folder_name_entry = Entry::new();
        folder_name_entry.set_placeholder_text(Some("e.g. Projects, Downloads, Documents"));
        folder_page.append(&folder_name_entry);

        let folder_target_lbl = Label::new(Some("Directory Path"));
        folder_target_lbl.set_halign(Align::Start);
        folder_target_lbl.add_css_class("setting-label");
        folder_page.append(&folder_target_lbl);

        let folder_input_box = GtkBox::new(Orientation::Horizontal, 8);
        let folder_target_entry = Entry::new();
        folder_target_entry.set_hexpand(true);
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/dev".to_string());
        folder_target_entry.set_text(&format!("{}/Projects", home));
        folder_input_box.append(&folder_target_entry);

        let browse_folder_btn = Button::with_label("Browse...");
        browse_folder_btn.add_css_class("action-btn");
        {
            let entry = folder_target_entry.clone();
            let parent_win = dialog.clone();
            browse_folder_btn.connect_clicked(move |_| {
                let chooser = gtk4::FileChooserDialog::builder()
                    .title("Select Folder")
                    .action(gtk4::FileChooserAction::SelectFolder)
                    .transient_for(&parent_win)
                    .modal(true)
                    .build();
                chooser.add_button("Cancel", gtk4::ResponseType::Cancel);
                chooser.add_button("Select", gtk4::ResponseType::Accept);

                let entry_for_resp = entry.clone();
                chooser.connect_response(move |dialog, response| {
                    if response == gtk4::ResponseType::Accept
                        && let Some(file) = dialog.file()
                            && let Some(path) = file.path() {
                                entry_for_resp.set_text(&path.to_string_lossy());
                            }
                    dialog.close();
                });
                chooser.show();
            });
        }
        folder_input_box.append(&browse_folder_btn);
        folder_page.append(&folder_input_box);

        stack.add_named(&folder_page, Some("folder"));

        // 4. File Tab
        let file_page = GtkBox::new(Orientation::Vertical, 12);
        let file_name_lbl = Label::new(Some("Display Name"));
        file_name_lbl.set_halign(Align::Start);
        file_name_lbl.add_css_class("setting-label");
        file_page.append(&file_name_lbl);
        let file_name_entry = Entry::new();
        file_name_entry.set_placeholder_text(Some("e.g. Project Notes, Roadmap, Config"));
        file_page.append(&file_name_entry);

        let file_target_lbl = Label::new(Some("File Path"));
        file_target_lbl.set_halign(Align::Start);
        file_target_lbl.add_css_class("setting-label");
        file_page.append(&file_target_lbl);

        let file_input_box = GtkBox::new(Orientation::Horizontal, 8);
        let file_target_entry = Entry::new();
        file_target_entry.set_hexpand(true);
        file_target_entry.set_placeholder_text(Some("/path/to/document.pdf or text file"));
        file_input_box.append(&file_target_entry);

        let browse_file_btn = Button::with_label("Browse...");
        browse_file_btn.add_css_class("action-btn");
        {
            let entry = file_target_entry.clone();
            let name_entry = file_name_entry.clone();
            let parent_win = dialog.clone();
            browse_file_btn.connect_clicked(move |_| {
                let chooser = gtk4::FileChooserDialog::builder()
                    .title("Select File")
                    .action(gtk4::FileChooserAction::Open)
                    .transient_for(&parent_win)
                    .modal(true)
                    .build();
                chooser.add_button("Cancel", gtk4::ResponseType::Cancel);
                chooser.add_button("Select", gtk4::ResponseType::Accept);

                let entry_for_resp = entry.clone();
                let name_entry_for_resp = name_entry.clone();
                chooser.connect_response(move |dialog, response| {
                    if response == gtk4::ResponseType::Accept
                        && let Some(file) = dialog.file()
                            && let Some(path) = file.path() {
                                let path_str = path.to_string_lossy().to_string();
                                entry_for_resp.set_text(&path_str);
                                if name_entry_for_resp.text().trim().is_empty()
                                    && let Some(stem) = path.file_name()
                                {
                                    name_entry_for_resp.set_text(&stem.to_string_lossy());
                                }
                            }
                    dialog.close();
                });
                chooser.show();
            });
        }
        file_input_box.append(&browse_file_btn);
        file_page.append(&file_input_box);

        stack.add_named(&file_page, Some("file"));

        // 5. Binary Tab
        let bin_page = GtkBox::new(Orientation::Vertical, 12);
        let bin_name_lbl = Label::new(Some("Display Name"));
        bin_name_lbl.set_halign(Align::Start);
        bin_name_lbl.add_css_class("setting-label");
        bin_page.append(&bin_name_lbl);
        let bin_name_entry = Entry::new();
        bin_name_entry.set_placeholder_text(Some("e.g. Build Script, Custom Binary"));
        bin_page.append(&bin_name_entry);

        let bin_target_lbl = Label::new(Some("Executable Binary Path"));
        bin_target_lbl.set_halign(Align::Start);
        bin_target_lbl.add_css_class("setting-label");
        bin_page.append(&bin_target_lbl);

        let bin_input_box = GtkBox::new(Orientation::Horizontal, 8);
        let bin_target_entry = Entry::new();
        bin_target_entry.set_hexpand(true);
        bin_target_entry.set_placeholder_text(Some("/usr/local/bin/my_tool or script path"));
        bin_input_box.append(&bin_target_entry);

        let browse_bin_btn = Button::with_label("Browse...");
        browse_bin_btn.add_css_class("action-btn");
        {
            let entry = bin_target_entry.clone();
            let name_entry = bin_name_entry.clone();
            let parent_win = dialog.clone();
            browse_bin_btn.connect_clicked(move |_| {
                let chooser = gtk4::FileChooserDialog::builder()
                    .title("Select Executable Binary")
                    .action(gtk4::FileChooserAction::Open)
                    .transient_for(&parent_win)
                    .modal(true)
                    .build();
                chooser.add_button("Cancel", gtk4::ResponseType::Cancel);
                chooser.add_button("Select", gtk4::ResponseType::Accept);

                let entry_for_resp = entry.clone();
                let name_entry_for_resp = name_entry.clone();
                chooser.connect_response(move |dialog, response| {
                    if response == gtk4::ResponseType::Accept
                        && let Some(file) = dialog.file()
                            && let Some(path) = file.path() {
                                let path_str = path.to_string_lossy().to_string();
                                entry_for_resp.set_text(&path_str);
                                if name_entry_for_resp.text().trim().is_empty()
                                    && let Some(stem) = path.file_name()
                                {
                                    name_entry_for_resp.set_text(&stem.to_string_lossy());
                                }
                            }
                    dialog.close();
                });
                chooser.show();
            });
        }
        bin_input_box.append(&browse_bin_btn);
        bin_page.append(&bin_input_box);

        stack.add_named(&bin_page, Some("binary"));

        // Switch tabs on dropdown change
        let stack_for_dd = stack.clone();
        kind_dropdown.connect_selected_notify(move |dd| {
            match dd.selected() {
                0 => stack_for_dd.set_visible_child_name("app"),
                1 => stack_for_dd.set_visible_child_name("url"),
                2 => stack_for_dd.set_visible_child_name("folder"),
                3 => stack_for_dd.set_visible_child_name("file"),
                4 => stack_for_dd.set_visible_child_name("binary"),
                _ => {}
            }
        });

        box_root.append(&stack);

        // Buttons
        let btn_box = GtkBox::new(Orientation::Horizontal, 10);
        btn_box.set_halign(Align::End);

        let cancel_btn = Button::with_label("Cancel");
        cancel_btn.add_css_class("action-btn");
        {
            let dlg = dialog.clone();
            cancel_btn.connect_clicked(move |_| dlg.close());
        }
        btn_box.append(&cancel_btn);

        let add_btn = Button::with_label("Add to Workspace");
        add_btn.add_css_class("action-btn-primary");
        {
            let win = win.clone();
            let dlg = dialog.clone();
            let kind_dd = kind_dropdown.clone();
            let app_list = app_list_box.clone();
            let cached_apps_list = cached_apps.clone();
            let url_name_e = url_name_entry.clone();
            let url_target_e = url_target_entry.clone();
            let folder_name_e = folder_name_entry.clone();
            let folder_target_e = folder_target_entry.clone();
            let file_name_e = file_name_entry.clone();
            let file_target_e = file_target_entry.clone();
            let bin_name_e = bin_name_entry.clone();
            let bin_target_e = bin_target_entry.clone();

            add_btn.connect_clicked(move |_| {
                let kind_idx = kind_dd.selected();
                let item_opt = match kind_idx {
                    0 => {
                        // App
                        if let Some(sel_row) = app_list.selected_row() {
                            let idx = sel_row.index() as usize;
                            cached_apps_list.get(idx).map(|app| WorkspaceItemConfig {
                                    name: app.name.clone(),
                                    kind: "app".to_string(),
                                    target: app.id.clone(),
                                    order: 0,
                                    icon: None,
                                })
                        } else {
                            None
                        }
                    }
                    1 => {
                        // URL
                        let name = url_name_e.text().trim().to_string();
                        let target = url_target_e.text().trim().to_string();
                        if !name.is_empty() && !target.is_empty() {
                            Some(WorkspaceItemConfig {
                                name,
                                kind: "url".to_string(),
                                target,
                                order: 0,
                                icon: None,
                            })
                        } else {
                            None
                        }
                    }
                    2 => {
                        // Folder
                        let name = folder_name_e.text().trim().to_string();
                        let target = folder_target_e.text().trim().to_string();
                        if !name.is_empty() && !target.is_empty() {
                            Some(WorkspaceItemConfig {
                                name,
                                kind: "folder".to_string(),
                                target,
                                order: 0,
                                icon: None,
                            })
                        } else {
                            None
                        }
                    }
                    3 => {
                        // File
                        let name = file_name_e.text().trim().to_string();
                        let target = file_target_e.text().trim().to_string();
                        if !target.is_empty() {
                            let display_name = if name.is_empty() {
                                Path::new(&target)
                                    .file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "File".to_string())
                            } else {
                                name
                            };
                            Some(WorkspaceItemConfig {
                                name: display_name,
                                kind: "file".to_string(),
                                target,
                                order: 0,
                                icon: None,
                            })
                        } else {
                            None
                        }
                    }
                    4 => {
                        // Binary
                        let name = bin_name_e.text().trim().to_string();
                        let target = bin_target_e.text().trim().to_string();
                        if !target.is_empty() {
                            let display_name = if name.is_empty() {
                                Path::new(&target)
                                    .file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "Binary".to_string())
                            } else {
                                name
                            };
                            Some(WorkspaceItemConfig {
                                name: display_name,
                                kind: "binary".to_string(),
                                target,
                                order: 0,
                                icon: None,
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(mut item) = item_opt {
                    let mut cfg = win.config.borrow_mut();
                    if let Some(ws) = cfg.workspace.get_mut(ws_idx) {
                        item.order = ws.items.len();
                        ws.items.push(item);
                        drop(cfg);
                        win.notify_saved("Added item to workspace");
                        on_saved();
                        dlg.close();
                    }
                }
            });
        }
        btn_box.append(&add_btn);
        box_root.append(&btn_box);

        dialog.set_child(Some(&box_root));
        dialog.present();
    }

    fn build_advanced_page(settings_win: &Rc<Self>) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 0);
        page.append(&Self::create_header_section(
            "Advanced",
            "Low-level system, terminal, file manager, and debugging options.",
        ));

        let group1_lbl = Label::new(Some("System Launch Handlers"));
        group1_lbl.set_halign(Align::Start);
        group1_lbl.add_css_class("group-heading");
        page.append(&group1_lbl);

        let card1 = GtkBox::new(Orientation::Vertical, 0);
        card1.add_css_class("settings-card");

        // Terminal override
        let term_entry = Entry::new();
        term_entry.set_text(&settings_win.config.borrow().advanced.terminal_override);
        term_entry.set_placeholder_text(Some("Auto-detect (e.g. kitty, foot, alacritty)"));
        term_entry.set_width_chars(25);
        {
            let win = settings_win.clone();
            term_entry.connect_changed(move |e| {
                win.config.borrow_mut().advanced.terminal_override = e.text().to_string();
                win.notify_saved("Terminal override updated");
            });
        }
        card1.append(&Self::create_card_row(
            "Terminal Emulator Override",
            "Override terminal emulator used for Terminal=true applications (defaults to auto-detect)",
            &term_entry,
        ));

        // File Manager override
        let fm_entry = Entry::new();
        fm_entry.set_text(&settings_win.config.borrow().advanced.file_manager_override);
        let detected_fm = crate::apps::detect_file_manager().unwrap_or_else(|| "xdg-open".to_string());
        let fm_placeholder = format!("Auto-detected: {} (or override: dolphin, nautilus, thunar)", detected_fm);
        fm_entry.set_placeholder_text(Some(&fm_placeholder));
        fm_entry.set_width_chars(25);
        {
            let win = settings_win.clone();
            fm_entry.connect_changed(move |e| {
                win.config.borrow_mut().advanced.file_manager_override = e.text().to_string();
                win.notify_saved("File manager override updated");
            });
        }
        card1.append(&Self::create_card_row(
            "File Manager Override",
            "Override file manager used for folder launches (protects against system MIME hijacking)",
            &fm_entry,
        ));
        page.append(&card1);

        let group2_lbl = Label::new(Some("Diagnostics, Network & Management"));
        group2_lbl.set_halign(Align::Start);
        group2_lbl.add_css_class("group-heading");
        page.append(&group2_lbl);

        let card2 = GtkBox::new(Orientation::Vertical, 0);
        card2.add_css_class("settings-card");

        // Auto-fetch favicons switch
        let sw_fav = Switch::builder()
            .active(settings_win.config.borrow().advanced.auto_fetch_favicons)
            .build();
        {
            let win = settings_win.clone();
            sw_fav.connect_active_notify(move |sw| {
                win.config.borrow_mut().advanced.auto_fetch_favicons = sw.is_active();
                win.notify_saved("Favicon fetch setting saved");
            });
        }
        card2.append(&Self::create_card_row(
            "Auto-Fetch Website Favicons",
            "Automatically resolve and cache high-resolution favicons for URL items in background",
            &sw_fav,
        ));

        // Debug logging
        let sw_dbg = Switch::builder()
            .active(settings_win.config.borrow().advanced.debug_logging)
            .build();
        {
            let win = settings_win.clone();
            sw_dbg.connect_active_notify(move |sw| {
                win.config.borrow_mut().advanced.debug_logging = sw.is_active();
                win.notify_saved("Logging setting saved");
            });
        }
        card2.append(&Self::create_card_row(
            "Debug Logging",
            "Log verbose compositor, evdev, and discovery events to stdout",
            &sw_dbg,
        ));

        // Inotify live watcher
        let sw_watch = Switch::builder()
            .active(settings_win.config.borrow().advanced.live_watcher)
            .build();
        {
            let win = settings_win.clone();
            sw_watch.connect_active_notify(move |sw| {
                win.config.borrow_mut().advanced.live_watcher = sw.is_active();
                win.notify_saved("Watcher setting saved");
            });
        }
        card2.append(&Self::create_card_row(
            "Live Desktop File Inotify Watcher",
            "Automatically detect new, changed, or deleted application .desktop files in real-time",
            &sw_watch,
        ));

        // Reload apps cache button
        let reload_btn = Button::with_label("Reload Application Cache");
        reload_btn.add_css_class("action-btn");
        {
            let win = settings_win.clone();
            reload_btn.connect_clicked(move |_| {
                let fresh_apps = apps::discover_applications();
                *win.cached_apps.borrow_mut() = fresh_apps;
                match ipc::send_command("RELOAD") {
                    Ok(resp) => {
                        win.notify_saved(&format!("Cache reloaded ({})", resp.trim()));
                    }
                    Err(_) => {
                        win.notify_saved("Daemon not running - saved to disk");
                    }
                }
            });
        }
        card2.append(&Self::create_card_row(
            "Rescan Applications",
            "Force an immediate rescan of all XDG application directories and icon themes",
            &reload_btn,
        ));
        page.append(&card2);

        page
    }
}
