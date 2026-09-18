//! Layer-shell overlay window implementation for Hyprvyl.
//!
//! Creates a transparent, unmanaged, unbordered overlay window using the
//! `wlr-layer-shell` protocol via `gtk4-layer-shell`. The overlay is anchored to all
//! 4 edges of the target monitor to provide a canvas for cursor-positioned radial arc menus.
//!
//! Features:
//! - Radial Arc Layout: Individual floating rounded-square icon buttons along a polar arc centered on the cursor.
//! - Two-Level Navigation Hierarchy: Workspaces -> Items (Apps, URLs, Folders).
//! - Dynamic arc orientation & edge-of-screen boundary clamping.
//! - Live motion tracking, 5% hover scaling, high-contrast inverted hover state with adjacent pill label.
//! - Click and Hold-Release selection triggers.
//! - Live type-to-filter with breadcrumb title & page indicator.
//! - Circular back button and Escape cancellation.

use crate::apps::{self, AppEntry};
use crate::config::Config;
use crate::hyprland::{self, CursorPosition, HyprMonitor};
use crate::renderer::{self, DisplayItem, LauncherRenderState};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, CssProvider, DrawingArea, EventControllerKey,
    EventControllerMotion, EventControllerScroll, EventControllerScrollFlags, GestureClick,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

/// Encapsulates the overlay window and its interactive state.
pub struct OverlayWindow {
    window: ApplicationWindow,
    drawing_area: DrawingArea,
    state: Rc<RefCell<LauncherRenderState>>,
    is_shown: Rc<RefCell<bool>>,
    apps: Rc<RefCell<Vec<AppEntry>>>,
    config: Rc<RefCell<Config>>,
    active_workspace_id: Rc<RefCell<Option<String>>>,
    current_items: Rc<RefCell<Vec<DisplayItem>>>,
    app: Application,
    settings_win: Rc<RefCell<Option<Rc<crate::settings::SettingsWindow>>>>,
}

impl OverlayWindow {
    /// Creates and configures a new Layer Shell Overlay window.
    pub fn new(app: &Application, config: Config) -> Rc<Self> {
        // 1. Create a non-decorated ApplicationWindow
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Hyprvyl Overlay")
            .decorated(false)
            .build();

        // 2. Attach CSS provider for full window alpha transparency
        let css_provider = CssProvider::new();
        css_provider.load_from_data(
            "window.hyprvyl-window, window.hyprvyl-window:backdrop {\
                background-color: transparent;\
                background: transparent;\
                box-shadow: none;\
                border: none;\
            }\
            drawingarea.hyprvyl-canvas {\
                background-color: transparent;\
                background: transparent;\
            }"
        );

        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &css_provider,
                STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        window.add_css_class("hyprvyl-window");

        // 3. Initialize the Wayland Layer Shell surface
        window.init_layer_shell();

        // Set Layer to Overlay (above normal windows, fullscreen apps, and status bars)
        window.set_layer(Layer::Overlay);

        // Namespace for identification in compositor rules / logs
        window.set_namespace(Some("hyprvyl"));

        // Anchor to all 4 edges to span the full active monitor
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);

        // Exclusive zone = -1 ensures the window does not reserve space or move tiled windows
        window.set_exclusive_zone(-1);

        // Keyboard interactivity: initially None when hidden
        window.set_keyboard_mode(KeyboardMode::None);

        let mut initial_state = LauncherRenderState::default();
        initial_state.layout = config.appearance.layout;
        initial_state.button_size = config.appearance.button_size;
        initial_state.corner_radius = config.appearance.corner_radius;
        initial_state.arc_radius = config.appearance.arc_radius;
        initial_state.arc_span_degrees = config.appearance.arc_span_degrees;
        initial_state.items_per_page = config.appearance.items_per_page;

        let state = Rc::new(RefCell::new(initial_state));
        let is_shown = Rc::new(RefCell::new(false));
        let apps = Rc::new(RefCell::new(Vec::new()));
        let config_cell = Rc::new(RefCell::new(config));
        let active_workspace_id = Rc::new(RefCell::new(None));
        let current_items = Rc::new(RefCell::new(Vec::new()));
        let settings_win = Rc::new(RefCell::new(None));

        // 4. Create DrawingArea for Cairo vector rendering
        let drawing_area = DrawingArea::new();
        drawing_area.add_css_class("hyprvyl-canvas");
        drawing_area.set_hexpand(true);
        drawing_area.set_vexpand(true);

        let draw_state = state.clone();
        let draw_items = current_items.clone();
        drawing_area.set_draw_func(move |_area, cr, width, height| {
            let st = draw_state.borrow();
            let items = draw_items.borrow();
            renderer::draw_radial_overlay(cr, &st, &items, width, height);
        });

        window.set_child(Some(&drawing_area));

        let overlay = Rc::new(Self {
            window,
            drawing_area,
            state,
            is_shown,
            apps,
            config: config_cell,
            active_workspace_id,
            current_items,
            app: app.clone(),
            settings_win,
        });

        // 5. Setup mouse, scroll, and keyboard event controllers
        overlay.setup_event_controllers();

        // Keep window hidden initially until triggered
        overlay.window.set_visible(false);

        overlay
    }

    /// Hides the radial overlay and presents the unified Settings window.
    pub fn open_settings(&self) {
        self.hide();
        let mut win_opt = self.settings_win.borrow_mut();
        if let Some(ref win) = *win_opt {
            win.present();
        } else {
            let win = crate::settings::SettingsWindow::new(&self.app);
            win.present();
            *win_opt = Some(win);
        }
    }

    /// Updates the current configuration.
    pub fn update_config(&self, new_config: Config) {
        let mut st = self.state.borrow_mut();
        st.layout = new_config.appearance.layout;
        st.button_size = new_config.appearance.button_size;
        st.corner_radius = new_config.appearance.corner_radius;
        st.arc_radius = new_config.appearance.arc_radius;
        st.arc_span_degrees = new_config.appearance.arc_span_degrees;
        st.items_per_page = new_config.appearance.items_per_page;
        *self.config.borrow_mut() = new_config;
        drop(st);
        self.refresh_current_items();
        self.drawing_area.queue_draw();
    }

    /// Updates the in-memory cached application entries.
    pub fn set_apps(&self, apps: Vec<AppEntry>) {
        *self.apps.borrow_mut() = apps;
        self.refresh_current_items();
        self.drawing_area.queue_draw();
    }

    /// Retrieves a cloned list of the cached application entries.
    #[allow(dead_code)]
    pub fn get_apps(&self) -> Vec<AppEntry> {
        self.apps.borrow().clone()
    }

    /// Returns the number of cached application entries.
    #[allow(dead_code)]
    pub fn get_app_count(&self) -> usize {
        self.apps.borrow().len()
    }

    /// Refreshes `current_items` and state based on whether we are at root Workspaces or inside a Workspace.
    pub fn refresh_current_items(&self) {
        let cfg = self.config.borrow();
        let apps = self.apps.borrow();
        let icon_size = cfg.appearance.icon_size;
        let active_ws = self.active_workspace_id.borrow().clone();

        let mut items = Vec::new();
        let breadcrumb;
        let is_sublevel;
        let ws_index;
        let total_workspaces = cfg.workspace.len().max(1);

        let auto_fetch = cfg.advanced.auto_fetch_favicons;

        match active_ws {
            None => {
                // Level 1: Root Workspaces list (or default first workspace items if only 1 workspace)
                breadcrumb = cfg.workspace.first().map(|w| w.name.clone()).unwrap_or_else(|| "Main".to_string());
                is_sublevel = false;
                ws_index = 1;

                let mut ws_list = cfg.workspace.clone();
                ws_list.sort_by_key(|w| w.order);

                for ws in ws_list {
                    let (icon_path, icon_name) = apps::resolve_kind_icon(
                        "workspace",
                        &ws.icon,
                        None,
                        &apps,
                        icon_size,
                        auto_fetch,
                    );
                    items.push(DisplayItem {
                        id: ws.id,
                        name: ws.name,
                        kind: "workspace".to_string(),
                        target: ws.icon,
                        icon_path,
                        icon_name,
                    });
                }
            }
            Some(ref ws_id) => {
                // Level 2: Workspace Items list
                if let Some((idx, ws)) = cfg.workspace.iter().enumerate().find(|(_, w)| &w.id == ws_id) {
                    breadcrumb = ws.name.clone();
                    is_sublevel = true;
                    ws_index = idx + 1;

                    let mut item_list = ws.items.clone();
                    item_list.sort_by_key(|it| it.order);

                    for it in item_list {
                        let (icon_path, icon_name) = apps::resolve_kind_icon(
                            &it.kind,
                            &it.target,
                            it.icon.as_deref(),
                            &apps,
                            icon_size,
                            auto_fetch,
                        );
                        items.push(DisplayItem {
                            id: it.name.clone(),
                            name: it.name,
                            kind: it.kind,
                            target: it.target,
                            icon_path,
                            icon_name,
                        });
                    }
                } else {
                    // Fallback to root if workspace not found
                    *self.active_workspace_id.borrow_mut() = None;
                    drop(cfg);
                    drop(apps);
                    self.refresh_current_items();
                    return;
                }
            }
        }

        let mut st = self.state.borrow_mut();
        st.breadcrumb_title = breadcrumb;
        st.is_sublevel = is_sublevel;
        st.workspace_index = ws_index;
        st.total_workspaces = total_workspaces;

        if st.search_query.is_empty() {
            st.reset_filter(&items);
        } else {
            st.update_filter(&items);
        }

        *self.current_items.borrow_mut() = items;
    }

    /// Handles selection/activation of a DisplayItem.
    fn handle_item_selection(&self, item: &DisplayItem) {
        match item.kind.as_str() {
            "workspace" => {
                println!("[Hyprvyl] Selected workspace '{}' (ID: {})", item.name, item.id);
                *self.active_workspace_id.borrow_mut() = Some(item.id.clone());
                self.refresh_current_items();
                self.drawing_area.queue_draw();
            }
            "app" => {
                println!("[Hyprvyl] Launching app item '{}' (Target: {})", item.name, item.target);
                let apps = self.apps.borrow();
                let fm_override = self.config.borrow().advanced.file_manager_override.clone();
                let _ = apps::launch_workspace_item("app", &item.target, Some(&fm_override), &apps);
                let close = self.config.borrow().general.close_on_launch;
                if close {
                    self.hide();
                }
            }
            "url" => {
                println!("[Hyprvyl] Opening URL item '{}' (Target: {})", item.name, item.target);
                let _ = apps::launch_url(&item.target);
                let close = self.config.borrow().general.close_on_launch;
                if close {
                    self.hide();
                }
            }
            "folder" => {
                let fm_override = self.config.borrow().advanced.file_manager_override.clone();
                println!(
                    "[Hyprvyl] Opening folder item '{}' (Target: {}, Override: {:?})",
                    item.name, item.target, fm_override
                );
                let _ = apps::launch_folder(&item.target, Some(&fm_override));
                let close = self.config.borrow().general.close_on_launch;
                if close {
                    self.hide();
                }
            }
            "file" => {
                println!("[Hyprvyl] Opening file item '{}' (Target: {})", item.name, item.target);
                let _ = apps::launch_file(&item.target);
                let close = self.config.borrow().general.close_on_launch;
                if close {
                    self.hide();
                }
            }
            "binary" | "bin" | "executable" => {
                println!("[Hyprvyl] Launching binary item '{}' (Target: {})", item.name, item.target);
                let _ = apps::launch_binary(&item.target);
                let close = self.config.borrow().general.close_on_launch;
                if close {
                    self.hide();
                }
            }
            _ => {
                let apps = self.apps.borrow();
                let fm_override = self.config.borrow().advanced.file_manager_override.clone();
                let _ = apps::launch_workspace_item(&item.kind, &item.target, Some(&fm_override), &apps);
                let close = self.config.borrow().general.close_on_launch;
                if close {
                    self.hide();
                }
            }
        }
    }

    /// Navigates back to the root workspace list, or closes the overlay if already at root.
    pub fn navigate_back(&self) {
        if self.active_workspace_id.borrow().is_some() {
            println!("[Hyprvyl] Back triggered -> returning to Workspace list");
            *self.active_workspace_id.borrow_mut() = None;
            self.refresh_current_items();
            self.drawing_area.queue_draw();
        } else {
            self.hide();
        }
    }

    /// Sets up mouse motion, click gestures, scroll wheel, and keyboard listeners.
    fn setup_event_controllers(self: &Rc<Self>) {
        let overlay_weak = Rc::downgrade(self);

        // 1. Motion Controller: tracks pointer position and highlights matching button live
        let motion_controller = EventControllerMotion::new();
        {
            let overlay_weak = overlay_weak.clone();
            motion_controller.connect_motion(move |_controller, x, y| {
                if let Some(overlay) = overlay_weak.upgrade() {
                    let mut st = overlay.state.borrow_mut();
                    let items = overlay.current_items.borrow();
                    let page_entries = st.get_current_page_entries(&items);

                    let new_center_hovered = renderer::is_center_hub_hovered(x, y, &st);
                    let new_hovered = renderer::get_hovered_button_for_point(x, y, &st, page_entries.len());
                    let new_main_hovered = renderer::is_main_button_hovered(x, y, &st);
                    let new_back_hovered = renderer::is_back_button_hovered(x, y, &st);

                    let mut need_redraw = false;
                    if st.hovered_center != new_center_hovered {
                        st.hovered_center = new_center_hovered;
                        need_redraw = true;
                    }
                    if st.hovered_index != new_hovered {
                        st.hovered_index = new_hovered;
                        need_redraw = true;
                    }
                    if st.hovered_main != new_main_hovered {
                        st.hovered_main = new_main_hovered;
                        need_redraw = true;
                    }
                    if st.hovered_back != new_back_hovered {
                        st.hovered_back = new_back_hovered;
                        need_redraw = true;
                    }

                    if need_redraw {
                        overlay.drawing_area.queue_draw();
                    }
                }
            });
        }
        self.drawing_area.add_controller(motion_controller);

        // 2. Click Gesture: handles selection click, center hub click, or click-outside dismissal
        let click_gesture = GestureClick::new();
        {
            let overlay_weak = overlay_weak.clone();
            click_gesture.connect_pressed(move |_gesture, _n_press, x, y| {
                if let Some(overlay) = overlay_weak.upgrade() {
                    let st = overlay.state.borrow();
                    let items = overlay.current_items.borrow();
                    let page_entries = st.get_current_page_entries(&items);

                    // Check if center hub was clicked -> open Settings
                    if renderer::is_center_hub_hovered(x, y, &st) {
                        drop(st);
                        drop(items);
                        println!("[Hyprvyl] Center hub clicked -> Opening Settings");
                        overlay.open_settings();
                        return;
                    }

                    // Check if bottom left "Main" / "Hyprvyl" button was clicked
                    if renderer::is_main_button_hovered(x, y, &st) {
                        let is_sub = st.is_sublevel;
                        drop(st);
                        drop(items);
                        if is_sub {
                            overlay.navigate_back();
                        } else {
                            overlay.center_on_screen();
                        }
                        return;
                    }

                    // Check if bottom right "Back" / "Center" button was clicked
                    if renderer::is_back_button_hovered(x, y, &st) {
                        let is_sub = st.is_sublevel;
                        drop(st);
                        drop(items);
                        if is_sub {
                            overlay.navigate_back();
                        } else {
                            // "Center" was clicked -> center wheel on screen!
                            overlay.center_on_screen();
                        }
                        return;
                    }

                    // Check if an item button was clicked
                    let hovered = renderer::get_hovered_button_for_point(x, y, &st, page_entries.len());
                    if let Some(idx) = hovered
                        && let Some(item) = page_entries.get(idx) {
                            let item_cloned = (*item).clone();
                            drop(st);
                            drop(items);
                            overlay.handle_item_selection(&item_cloned);
                            return;
                        }

                    // Clicked outside radial menu -> dismiss
                    println!("[Hyprvyl] Clicked outside radial menu at ({:.1}, {:.1}) -> Dismissing", x, y);
                    drop(st);
                    drop(items);
                    overlay.hide();
                }
            });
        }
        self.drawing_area.add_controller(click_gesture);

        // 3. Scroll Controller: cycles through item pages smoothly
        let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL | EventControllerScrollFlags::HORIZONTAL);
        {
            let overlay_weak = overlay_weak.clone();
            scroll_controller.connect_scroll(move |_controller, _dx, dy| {
                if let Some(overlay) = overlay_weak.upgrade() {
                    let mut st = overlay.state.borrow_mut();
                    if dy > 0.0 {
                        st.next_page();
                        overlay.drawing_area.queue_draw();
                    } else if dy < 0.0 {
                        st.prev_page();
                        overlay.drawing_area.queue_draw();
                    }
                }
                glib::Propagation::Stop
            });
        }
        self.drawing_area.add_controller(scroll_controller);

        // 4. Key Controller: handles Live Search / Filtering, Number key shortcuts, Navigation, Launch, and Dismissal
        let key_controller = EventControllerKey::new();
        {
            let overlay_weak = overlay_weak.clone();
            key_controller.connect_key_pressed(move |_controller, keyval, _keycode, _state| {
                if let Some(overlay) = overlay_weak.upgrade() {
                    // Escape: clear search query if set, else navigate back / dismiss
                    if keyval == gdk::Key::Escape {
                        let mut st = overlay.state.borrow_mut();
                        if !st.search_query.is_empty() {
                            let items = overlay.current_items.borrow();
                            st.reset_filter(&items);
                            overlay.drawing_area.queue_draw();
                        } else {
                            drop(st);
                            overlay.navigate_back();
                        }
                        return glib::Propagation::Stop;
                    }

                    // Enter / Return / KP_Enter: launch currently hovered or top match
                    if keyval == gdk::Key::Return || keyval == gdk::Key::KP_Enter {
                        let st = overlay.state.borrow();
                        let items = overlay.current_items.borrow();
                        let item_to_launch = if let Some(item) = st.get_hovered_entry(&items) {
                            Some((*item).clone())
                        } else if let Some(&first_idx) = st.filtered_indices.first() {
                            items.get(first_idx).cloned()
                        } else {
                            None
                        };

                        drop(st);
                        drop(items);

                        if let Some(item) = item_to_launch {
                            overlay.handle_item_selection(&item);
                        }
                        return glib::Propagation::Stop;
                    }

                    // Number keys 1..9 for instant workspace switching (when enabled in settings and not searching)
                    if let Some(ch) = keyval.to_unicode()
                        && ch.is_ascii_digit() && ch != '0' {
                            let is_empty_search = overlay.state.borrow().search_query.is_empty();
                            let is_keys_mode = overlay.config.borrow().workspaces.workspace_switching == "keys";
                            if is_empty_search && is_keys_mode {
                                let digit = (ch as u8 - b'1') as usize;
                                let cfg = overlay.config.borrow();
                                if let Some(ws) = cfg.workspace.get(digit) {
                                    let ws_id = ws.id.clone();
                                    drop(cfg);
                                    *overlay.active_workspace_id.borrow_mut() = Some(ws_id);
                                    overlay.refresh_current_items();
                                    overlay.drawing_area.queue_draw();
                                    return glib::Propagation::Stop;
                                }
                            }
                        }

                    // Backspace: remove last character from search query
                    if keyval == gdk::Key::BackSpace {
                        let mut st = overlay.state.borrow_mut();
                        if !st.search_query.is_empty() {
                            st.search_query.pop();
                            let items = overlay.current_items.borrow();
                            st.update_filter(&items);
                            overlay.drawing_area.queue_draw();
                        }
                        return glib::Propagation::Stop;
                    }

                    // Down / Tab: navigate selection forward
                    if keyval == gdk::Key::Down || keyval == gdk::Key::Tab {
                        let mut st = overlay.state.borrow_mut();
                        let items = overlay.current_items.borrow();
                        let page_entries = st.get_current_page_entries(&items);
                        let count = page_entries.len();
                        if count > 0 {
                            let next = match st.hovered_index {
                                Some(idx) => (idx + 1) % count,
                                None => 0,
                            };
                            st.hovered_index = Some(next);
                            overlay.drawing_area.queue_draw();
                        }
                        return glib::Propagation::Stop;
                    }

                    // Up / Shift+Tab: navigate selection backward
                    if keyval == gdk::Key::Up || keyval == gdk::Key::ISO_Left_Tab {
                        let mut st = overlay.state.borrow_mut();
                        let items = overlay.current_items.borrow();
                        let page_entries = st.get_current_page_entries(&items);
                        let count = page_entries.len();
                        if count > 0 {
                            let prev = match st.hovered_index {
                                Some(0) | None => count - 1,
                                Some(idx) => idx - 1,
                            };
                            st.hovered_index = Some(prev);
                            overlay.drawing_area.queue_draw();
                        }
                        return glib::Propagation::Stop;
                    }

                    // Right / Page_Down: navigate next page
                    if keyval == gdk::Key::Right || keyval == gdk::Key::Page_Down {
                        let mut st = overlay.state.borrow_mut();
                        st.next_page();
                        overlay.drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }

                    // Left / Page_Up: navigate prev page
                    if keyval == gdk::Key::Left || keyval == gdk::Key::Page_Up {
                        let mut st = overlay.state.borrow_mut();
                        st.prev_page();
                        overlay.drawing_area.queue_draw();
                        return glib::Propagation::Stop;
                    }

                    // Type-to-Filter: capture printable character keys
                    if let Some(ch) = keyval.to_unicode()
                        && !ch.is_control() {
                            let mut st = overlay.state.borrow_mut();
                            st.search_query.push(ch);
                            let items = overlay.current_items.borrow();
                            st.update_filter(&items);
                            overlay.drawing_area.queue_draw();
                            return glib::Propagation::Stop;
                        }
                }
                glib::Propagation::Proceed
            });
        }
        self.window.add_controller(key_controller);
    }

    /// Handles Hold-Release trigger event from evdev listener or external IPC.
    pub fn handle_trigger_release(&self) {
        if !self.is_visible() {
            return;
        }

        let st = self.state.borrow();
        let items = self.current_items.borrow();

        if st.hovered_center {
            drop(st);
            drop(items);
            println!("[Hyprvyl Hold-Release] Released on Center Hub -> Opening Settings");
            self.open_settings();
        } else if let Some(item) = st.get_hovered_entry(&items) {
            let item_cloned = (*item).clone();
            drop(st);
            drop(items);
            println!("[Hyprvyl Hold-Release] Selected '{}' (ID: {})", item_cloned.name, item_cloned.id);
            self.handle_item_selection(&item_cloned);
        } else {
            // Released outside any button -> cancel cleanly
            println!("[Hyprvyl Hold-Release] Released outside button -> Closing overlay");
            drop(st);
            drop(items);
            self.hide();
        }
    }

    /// Shows the overlay on the monitor where the cursor currently resides.
    pub fn show_at_cursor(&self) {
        let cursor_opt = hyprland::get_cursor_position();
        let monitors = hyprland::get_monitors();
        let mon = if let Some(cur) = cursor_opt {
            hyprland::find_monitor_for_cursor(cur, &monitors)
                .cloned()
                .unwrap_or_else(|| monitors.first().cloned().unwrap())
        } else {
            monitors
                .iter()
                .find(|m| m.focused)
                .cloned()
                .unwrap_or_else(|| monitors.first().cloned().unwrap())
        };

        self.bind_to_monitor(&mon);

        // Always open at root level (Workspaces list) with clean search filter
        *self.active_workspace_id.borrow_mut() = None;
        self.refresh_current_items();

        let mut st = self.state.borrow_mut();
        let mon_w = (mon.width as f64 / mon.scale).round();
        let mon_h = (mon.height as f64 / mon.scale).round();

        // Default location: centered on the active monitor
        st.center_x = mon_w / 2.0;
        st.center_y = mon_h / 2.0;

        let items = self.current_items.borrow();
        let page_entries = st.get_current_page_entries(&items);
        let (eval_x, eval_y) = if let Some(cur) = cursor_opt {
            ((cur.x - mon.x) as f64, (cur.y - mon.y) as f64)
        } else {
            (st.center_x, st.center_y)
        };

        st.hovered_center = renderer::is_center_hub_hovered(eval_x, eval_y, &st);
        st.hovered_index = renderer::get_hovered_button_for_point(eval_x, eval_y, &st, page_entries.len());
        st.hovered_main = renderer::is_main_button_hovered(eval_x, eval_y, &st);
        st.hovered_back = renderer::is_back_button_hovered(eval_x, eval_y, &st);

        drop(st);
        drop(items);

        // Enable exclusive keyboard mode so user typing immediately goes to live filter
        self.window.set_keyboard_mode(KeyboardMode::Exclusive);

        // Make visible and queue draw
        self.window.set_visible(true);
        self.window.present();
        self.drawing_area.queue_draw();

        *self.is_shown.borrow_mut() = true;
        println!("[Hyprvyl] Radial overlay opened at pos ({:.1}, {:.1})", self.state.borrow().center_x, self.state.borrow().center_y);
    }

    /// Dynamically re-centers the radial wheel in the exact middle of the active monitor.
    pub fn center_on_screen(&self) {
        let cursor = hyprland::get_cursor_position().unwrap_or(CursorPosition { x: 500, y: 500 });
        let monitors = hyprland::get_monitors();
        let target_monitor = hyprland::find_monitor_for_cursor(cursor, &monitors);

        if let Some(mon) = target_monitor {
            let mon_w = (mon.width as f64 / mon.scale).round();
            let mon_h = (mon.height as f64 / mon.scale).round();

            let mut st = self.state.borrow_mut();
            st.center_x = mon_w / 2.0;
            st.center_y = mon_h / 2.0;
            st.hovered_index = None;
            st.hovered_center = false;
            st.hovered_main = false;
            st.hovered_back = false;
            drop(st);

            self.drawing_area.queue_draw();
            println!("[Hyprvyl] Re-centered wheel to screen middle ({:.1}, {:.1})", mon_w / 2.0, mon_h / 2.0);
        }
    }

    /// Hides the overlay and releases all keyboard and pointer grabs (full click-through).
    pub fn hide(&self) {
        self.window.set_keyboard_mode(KeyboardMode::None);
        self.window.set_visible(false);
        *self.is_shown.borrow_mut() = false;
        println!("[Hyprvyl] Overlay hidden (pass-through active)");
    }

    /// Toggles the overlay state between visible and hidden.
    pub fn toggle(&self) {
        let currently_shown = *self.is_shown.borrow();
        if currently_shown {
            self.hide();
        } else {
            self.show_at_cursor();
        }
    }

    /// Checks if the overlay is currently visible.
    pub fn is_visible(&self) -> bool {
        *self.is_shown.borrow()
    }

    /// Associates the layer-shell window with a specific GDK / Wayland monitor.
    fn bind_to_monitor(&self, hypr_mon: &HyprMonitor) {
        if let Some(display) = gdk::Display::default() {
            let gdk_monitors = display.monitors();
            for i in 0..gdk_monitors.n_items() {
                if let Some(item) = gdk_monitors.item(i)
                    && let Ok(gdk_mon) = item.downcast::<gdk::Monitor>()
                        && let Some(connector) = gdk_mon.connector()
                            && connector == hypr_mon.name {
                                self.window.set_monitor(Some(&gdk_mon));
                                return;
                            }
            }
        }
    }
}
