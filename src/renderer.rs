//! Vector graphics renderer for the radial wheel application and workspace launcher menu.
//!
//! Uses Cairo for 2D anti-aliased vector drawing on a fully transparent GTK4 layer-shell surface.
//!
//! Target Design Specifications (Rovyl-Matched):
//! - Full 360° circular radial wheel centered on the trigger cursor (or optional radial arc).
//! - Center Hub Button (~56px diameter): dark circle with return arrow `↩` + dot `•` in sub-level, or Rovyl emblem in root.
//! - Top Workspace Pill Badge (e.g. `Main [ 1 ]` with white background and dark typography).
//! - Floating rounded-square squircle buttons:
//!   - Unhovered: `rgba(28, 28, 30, 0.90)`, corner radius ~14px, subtle border & drop shadow.
//!   - Hovered: Inverted to crisp pure white `#ffffff`, 1.08x scale-up, full-color icon.
//!   - Adjacent Tooltip Pill: White `#ffffff` rounded pill with bold black label text (`#111111`).
//! - Bottom Segmented Switcher / Breadcrumb Pill (`[ Main | Back ]` or `[ Rovyl | Center ]`).
//! - Polar coordinate calculations and smooth gesture flick detection.

use crate::config::WheelLayout;
use gdk_pixbuf::Pixbuf;
use gtk4::cairo::{Context, FontSlant, FontWeight};
use gtk4::gdk::prelude::GdkCairoContextExt;
use std::cell::RefCell;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::path::{Path, PathBuf};

pub const DEFAULT_ITEMS_PER_PAGE: usize = 8;
pub const DEFAULT_BUTTON_SIZE: f64 = 56.0;
pub const DEFAULT_ARC_RADIUS: f64 = 140.0;
pub const DEFAULT_ARC_SPAN_DEGREES: f64 = 140.0;
pub const DEFAULT_CORNER_RADIUS: f64 = 14.0;
pub const CENTER_HUB_RADIUS: f64 = 28.0;
pub const PILL_HEIGHT: f64 = 34.0;
pub const BREADCRUMB_HEIGHT: f64 = 32.0;

/// Represents an item ready to be rendered on the launcher canvas.
#[derive(Debug, Clone)]
pub struct DisplayItem {
    pub id: String,
    pub name: String,
    pub kind: String, // "workspace", "app", "url", "folder"
    pub target: String,
    pub icon_path: Option<PathBuf>,
    #[allow(dead_code)]
    pub icon_name: String,
}

/// Orientation of the radial arc (when layout == WheelLayout::Arc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
#[allow(dead_code)]
pub enum ArcOrientation {
    /// Arc curves along the right side of the cursor (0° base)
    #[default]
    Right,
    /// Arc curves along the left side of the cursor (180° base)
    Left,
}


/// Holds the interactive render state for the radial launcher menu.
#[derive(Debug, Clone)]
pub struct LauncherRenderState {
    pub center_x: f64,
    pub center_y: f64,
    pub layout: WheelLayout,
    pub arc_radius: f64,
    pub arc_span_degrees: f64,
    pub orientation: ArcOrientation,
    pub button_size: f64,
    pub corner_radius: f64,
    pub items_per_page: usize,
    pub hovered_index: Option<usize>,
    pub hovered_center: bool,
    pub hovered_main: bool,
    pub hovered_back: bool,
    pub current_page: usize,
    pub search_query: String,
    pub filtered_indices: Vec<usize>,
    pub breadcrumb_title: String,
    pub workspace_index: usize,
    pub total_workspaces: usize,
    pub is_sublevel: bool,
    icon_cache: RefCell<HashMap<(PathBuf, i32), Option<Pixbuf>>>,
}

impl Default for LauncherRenderState {
    fn default() -> Self {
        Self {
            center_x: 500.0,
            center_y: 350.0,
            layout: WheelLayout::Wheel,
            arc_radius: DEFAULT_ARC_RADIUS,
            arc_span_degrees: DEFAULT_ARC_SPAN_DEGREES,
            orientation: ArcOrientation::Right,
            button_size: DEFAULT_BUTTON_SIZE,
            corner_radius: DEFAULT_CORNER_RADIUS,
            items_per_page: DEFAULT_ITEMS_PER_PAGE,
            hovered_index: None,
            hovered_center: false,
            hovered_main: false,
            hovered_back: false,
            current_page: 0,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            breadcrumb_title: "Main".to_string(),
            workspace_index: 1,
            total_workspaces: 1,
            is_sublevel: false,
            icon_cache: RefCell::new(HashMap::new()),
        }
    }
}

impl LauncherRenderState {
    /// Resets search filter and sets all items visible.
    pub fn reset_filter(&mut self, items: &[DisplayItem]) {
        self.search_query.clear();
        self.current_page = 0;
        self.hovered_index = None;
        self.hovered_center = false;
        self.hovered_main = false;
        self.hovered_back = false;
        self.filtered_indices = (0..items.len()).collect();
    }

    /// Updates the filtered index list based on the active search query.
    pub fn update_filter(&mut self, items: &[DisplayItem]) {
        let q = self.search_query.trim().to_lowercase();
        if q.is_empty() {
            self.filtered_indices = (0..items.len()).collect();
        } else {
            let mut exact_starts = Vec::new();
            let mut name_contains = Vec::new();
            let mut other_matches = Vec::new();

            for (idx, item) in items.iter().enumerate() {
                let name_lower = item.name.to_lowercase();
                let id_lower = item.id.to_lowercase();
                let target_lower = item.target.to_lowercase();

                if name_lower.starts_with(&q) || id_lower.starts_with(&q) {
                    exact_starts.push(idx);
                } else if name_lower.contains(&q) || id_lower.contains(&q) || target_lower.contains(&q) {
                    name_contains.push(idx);
                } else {
                    other_matches.push(idx);
                }
            }

            let mut results = exact_starts;
            results.extend(name_contains);
            self.filtered_indices = results;
        }

        let total_pages = self.get_total_pages();
        if self.current_page >= total_pages {
            self.current_page = total_pages.saturating_sub(1);
        }

        if !q.is_empty() && !self.filtered_indices.is_empty() {
            self.hovered_index = Some(0);
        } else {
            self.hovered_index = None;
        }
    }

    /// Computes total number of pages based on filtered items.
    pub fn get_total_pages(&self) -> usize {
        let count = self.filtered_indices.len();
        if count == 0 {
            1
        } else {
            let per_page = self.items_per_page.max(1);
            count.div_ceil(per_page)
        }
    }

    /// Advances to the next page.
    pub fn next_page(&mut self) {
        let total = self.get_total_pages();
        if total > 1 {
            self.current_page = (self.current_page + 1) % total;
            self.hovered_index = None;
        }
    }

    /// Steps back to the previous page.
    pub fn prev_page(&mut self) {
        let total = self.get_total_pages();
        if total > 1 {
            if self.current_page == 0 {
                self.current_page = total - 1;
            } else {
                self.current_page -= 1;
            }
            self.hovered_index = None;
        }
    }

    /// Retrieves the slice of DisplayItem references belonging to the current page.
    pub fn get_current_page_entries<'a>(&self, items: &'a [DisplayItem]) -> Vec<&'a DisplayItem> {
        let per_page = self.items_per_page.max(1);
        let start = self.current_page * per_page;
        let end = (start + per_page).min(self.filtered_indices.len());
        if start >= self.filtered_indices.len() {
            return Vec::new();
        }

        self.filtered_indices[start..end]
            .iter()
            .filter_map(|&idx| items.get(idx))
            .collect()
    }

    /// Retrieves the currently highlighted DisplayItem, if any.
    pub fn get_hovered_entry<'a>(&self, items: &'a [DisplayItem]) -> Option<&'a DisplayItem> {
        let idx = self.hovered_index?;
        let page_entries = self.get_current_page_entries(items);
        page_entries.get(idx).copied()
    }

    /// Loads and caches a Pixbuf for a given icon path at scale.
    pub fn get_icon_pixbuf(&self, path: &Path, size: i32) -> Option<Pixbuf> {
        let key = (path.to_path_buf(), size);
        let mut cache = self.icon_cache.borrow_mut();
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }

        let loaded = Pixbuf::from_file_at_scale(path, size, size, true).ok();
        cache.insert(key, loaded.clone());
        loaded
    }
}

/// Computes the angle in radians for button `index` of `count`.
/// In 360° circular wheel mode, button 0 starts at 12 o'clock (-π/2) and moves clockwise.
pub fn get_button_angle(state: &LauncherRenderState, index: usize, count: usize) -> f64 {
    if count == 0 {
        return 0.0;
    }

    match state.layout {
        WheelLayout::Wheel => {
            if count == 1 {
                -PI / 2.0
            } else {
                // Starts at top (-PI/2) and distributes evenly clockwise around 360°
                -PI / 2.0 + (index as f64) * (2.0 * PI / (count as f64))
            }
        }
        WheelLayout::Arc => {
            let span_rad = state.arc_span_degrees.to_radians();
            match state.orientation {
                ArcOrientation::Right => {
                    if count == 1 {
                        0.0
                    } else {
                        -span_rad / 2.0 + (index as f64) * (span_rad / ((count - 1) as f64))
                    }
                }
                ArcOrientation::Left => {
                    if count == 1 {
                        PI
                    } else {
                        PI - (span_rad / 2.0) + (index as f64) * (span_rad / ((count - 1) as f64))
                    }
                }
            }
        }
    }
}

/// Computes the exact `(x, y)` center coordinate of button `index` along the radial wheel/arc.
pub fn get_button_center(state: &LauncherRenderState, index: usize, count: usize) -> (f64, f64) {
    if count == 0 {
        return (state.center_x, state.center_y);
    }

    let angle = get_button_angle(state, index, count);
    let r = state.arc_radius;
    let bx = state.center_x + r * angle.cos();
    let by = state.center_y + r * angle.sin();
    (bx, by)
}

/// Helper function to draw a smooth rounded rectangle path onto a Cairo context.
pub fn draw_rounded_rect(cr: &Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    let r = radius.min(width / 2.0).min(height / 2.0);
    let degrees = PI / 180.0;

    cr.new_sub_path();
    cr.arc(x + width - r, y + r, r, -90.0 * degrees, 0.0 * degrees);
    cr.arc(x + width - r, y + height - r, r, 0.0 * degrees, 90.0 * degrees);
    cr.arc(x + r, y + height - r, r, 90.0 * degrees, 180.0 * degrees);
    cr.arc(x + r, y + r, r, 180.0 * degrees, 270.0 * degrees);
    cr.close_path();
}

/// Draws drop shadow around a rounded rectangle element: `0 3px 12px rgba(0, 0, 0, 0.45)`.
fn draw_drop_shadow(cr: &Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    cr.save().ok();
    for (offset_y, blur_r, alpha) in [(3.0, 10.0, 0.12), (2.0, 5.0, 0.18), (1.0, 2.5, 0.22)] {
        draw_rounded_rect(
            cr,
            x - blur_r * 0.4,
            y + offset_y - blur_r * 0.2,
            width + blur_r * 0.8,
            height + blur_r * 0.8,
            radius + blur_r * 0.3,
        );
        cr.set_source_rgba(0.0, 0.0, 0.0, alpha);
        cr.fill().ok();
    }
    cr.restore().ok();
}

/// Draws subtle drop shadow around a circle element.
fn draw_circle_drop_shadow(cr: &Context, cx: f64, cy: f64, r: f64) {
    cr.save().ok();
    for (offset_y, blur_r, alpha) in [(2.5, 8.0, 0.16), (1.2, 4.0, 0.22)] {
        cr.arc(cx, cy + offset_y, r + blur_r * 0.4, 0.0, 2.0 * PI);
        cr.set_source_rgba(0.0, 0.0, 0.0, alpha);
        cr.fill().ok();
    }
    cr.restore().ok();
}

/// Draws the Hyprvyl helm / steering wheel logo in crisp monochrome vector lines.
pub fn draw_helm_logo(cr: &Context, cx: f64, cy: f64, outer_r: f64, (r, g, b, a): (f64, f64, f64, f64)) {
    cr.save().ok();

    let inner_hub_r = outer_r * 0.22;
    let hub_ring_r = outer_r * 0.38;
    let mid_ring_r = outer_r * 0.72;
    let rim_outer_r = outer_r * 0.95;
    let pin_reach_r = outer_r * 1.30;
    let pin_dot_r = outer_r * 0.11;

    cr.set_line_width(1.35);
    cr.set_source_rgba(r, g, b, a);

    // 1. Center hub circle (filled)
    cr.arc(cx, cy, inner_hub_r, 0.0, 2.0 * PI);
    cr.fill().ok();

    // 2. Hub concentric ring
    cr.arc(cx, cy, hub_ring_r, 0.0, 2.0 * PI);
    cr.stroke().ok();

    // 3. Middle ring & Outer rim ring
    cr.arc(cx, cy, mid_ring_r, 0.0, 2.0 * PI);
    cr.stroke().ok();

    cr.arc(cx, cy, rim_outer_r, 0.0, 2.0 * PI);
    cr.stroke().ok();

    // 4. Eight radial spokes
    for i in 0..8 {
        let theta = (i as f64) * (PI / 4.0); // 0, 45, 90, 135, 180, 225, 270, 315 deg
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        let start_x = cx + cos_t * hub_ring_r;
        let start_y = cy + sin_t * hub_ring_r;

        if i % 2 == 0 {
            // Cardinal directions (Top, Right, Bottom, Left) with extended pin and terminal ball
            let end_x = cx + cos_t * pin_reach_r;
            let end_y = cy + sin_t * pin_reach_r;

            cr.move_to(start_x, start_y);
            cr.line_to(end_x, end_y);
            cr.stroke().ok();

            // Terminal ball dot
            cr.arc(end_x, end_y, pin_dot_r, 0.0, 2.0 * PI);
            cr.fill().ok();
        } else {
            // Diagonal spokes (extend to outer rim)
            let end_x = cx + cos_t * (rim_outer_r + 1.2);
            let end_y = cy + sin_t * (rim_outer_r + 1.2);

            cr.move_to(start_x, start_y);
            cr.line_to(end_x, end_y);
            cr.stroke().ok();
        }
    }

    cr.restore().ok();
}

/// Calculates the layout coordinates for the top workspace badge pill (e.g. `Main [ 1 ]`).
pub fn get_top_workspace_pill_layout(state: &LauncherRenderState) -> (f64, f64, f64, f64) {
    let pill_h = 30.0;
    let pill_w = 84.0;
    let pill_x = state.center_x - (pill_w / 2.0);
    let pill_y = state.center_y - state.arc_radius - (state.button_size / 2.0) - 40.0;
    (pill_x, pill_y, pill_w, pill_h)
}

/// Calculates the layout coordinates for the bottom breadcrumb / switcher pill.
pub fn get_bottom_switcher_layout(state: &LauncherRenderState, _text: &str) -> (f64, f64, f64, f64) {
    let pill_h = BREADCRUMB_HEIGHT;
    let pill_w = 140.0;
    let pill_x = state.center_x - (pill_w / 2.0);
    let pill_y = state.center_y + state.arc_radius + (state.button_size / 2.0) + 18.0;
    (pill_x, pill_y, pill_w, pill_h)
}

/// Draws the complete radial wheel launcher overlay onto the Cairo context.
pub fn draw_radial_overlay(
    cr: &Context,
    state: &LauncherRenderState,
    items: &[DisplayItem],
    _width: i32,
    _height: i32,
) {
    let page_entries = state.get_current_page_entries(items);
    let count = page_entries.len();

    let base_btn_size = state.button_size;
    let base_corner_r = if state.corner_radius > 0.0 {
        state.corner_radius
    } else {
        DEFAULT_CORNER_RADIUS
    };

    // -------------------------------------------------------------
    // 1. Draw Top Workspace Badge Pill (Image 2 style: `Main [ 1 ]`)
    // -------------------------------------------------------------
    let (top_px, top_py, top_pw, top_ph) = get_top_workspace_pill_layout(state);
    let top_pr = top_ph / 2.0;

    draw_drop_shadow(cr, top_px, top_py, top_pw, top_ph, top_pr);

    // Body: White background (#ffffff)
    draw_rounded_rect(cr, top_px, top_py, top_pw, top_ph, top_pr);
    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    cr.fill().ok();

    // Text: Workspace Name (e.g. "Main")
    cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
    cr.set_font_size(12.5);
    cr.set_source_rgba(0.12, 0.12, 0.14, 1.0);

    let ws_title = &state.breadcrumb_title;
    let title_x = top_px + 14.0;
    let title_y = top_py + (top_ph / 2.0) + 4.0;
    cr.move_to(title_x, title_y);
    cr.show_text(ws_title).ok();

    // Badge: Rounded Tag Pill with number (e.g. "1")
    let badge_w = 18.0;
    let badge_h = 18.0;
    let badge_x = top_px + top_pw - badge_w - 8.0;
    let badge_y = top_py + (top_ph - badge_h) / 2.0;
    let badge_r = 5.0;

    draw_rounded_rect(cr, badge_x, badge_y, badge_w, badge_h, badge_r);
    cr.set_source_rgba(0.91, 0.91, 0.93, 1.0);
    cr.fill().ok();

    let num_str = format!("{}", state.workspace_index);
    cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
    cr.set_font_size(11.0);
    cr.set_source_rgba(0.40, 0.40, 0.45, 1.0);
    if let Ok(ext) = cr.text_extents(&num_str) {
        let num_x = badge_x + (badge_w - ext.width()) / 2.0;
        let num_y = badge_y + (badge_h + ext.height()) / 2.0 - 1.0;
        cr.move_to(num_x, num_y);
        cr.show_text(&num_str).ok();
    }

    // -------------------------------------------------------------
    // 2. Draw Center Hub Button (Image 1 & 2 style)
    // -------------------------------------------------------------
    let hub_r = CENTER_HUB_RADIUS;
    let cx = state.center_x;
    let cy = state.center_y;

    draw_circle_drop_shadow(cr, cx, cy, hub_r);

    cr.arc(cx, cy, hub_r, 0.0, 2.0 * PI);
    if state.hovered_center {
        // Highlighted on hover
        cr.set_source_rgba(0.24, 0.24, 0.28, 0.98);
    } else {
        // Normal dark translucent
        cr.set_source_rgba(0.11, 0.11, 0.13, 0.92);
    }
    cr.fill_preserve().ok();

    // Subtle hub border
    cr.set_line_width(1.0);
    cr.set_source_rgba(1.0, 1.0, 1.0, if state.hovered_center { 0.30 } else { 0.12 });
    cr.stroke().ok();

    // Center Glyph / Icon:
    // Always render the Official Hyprvyl Helm Wheel Logo (matches assets/logo.png)
    let alpha = if state.hovered_center { 1.0 } else { 0.88 };
    draw_helm_logo(cr, cx, cy, 18.0, (0.95, 0.95, 0.98, alpha));

    // -------------------------------------------------------------
    // 3. Draw Radial Buttons & Adjacent Tooltip Pills
    // -------------------------------------------------------------
    for (i, item) in page_entries.iter().enumerate() {
        let (bx_center, by_center) = get_button_center(state, i, count);
        let is_hovered = state.hovered_index == Some(i);

        // Hover scale: ~8% increase in dimensions
        let btn_size = if is_hovered {
            base_btn_size * 1.08
        } else {
            base_btn_size
        };
        let corner_r = base_corner_r * (btn_size / base_btn_size);

        let bx = bx_center - btn_size / 2.0;
        let by = by_center - btn_size / 2.0;

        // Button drop shadow
        draw_drop_shadow(cr, bx, by, btn_size, btn_size, corner_r);

        // Button body
        draw_rounded_rect(cr, bx, by, btn_size, btn_size, corner_r);

        if is_hovered {
            // Hovered state: pure bright white #ffffff
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.fill_preserve().ok();
            cr.set_line_width(1.5);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
            cr.stroke().ok();
        } else {
            // Normal unhovered state: rgba(28, 28, 30, 0.90)
            cr.set_source_rgba(0.11, 0.11, 0.12, 0.90);
            cr.fill_preserve().ok();
            cr.set_line_width(1.0);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.08);
            cr.stroke().ok();
        }

        // Draw centered icon
        let icon_target_size = (btn_size - 20.0).max(24.0) as i32;

        let icon_drawn = if let Some(ref icon_path) = item.icon_path {
            if let Some(pixbuf) = state.get_icon_pixbuf(icon_path, icon_target_size) {
                let px = bx_center - (pixbuf.width() as f64) / 2.0;
                let py = by_center - (pixbuf.height() as f64) / 2.0;
                cr.set_source_pixbuf(&pixbuf, px, py);
                cr.paint().ok();
                true
            } else {
                false
            }
        } else if let Some(pixbuf) = crate::icons::get_bundled_icon_pixbuf(&item.icon_name, icon_target_size) {
            let px = bx_center - (pixbuf.width() as f64) / 2.0;
            let py = by_center - (pixbuf.height() as f64) / 2.0;
            cr.set_source_pixbuf(&pixbuf, px, py);
            cr.paint().ok();
            true
        } else if item.kind == "workspace"
            && let Some(pixbuf) = crate::icons::get_bundled_icon_pixbuf(&item.target, icon_target_size)
        {
            let px = bx_center - (pixbuf.width() as f64) / 2.0;
            let py = by_center - (pixbuf.height() as f64) / 2.0;
            cr.set_source_pixbuf(&pixbuf, px, py);
            cr.paint().ok();
            true
        } else {
            false
        };

        if !icon_drawn {
            // High-quality fallback icon in clean typography
            cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
            cr.set_font_size(15.0);
            if is_hovered {
                cr.set_source_rgba(0.10, 0.10, 0.12, 1.0);
            } else {
                cr.set_source_rgba(0.92, 0.92, 0.95, 0.95);
            }

            let initial_str = item.name.chars().next().unwrap_or('?').to_uppercase().to_string();
            let fallback_glyph = match item.kind.as_str() {
                "workspace" => "W",
                "folder" => "DIR",
                "url" => "URL",
                _ => initial_str.as_str(),
            };

            if let Ok(ext) = cr.text_extents(fallback_glyph) {
                cr.move_to(bx_center - (ext.width() / 2.0), by_center + (ext.height() / 2.0) - 1.0);
                cr.show_text(fallback_glyph).ok();
            }
        }

        // ---------------------------------------------------------
        // 4. Hovered State: Adjacent Tooltip Pill Label (e.g. "WhatsApp")
        // ---------------------------------------------------------
        if is_hovered {
            let label_text = &item.name;
            cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
            cr.set_font_size(13.5);

            let (tw, th) = if let Ok(ext) = cr.text_extents(label_text) {
                (ext.width(), ext.height())
            } else {
                (60.0, 14.0)
            };

            let pill_h = PILL_HEIGHT;
            let pill_padding_h = 16.0;
            let pill_w = (tw + pill_padding_h * 2.0).max(74.0);
            let pill_r = pill_h / 2.0;

            // Position pill outward from center
            let angle = get_button_angle(state, i, count);
            let (lx, ly) = if angle.cos() >= -0.1 {
                // Right side
                (bx_center + (btn_size / 2.0) + 12.0, by_center - (pill_h / 2.0))
            } else {
                // Left side
                (bx_center - (btn_size / 2.0) - pill_w - 12.0, by_center - (pill_h / 2.0))
            };

            // Pill drop shadow
            draw_drop_shadow(cr, lx, ly, pill_w, pill_h, pill_r);

            // Pill body (#ffffff)
            draw_rounded_rect(cr, lx, ly, pill_w, pill_h, pill_r);
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.fill().ok();

            // Label text (crisp dark #111111)
            cr.set_source_rgba(0.08, 0.08, 0.10, 1.0);
            let tx = lx + pill_padding_h;
            let ty = ly + (pill_h / 2.0) + (th / 2.0) - 1.0;
            cr.move_to(tx, ty);
            cr.show_text(label_text).ok();
        }
    }

    // -------------------------------------------------------------
    // 5. Draw Bottom Segmented Switcher / Breadcrumb Pill
    // (Image 1 & 2 style: `[ Main | Back ]` or `[ Rovyl | Center ]`)
    // -------------------------------------------------------------
    let breadcrumb_text = if !state.search_query.is_empty() {
        format!("Search: \"{}\" ({})", truncate_text(&state.search_query, 12), state.filtered_indices.len())
    } else {
        format!("{} · {}", state.breadcrumb_title, state.workspace_index)
    };

    let (bot_x, bot_y, bot_w, bot_h) = get_bottom_switcher_layout(state, &breadcrumb_text);
    let bot_r = bot_h / 2.0;

    draw_drop_shadow(cr, bot_x, bot_y, bot_w, bot_h, bot_r);

    // Outer container background
    draw_rounded_rect(cr, bot_x, bot_y, bot_w, bot_h, bot_r);
    cr.set_source_rgba(0.09, 0.09, 0.11, 0.90);
    cr.fill_preserve().ok();
    cr.set_line_width(1.0);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.10);
    cr.stroke().ok();

    if !state.search_query.is_empty() {
        // Render search status
        cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
        cr.set_font_size(11.5);
        cr.set_source_rgba(0.92, 0.92, 0.95, 0.95);
        if let Ok(ext) = cr.text_extents(&breadcrumb_text) {
            let tx = bot_x + (bot_w - ext.width()) / 2.0;
            let ty = bot_y + (bot_h + ext.height()) / 2.0 - 1.0;
            cr.move_to(tx, ty);
            cr.show_text(&breadcrumb_text).ok();
        }
    } else {
        // Two Segment Button: [ Main | Back ]
        let seg_w = (bot_w - 6.0) / 2.0;
        let seg_h = bot_h - 6.0;
        let seg_r = seg_h / 2.0;

        // Left Segment (e.g. "Main" or "Hyprvyl")
        let seg1_x = bot_x + 3.0;
        let seg1_y = bot_y + 3.0;

        if state.hovered_main {
            draw_rounded_rect(cr, seg1_x, seg1_y, seg_w, seg_h, seg_r);
            cr.set_source_rgba(0.24, 0.24, 0.28, 0.95);
            cr.fill().ok();
        } else {
            draw_rounded_rect(cr, seg1_x, seg1_y, seg_w, seg_h, seg_r);
            cr.set_source_rgba(0.18, 0.18, 0.22, 0.95);
            cr.fill().ok();
        }

        cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
        cr.set_font_size(11.5);
        cr.set_source_rgba(0.95, 0.95, 0.98, 0.95);
        let left_text = if state.is_sublevel { &state.breadcrumb_title } else { "Hyprvyl" };
        if let Ok(ext) = cr.text_extents(left_text) {
            let tx = seg1_x + (seg_w - ext.width()) / 2.0;
            let ty = seg1_y + (seg_h + ext.height()) / 2.0 - 1.0;
            cr.move_to(tx, ty);
            cr.show_text(left_text).ok();
        }

        // Right Segment (e.g. "Back" or "Center")
        let seg2_x = bot_x + 3.0 + seg_w;
        let seg2_y = bot_y + 3.0;

        if state.hovered_back {
            draw_rounded_rect(cr, seg2_x, seg2_y, seg_w, seg_h, seg_r);
            cr.set_source_rgba(0.24, 0.24, 0.28, 0.95);
            cr.fill().ok();
        }

        let right_text = if state.is_sublevel { "Back" } else { "Center" };
        cr.select_font_face("Sans", FontSlant::Normal, if state.hovered_back { FontWeight::Bold } else { FontWeight::Normal });
        cr.set_font_size(11.5);
        if state.hovered_back {
            cr.set_source_rgba(0.95, 0.95, 0.98, 0.95);
        } else {
            cr.set_source_rgba(0.60, 0.60, 0.65, 0.90);
        }
        if let Ok(ext) = cr.text_extents(right_text) {
            let tx = seg2_x + (seg_w - ext.width()) / 2.0;
            let ty = seg2_y + (seg_h + ext.height()) / 2.0 - 1.0;
            cr.move_to(tx, ty);
            cr.show_text(right_text).ok();
        }
    }
}

/// Evaluates mouse coordinates `(x, y)` and returns the matching button index `0..count`, or `None`.
/// Supports direct bounding box hit-testing and angle-based gesture flick detection.
pub fn get_hovered_button_for_point(
    x: f64,
    y: f64,
    state: &LauncherRenderState,
    num_items: usize,
) -> Option<usize> {
    if num_items == 0 {
        return None;
    }

    let dx = x - state.center_x;
    let dy = y - state.center_y;
    let dist = (dx * dx + dy * dy).sqrt();

    // If mouse is inside center hub deadzone, no outer button is hovered
    if dist <= CENTER_HUB_RADIUS + 4.0 {
        return None;
    }

    let btn_size = state.button_size;
    let half_size = btn_size / 2.0;

    // 1. Check direct bounding boxes (including extended pill area when hovered)
    for i in 0..num_items {
        let (cx, cy) = get_button_center(state, i, num_items);
        let is_currently_hovered = state.hovered_index == Some(i);

        let min_y = cy - half_size - 6.0;
        let max_y = cy + half_size + 6.0;

        let angle = get_button_angle(state, i, num_items);
        let (min_x, max_x) = if angle.cos() >= -0.1 {
            let min = cx - half_size - 6.0;
            let max = if is_currently_hovered {
                cx + half_size + 240.0 // Allow hovering over adjacent pill label
            } else {
                cx + half_size + 6.0
            };
            (min, max)
        } else {
            let max = cx + half_size + 6.0;
            let min = if is_currently_hovered {
                cx - half_size - 240.0 // Allow hovering over adjacent pill label
            } else {
                cx - half_size - 6.0
            };
            (min, max)
        };

        if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
            return Some(i);
        }
    }

    // 2. Gesture / Directional Flick Hit-Testing (for 360° circular wheel mode)
    if state.layout == WheelLayout::Wheel && dist > CENTER_HUB_RADIUS + 6.0 && dist <= state.arc_radius + btn_size + 50.0 {
        let mouse_angle = dy.atan2(dx); // -PI to +PI
        let mut best_idx = None;
        let mut min_diff = f64::MAX;

        for i in 0..num_items {
            let btn_angle = get_button_angle(state, i, num_items);
            let mut diff = (mouse_angle - btn_angle).abs();
            if diff > PI {
                diff = 2.0 * PI - diff;
            }

            if diff < min_diff {
                min_diff = diff;
                best_idx = Some(i);
            }
        }

        let max_allowed_diff = (PI / (num_items as f64)).max(0.35);
        if min_diff <= max_allowed_diff {
            return best_idx;
        }
    }

    None
}

/// Checks if mouse coordinates `(x, y)` hover over the center hub.
pub fn is_center_hub_hovered(x: f64, y: f64, state: &LauncherRenderState) -> bool {
    let dx = x - state.center_x;
    let dy = y - state.center_y;
    (dx * dx + dy * dy) <= (CENTER_HUB_RADIUS + 4.0) * (CENTER_HUB_RADIUS + 4.0)
}

/// Checks if mouse coordinates `(x, y)` hover over the bottom left button segment ("Main" / "Hyprvyl").
pub fn is_main_button_hovered(x: f64, y: f64, state: &LauncherRenderState) -> bool {
    let dummy_text = "Main · 1";
    let (bot_x, bot_y, bot_w, bot_h) = get_bottom_switcher_layout(state, dummy_text);
    let seg_w = (bot_w - 6.0) / 2.0;
    let seg_h = bot_h - 6.0;
    let seg1_x = bot_x + 3.0;
    let seg1_y = bot_y + 3.0;

    x >= seg1_x && x <= seg1_x + seg_w && y >= seg1_y && y <= seg1_y + seg_h
}

/// Checks if mouse coordinates `(x, y)` hover over the bottom right button segment ("Back" / "Center").
pub fn is_back_button_hovered(x: f64, y: f64, state: &LauncherRenderState) -> bool {
    let dummy_text = "Main · 1";
    let (bot_x, bot_y, bot_w, bot_h) = get_bottom_switcher_layout(state, dummy_text);
    let seg_w = (bot_w - 6.0) / 2.0;
    let seg_h = bot_h - 6.0;
    let seg2_x = bot_x + 3.0 + seg_w;
    let seg2_y = bot_y + 3.0;

    x >= seg2_x && x <= seg2_x + seg_w && y >= seg2_y && y <= seg2_y + seg_h
}

/// Truncates text with an ellipsis if it exceeds `max_chars`.
pub fn truncate_text(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        let mut truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        truncated.push('…');
        truncated
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_item(name: &str, kind: &str, target: &str) -> DisplayItem {
        DisplayItem {
            id: name.to_lowercase(),
            name: name.to_string(),
            kind: kind.to_string(),
            target: target.to_string(),
            icon_path: None,
            icon_name: String::new(),
        }
    }

    #[test]
    fn test_radial_wheel_geometry_360() {
        let mut state = LauncherRenderState::default();
        state.center_x = 500.0;
        state.center_y = 400.0;
        state.arc_radius = 140.0;
        state.layout = WheelLayout::Wheel;

        // 4 items: Top, Right, Bottom, Left
        let (x0, y0) = get_button_center(&state, 0, 4);
        assert!((x0 - 500.0).abs() < 1e-4, "Top button x should be center_x");
        assert!((y0 - (400.0 - 140.0)).abs() < 1e-4, "Top button y should be center_y - radius");

        let (x1, y1) = get_button_center(&state, 1, 4);
        assert!((x1 - (500.0 + 140.0)).abs() < 1e-4, "Right button x should be center_x + radius");
        assert!((y1 - 400.0).abs() < 1e-4, "Right button y should be center_y");

        let (x2, y2) = get_button_center(&state, 2, 4);
        assert!((x2 - 500.0).abs() < 1e-4, "Bottom button x should be center_x");
        assert!((y2 - (400.0 + 140.0)).abs() < 1e-4, "Bottom button y should be center_y + radius");

        let (x3, y3) = get_button_center(&state, 3, 4);
        assert!((x3 - (500.0 - 140.0)).abs() < 1e-4, "Left button x should be center_x - radius");
        assert!((y3 - 400.0).abs() < 1e-4, "Left button y should be center_y");
    }

    #[test]
    fn test_center_hub_hit_testing() {
        let mut state = LauncherRenderState::default();
        state.center_x = 500.0;
        state.center_y = 400.0;

        assert!(is_center_hub_hovered(500.0, 400.0, &state));
        assert!(is_center_hub_hovered(510.0, 410.0, &state));
        assert!(!is_center_hub_hovered(500.0, 460.0, &state));
    }

    #[test]
    fn test_directional_flick_hit_testing() {
        let mut state = LauncherRenderState::default();
        state.center_x = 500.0;
        state.center_y = 400.0;
        state.arc_radius = 140.0;
        state.layout = WheelLayout::Wheel;

        // Mouse moved rightward from center (x=600, y=400)
        let hovered = get_hovered_button_for_point(600.0, 400.0, &state, 4);
        assert_eq!(hovered, Some(1), "Rightward flick should select right button (index 1)");

        // Mouse moved upward from center (x=500, y=300)
        let hovered_top = get_hovered_button_for_point(500.0, 300.0, &state, 4);
        assert_eq!(hovered_top, Some(0), "Upward flick should select top button (index 0)");
    }

    #[test]
    fn test_pagination_and_filter() {
        let items = vec![
            dummy_item("Chrome", "app", "google-chrome.desktop"),
            dummy_item("WhatsApp", "url", "https://web.whatsapp.com"),
            dummy_item("Notepad", "app", "gedit.desktop"),
            dummy_item("Calculator", "app", "gnome-calculator.desktop"),
            dummy_item("Files", "app", "nemo.desktop"),
            dummy_item("Music", "folder", "/home/dev/Music"),
            dummy_item("Editor", "app", "code.desktop"),
            dummy_item("Settings", "app", "hyprvyl --settings"),
            dummy_item("Github", "url", "https://github.com"),
        ];

        let mut state = LauncherRenderState::default();
        state.items_per_page = 8;
        state.reset_filter(&items);

        assert_eq!(state.get_total_pages(), 2);
        assert_eq!(state.get_current_page_entries(&items).len(), 8);

        state.next_page();
        assert_eq!(state.current_page, 1);
        assert_eq!(state.get_current_page_entries(&items).len(), 1);

        state.search_query = "whats".to_string();
        state.update_filter(&items);
        assert_eq!(state.filtered_indices.len(), 1);
        assert_eq!(state.hovered_index, Some(0));
    }
}
