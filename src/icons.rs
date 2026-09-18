//! Curated Bundled SVG Icon Library for Hyprvyl.
//!
//! Provides 180+ clean, crisp, modern SVG icons embedded at compile-time.
//! Prioritized during icon resolution and displayed in the Settings Icon Picker.

use gdk_pixbuf::{Pixbuf, PixbufLoader};
use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// Metadata for a bundled SVG icon entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundledIcon {
    /// Unique string identifier (e.g. "folder-code", "globe", "github")
    pub id: &'static str,
    /// Human-friendly display name (e.g. "Code Folder", "Globe / Internet")
    pub name: &'static str,
    /// Category group (e.g. "Folders", "Web", "Development", "Media", "Communication", "Productivity", "System", "Brands")
    pub category: &'static str,
    /// Raw SVG vector markup embedded at compile-time
    pub svg_data: &'static str,
}

/// Full compile-time registry of all bundled icons.
pub static BUNDLED_ICONS: &[BundledIcon] = &[
    BundledIcon {
        id: "folder",
        name: "Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder.svg"),
    },
    BundledIcon {
        id: "folder-code",
        name: "Code Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-code.svg"),
    },
    BundledIcon {
        id: "folder-git",
        name: "Git Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-git.svg"),
    },
    BundledIcon {
        id: "folder-music",
        name: "Music Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-music.svg"),
    },
    BundledIcon {
        id: "folder-pictures",
        name: "Pictures Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-pictures.svg"),
    },
    BundledIcon {
        id: "folder-videos",
        name: "Videos Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-videos.svg"),
    },
    BundledIcon {
        id: "folder-download",
        name: "Downloads Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-download.svg"),
    },
    BundledIcon {
        id: "folder-documents",
        name: "Documents Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-documents.svg"),
    },
    BundledIcon {
        id: "folder-archive",
        name: "Archive Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-archive.svg"),
    },
    BundledIcon {
        id: "folder-lock",
        name: "Locked Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-lock.svg"),
    },
    BundledIcon {
        id: "folder-heart",
        name: "Favorites Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-heart.svg"),
    },
    BundledIcon {
        id: "folder-star",
        name: "Starred Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-star.svg"),
    },
    BundledIcon {
        id: "folder-cloud",
        name: "Cloud Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-cloud.svg"),
    },
    BundledIcon {
        id: "folder-home",
        name: "Home Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-home.svg"),
    },
    BundledIcon {
        id: "folder-open",
        name: "Open Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-open.svg"),
    },
    BundledIcon {
        id: "folder-shared",
        name: "Shared Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-shared.svg"),
    },
    BundledIcon {
        id: "folder-work",
        name: "Work Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-work.svg"),
    },
    BundledIcon {
        id: "folder-trash",
        name: "Trash Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-trash.svg"),
    },
    BundledIcon {
        id: "folder-settings",
        name: "Settings Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-settings.svg"),
    },
    BundledIcon {
        id: "folder-terminal",
        name: "Terminal Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-terminal.svg"),
    },
    BundledIcon {
        id: "folder-plus",
        name: "New Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-plus.svg"),
    },
    BundledIcon {
        id: "folder-minus",
        name: "Remove Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-minus.svg"),
    },
    BundledIcon {
        id: "folder-check",
        name: "Checked Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-check.svg"),
    },
    BundledIcon {
        id: "folder-alert",
        name: "Alert Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-alert.svg"),
    },
    BundledIcon {
        id: "folder-search",
        name: "Search Folder",
        category: "Folders",
        svg_data: include_str!("../assets/icons/folder-search.svg"),
    },
    BundledIcon {
        id: "file",
        name: "File",
        category: "Files",
        svg_data: include_str!("../assets/icons/file.svg"),
    },
    BundledIcon {
        id: "file-text",
        name: "Text Document",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-text.svg"),
    },
    BundledIcon {
        id: "file-code",
        name: "Code File",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-code.svg"),
    },
    BundledIcon {
        id: "file-archive",
        name: "Zip File",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-archive.svg"),
    },
    BundledIcon {
        id: "file-image",
        name: "Image File",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-image.svg"),
    },
    BundledIcon {
        id: "file-audio",
        name: "Audio File",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-audio.svg"),
    },
    BundledIcon {
        id: "file-video",
        name: "Video File",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-video.svg"),
    },
    BundledIcon {
        id: "file-pdf",
        name: "PDF Document",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-pdf.svg"),
    },
    BundledIcon {
        id: "file-json",
        name: "JSON File",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-json.svg"),
    },
    BundledIcon {
        id: "file-lock",
        name: "Locked File",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-lock.svg"),
    },
    BundledIcon {
        id: "file-check",
        name: "Verified File",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-check.svg"),
    },
    BundledIcon {
        id: "file-plus",
        name: "Add File",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-plus.svg"),
    },
    BundledIcon {
        id: "file-diff",
        name: "Diff File",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-diff.svg"),
    },
    BundledIcon {
        id: "file-spreadsheet",
        name: "Spreadsheet",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-spreadsheet.svg"),
    },
    BundledIcon {
        id: "file-presentation",
        name: "Presentation",
        category: "Files",
        svg_data: include_str!("../assets/icons/file-presentation.svg"),
    },
    BundledIcon {
        id: "globe",
        name: "Globe / Internet",
        category: "Web",
        svg_data: include_str!("../assets/icons/globe.svg"),
    },
    BundledIcon {
        id: "browser",
        name: "Browser",
        category: "Web",
        svg_data: include_str!("../assets/icons/browser.svg"),
    },
    BundledIcon {
        id: "compass",
        name: "Compass",
        category: "Web",
        svg_data: include_str!("../assets/icons/compass.svg"),
    },
    BundledIcon {
        id: "link",
        name: "Hyperlink",
        category: "Web",
        svg_data: include_str!("../assets/icons/link.svg"),
    },
    BundledIcon {
        id: "link-external",
        name: "External Link",
        category: "Web",
        svg_data: include_str!("../assets/icons/link-external.svg"),
    },
    BundledIcon {
        id: "bookmark",
        name: "Bookmark",
        category: "Web",
        svg_data: include_str!("../assets/icons/bookmark.svg"),
    },
    BundledIcon {
        id: "cloud",
        name: "Cloud",
        category: "Web",
        svg_data: include_str!("../assets/icons/cloud.svg"),
    },
    BundledIcon {
        id: "cloud-download",
        name: "Cloud Download",
        category: "Web",
        svg_data: include_str!("../assets/icons/cloud-download.svg"),
    },
    BundledIcon {
        id: "cloud-upload",
        name: "Cloud Upload",
        category: "Web",
        svg_data: include_str!("../assets/icons/cloud-upload.svg"),
    },
    BundledIcon {
        id: "wifi",
        name: "Wi-Fi",
        category: "Web",
        svg_data: include_str!("../assets/icons/wifi.svg"),
    },
    BundledIcon {
        id: "network",
        name: "Network",
        category: "Web",
        svg_data: include_str!("../assets/icons/network.svg"),
    },
    BundledIcon {
        id: "rss",
        name: "RSS",
        category: "Web",
        svg_data: include_str!("../assets/icons/rss.svg"),
    },
    BundledIcon {
        id: "search",
        name: "Search",
        category: "Web",
        svg_data: include_str!("../assets/icons/search.svg"),
    },
    BundledIcon {
        id: "server",
        name: "Server",
        category: "Web",
        svg_data: include_str!("../assets/icons/server.svg"),
    },
    BundledIcon {
        id: "database",
        name: "Database",
        category: "Web",
        svg_data: include_str!("../assets/icons/database.svg"),
    },
    BundledIcon {
        id: "shield",
        name: "Shield",
        category: "Web",
        svg_data: include_str!("../assets/icons/shield.svg"),
    },
    BundledIcon {
        id: "shield-check",
        name: "Verified Shield",
        category: "Web",
        svg_data: include_str!("../assets/icons/shield-check.svg"),
    },
    BundledIcon {
        id: "lock",
        name: "Lock",
        category: "Web",
        svg_data: include_str!("../assets/icons/lock.svg"),
    },
    BundledIcon {
        id: "unlock",
        name: "Unlock",
        category: "Web",
        svg_data: include_str!("../assets/icons/unlock.svg"),
    },
    BundledIcon {
        id: "key",
        name: "Key",
        category: "Web",
        svg_data: include_str!("../assets/icons/key.svg"),
    },
    BundledIcon {
        id: "code",
        name: "Code Brackets",
        category: "Development",
        svg_data: include_str!("../assets/icons/code.svg"),
    },
    BundledIcon {
        id: "terminal",
        name: "Terminal",
        category: "Development",
        svg_data: include_str!("../assets/icons/terminal.svg"),
    },
    BundledIcon {
        id: "cpu",
        name: "CPU Chip",
        category: "Development",
        svg_data: include_str!("../assets/icons/cpu.svg"),
    },
    BundledIcon {
        id: "hard-drive",
        name: "Hard Drive",
        category: "Development",
        svg_data: include_str!("../assets/icons/hard-drive.svg"),
    },
    BundledIcon {
        id: "git-branch",
        name: "Git Branch",
        category: "Development",
        svg_data: include_str!("../assets/icons/git-branch.svg"),
    },
    BundledIcon {
        id: "git-commit",
        name: "Git Commit",
        category: "Development",
        svg_data: include_str!("../assets/icons/git-commit.svg"),
    },
    BundledIcon {
        id: "git-pull-request",
        name: "Pull Request",
        category: "Development",
        svg_data: include_str!("../assets/icons/git-pull-request.svg"),
    },
    BundledIcon {
        id: "git-merge",
        name: "Git Merge",
        category: "Development",
        svg_data: include_str!("../assets/icons/git-merge.svg"),
    },
    BundledIcon {
        id: "bug",
        name: "Bug",
        category: "Development",
        svg_data: include_str!("../assets/icons/bug.svg"),
    },
    BundledIcon {
        id: "box",
        name: "Box",
        category: "Development",
        svg_data: include_str!("../assets/icons/box.svg"),
    },
    BundledIcon {
        id: "package",
        name: "Package",
        category: "Development",
        svg_data: include_str!("../assets/icons/package.svg"),
    },
    BundledIcon {
        id: "layers",
        name: "Layers",
        category: "Development",
        svg_data: include_str!("../assets/icons/layers.svg"),
    },
    BundledIcon {
        id: "tool",
        name: "Tool",
        category: "Development",
        svg_data: include_str!("../assets/icons/tool.svg"),
    },
    BundledIcon {
        id: "wrench",
        name: "Wrench",
        category: "Development",
        svg_data: include_str!("../assets/icons/wrench.svg"),
    },
    BundledIcon {
        id: "hammer",
        name: "Hammer",
        category: "Development",
        svg_data: include_str!("../assets/icons/hammer.svg"),
    },
    BundledIcon {
        id: "settings",
        name: "Settings Gear",
        category: "Development",
        svg_data: include_str!("../assets/icons/settings.svg"),
    },
    BundledIcon {
        id: "sliders",
        name: "Sliders",
        category: "Development",
        svg_data: include_str!("../assets/icons/sliders.svg"),
    },
    BundledIcon {
        id: "command",
        name: "Command",
        category: "Development",
        svg_data: include_str!("../assets/icons/command.svg"),
    },
    BundledIcon {
        id: "activity",
        name: "Activity Monitor",
        category: "Development",
        svg_data: include_str!("../assets/icons/activity.svg"),
    },
    BundledIcon {
        id: "flask",
        name: "Flask / Lab",
        category: "Development",
        svg_data: include_str!("../assets/icons/flask.svg"),
    },
    BundledIcon {
        id: "binary",
        name: "Binary Code",
        category: "Development",
        svg_data: include_str!("../assets/icons/binary.svg"),
    },
    BundledIcon {
        id: "code-square",
        name: "Code Box",
        category: "Development",
        svg_data: include_str!("../assets/icons/code-square.svg"),
    },
    BundledIcon {
        id: "terminal-box",
        name: "Terminal Box",
        category: "Development",
        svg_data: include_str!("../assets/icons/terminal-box.svg"),
    },
    BundledIcon {
        id: "webhook",
        name: "Webhook",
        category: "Development",
        svg_data: include_str!("../assets/icons/webhook.svg"),
    },
    BundledIcon {
        id: "api",
        name: "API Connect",
        category: "Development",
        svg_data: include_str!("../assets/icons/api.svg"),
    },
    BundledIcon {
        id: "music",
        name: "Music Note",
        category: "Media",
        svg_data: include_str!("../assets/icons/music.svg"),
    },
    BundledIcon {
        id: "headphones",
        name: "Headphones",
        category: "Media",
        svg_data: include_str!("../assets/icons/headphones.svg"),
    },
    BundledIcon {
        id: "play",
        name: "Play",
        category: "Media",
        svg_data: include_str!("../assets/icons/play.svg"),
    },
    BundledIcon {
        id: "pause",
        name: "Pause",
        category: "Media",
        svg_data: include_str!("../assets/icons/pause.svg"),
    },
    BundledIcon {
        id: "film",
        name: "Film Movie",
        category: "Media",
        svg_data: include_str!("../assets/icons/film.svg"),
    },
    BundledIcon {
        id: "video",
        name: "Video Camera",
        category: "Media",
        svg_data: include_str!("../assets/icons/video.svg"),
    },
    BundledIcon {
        id: "camera",
        name: "Camera",
        category: "Media",
        svg_data: include_str!("../assets/icons/camera.svg"),
    },
    BundledIcon {
        id: "image",
        name: "Picture",
        category: "Media",
        svg_data: include_str!("../assets/icons/image.svg"),
    },
    BundledIcon {
        id: "speaker",
        name: "Speaker",
        category: "Media",
        svg_data: include_str!("../assets/icons/speaker.svg"),
    },
    BundledIcon {
        id: "volume-2",
        name: "High Volume",
        category: "Media",
        svg_data: include_str!("../assets/icons/volume-2.svg"),
    },
    BundledIcon {
        id: "mic",
        name: "Microphone",
        category: "Media",
        svg_data: include_str!("../assets/icons/mic.svg"),
    },
    BundledIcon {
        id: "radio",
        name: "Radio",
        category: "Media",
        svg_data: include_str!("../assets/icons/radio.svg"),
    },
    BundledIcon {
        id: "tv",
        name: "Television",
        category: "Media",
        svg_data: include_str!("../assets/icons/tv.svg"),
    },
    BundledIcon {
        id: "disc",
        name: "Vinyl Disc",
        category: "Media",
        svg_data: include_str!("../assets/icons/disc.svg"),
    },
    BundledIcon {
        id: "cast",
        name: "Screen Cast",
        category: "Media",
        svg_data: include_str!("../assets/icons/cast.svg"),
    },
    BundledIcon {
        id: "gamepad",
        name: "Game Controller",
        category: "Media",
        svg_data: include_str!("../assets/icons/gamepad.svg"),
    },
    BundledIcon {
        id: "palette",
        name: "Palette / Art",
        category: "Media",
        svg_data: include_str!("../assets/icons/palette.svg"),
    },
    BundledIcon {
        id: "sparkles",
        name: "Sparkles",
        category: "Media",
        svg_data: include_str!("../assets/icons/sparkles.svg"),
    },
    BundledIcon {
        id: "star",
        name: "Star",
        category: "Media",
        svg_data: include_str!("../assets/icons/star.svg"),
    },
    BundledIcon {
        id: "heart",
        name: "Heart",
        category: "Media",
        svg_data: include_str!("../assets/icons/heart.svg"),
    },
    BundledIcon {
        id: "message-square",
        name: "Chat Square",
        category: "Communication",
        svg_data: include_str!("../assets/icons/message-square.svg"),
    },
    BundledIcon {
        id: "message-circle",
        name: "Chat Circle",
        category: "Communication",
        svg_data: include_str!("../assets/icons/message-circle.svg"),
    },
    BundledIcon {
        id: "mail",
        name: "Email",
        category: "Communication",
        svg_data: include_str!("../assets/icons/mail.svg"),
    },
    BundledIcon {
        id: "inbox",
        name: "Inbox",
        category: "Communication",
        svg_data: include_str!("../assets/icons/inbox.svg"),
    },
    BundledIcon {
        id: "send",
        name: "Send",
        category: "Communication",
        svg_data: include_str!("../assets/icons/send.svg"),
    },
    BundledIcon {
        id: "phone",
        name: "Telephone",
        category: "Communication",
        svg_data: include_str!("../assets/icons/phone.svg"),
    },
    BundledIcon {
        id: "phone-call",
        name: "Active Call",
        category: "Communication",
        svg_data: include_str!("../assets/icons/phone-call.svg"),
    },
    BundledIcon {
        id: "users",
        name: "Users Group",
        category: "Communication",
        svg_data: include_str!("../assets/icons/users.svg"),
    },
    BundledIcon {
        id: "user",
        name: "User Profile",
        category: "Communication",
        svg_data: include_str!("../assets/icons/user.svg"),
    },
    BundledIcon {
        id: "user-check",
        name: "Verified User",
        category: "Communication",
        svg_data: include_str!("../assets/icons/user-check.svg"),
    },
    BundledIcon {
        id: "user-plus",
        name: "Add User",
        category: "Communication",
        svg_data: include_str!("../assets/icons/user-plus.svg"),
    },
    BundledIcon {
        id: "bell",
        name: "Bell",
        category: "Communication",
        svg_data: include_str!("../assets/icons/bell.svg"),
    },
    BundledIcon {
        id: "bell-ring",
        name: "Ringing Bell",
        category: "Communication",
        svg_data: include_str!("../assets/icons/bell-ring.svg"),
    },
    BundledIcon {
        id: "share",
        name: "Share Network",
        category: "Communication",
        svg_data: include_str!("../assets/icons/share.svg"),
    },
    BundledIcon {
        id: "share-2",
        name: "Share Nodes",
        category: "Communication",
        svg_data: include_str!("../assets/icons/share-2.svg"),
    },
    BundledIcon {
        id: "chat",
        name: "Chat Bubbles",
        category: "Communication",
        svg_data: include_str!("../assets/icons/chat.svg"),
    },
    BundledIcon {
        id: "hash",
        name: "Hashtag",
        category: "Communication",
        svg_data: include_str!("../assets/icons/hash.svg"),
    },
    BundledIcon {
        id: "at-sign",
        name: "At Sign",
        category: "Communication",
        svg_data: include_str!("../assets/icons/at-sign.svg"),
    },
    BundledIcon {
        id: "thumbs-up",
        name: "Thumbs Up",
        category: "Communication",
        svg_data: include_str!("../assets/icons/thumbs-up.svg"),
    },
    BundledIcon {
        id: "flame",
        name: "Flame / Hot",
        category: "Communication",
        svg_data: include_str!("../assets/icons/flame.svg"),
    },
    BundledIcon {
        id: "calendar",
        name: "Calendar",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/calendar.svg"),
    },
    BundledIcon {
        id: "clock",
        name: "Clock Time",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/clock.svg"),
    },
    BundledIcon {
        id: "check-square",
        name: "Check Square",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/check-square.svg"),
    },
    BundledIcon {
        id: "check-circle",
        name: "Check Circle",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/check-circle.svg"),
    },
    BundledIcon {
        id: "clipboard",
        name: "Clipboard",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/clipboard.svg"),
    },
    BundledIcon {
        id: "edit",
        name: "Edit Pencil",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/edit.svg"),
    },
    BundledIcon {
        id: "book",
        name: "Book",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/book.svg"),
    },
    BundledIcon {
        id: "book-open",
        name: "Open Book",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/book-open.svg"),
    },
    BundledIcon {
        id: "briefcase",
        name: "Briefcase",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/briefcase.svg"),
    },
    BundledIcon {
        id: "calculator",
        name: "Calculator",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/calculator.svg"),
    },
    BundledIcon {
        id: "dollar-sign",
        name: "Finance Dollar",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/dollar-sign.svg"),
    },
    BundledIcon {
        id: "credit-card",
        name: "Credit Card",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/credit-card.svg"),
    },
    BundledIcon {
        id: "pie-chart",
        name: "Pie Chart",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/pie-chart.svg"),
    },
    BundledIcon {
        id: "bar-chart",
        name: "Bar Chart",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/bar-chart.svg"),
    },
    BundledIcon {
        id: "trending-up",
        name: "Trending Up",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/trending-up.svg"),
    },
    BundledIcon {
        id: "award",
        name: "Trophy Award",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/award.svg"),
    },
    BundledIcon {
        id: "coffee",
        name: "Coffee",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/coffee.svg"),
    },
    BundledIcon {
        id: "target",
        name: "Target Bullseye",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/target.svg"),
    },
    BundledIcon {
        id: "zap",
        name: "Lightning Zap",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/zap.svg"),
    },
    BundledIcon {
        id: "zap-off",
        name: "Energy Saver",
        category: "Productivity",
        svg_data: include_str!("../assets/icons/zap-off.svg"),
    },
    BundledIcon {
        id: "monitor",
        name: "Monitor Display",
        category: "System",
        svg_data: include_str!("../assets/icons/monitor.svg"),
    },
    BundledIcon {
        id: "smartphone",
        name: "Smartphone",
        category: "System",
        svg_data: include_str!("../assets/icons/smartphone.svg"),
    },
    BundledIcon {
        id: "tablet",
        name: "Tablet",
        category: "System",
        svg_data: include_str!("../assets/icons/tablet.svg"),
    },
    BundledIcon {
        id: "printer",
        name: "Printer",
        category: "System",
        svg_data: include_str!("../assets/icons/printer.svg"),
    },
    BundledIcon {
        id: "power",
        name: "Power",
        category: "System",
        svg_data: include_str!("../assets/icons/power.svg"),
    },
    BundledIcon {
        id: "refresh",
        name: "Refresh",
        category: "System",
        svg_data: include_str!("../assets/icons/refresh.svg"),
    },
    BundledIcon {
        id: "sun",
        name: "Light Mode",
        category: "System",
        svg_data: include_str!("../assets/icons/sun.svg"),
    },
    BundledIcon {
        id: "moon",
        name: "Dark Mode",
        category: "System",
        svg_data: include_str!("../assets/icons/moon.svg"),
    },
    BundledIcon {
        id: "archive",
        name: "Archive Box",
        category: "System",
        svg_data: include_str!("../assets/icons/archive.svg"),
    },
    BundledIcon {
        id: "trash",
        name: "Trash Bin",
        category: "System",
        svg_data: include_str!("../assets/icons/trash.svg"),
    },
    BundledIcon {
        id: "help-circle",
        name: "Help Circle",
        category: "System",
        svg_data: include_str!("../assets/icons/help-circle.svg"),
    },
    BundledIcon {
        id: "info",
        name: "Info Circle",
        category: "System",
        svg_data: include_str!("../assets/icons/info.svg"),
    },
    BundledIcon {
        id: "alert-circle",
        name: "Alert Circle",
        category: "System",
        svg_data: include_str!("../assets/icons/alert-circle.svg"),
    },
    BundledIcon {
        id: "alert-triangle",
        name: "Alert Triangle",
        category: "System",
        svg_data: include_str!("../assets/icons/alert-triangle.svg"),
    },
    BundledIcon {
        id: "layout",
        name: "Window Layout",
        category: "System",
        svg_data: include_str!("../assets/icons/layout.svg"),
    },
    BundledIcon {
        id: "grid",
        name: "Grid",
        category: "System",
        svg_data: include_str!("../assets/icons/grid.svg"),
    },
    BundledIcon {
        id: "maximize",
        name: "Maximize",
        category: "System",
        svg_data: include_str!("../assets/icons/maximize.svg"),
    },
    BundledIcon {
        id: "eye",
        name: "Eye / View",
        category: "System",
        svg_data: include_str!("../assets/icons/eye.svg"),
    },
    BundledIcon {
        id: "github",
        name: "GitHub",
        category: "Brands",
        svg_data: include_str!("../assets/icons/github.svg"),
    },
    BundledIcon {
        id: "gitlab",
        name: "GitLab",
        category: "Brands",
        svg_data: include_str!("../assets/icons/gitlab.svg"),
    },
    BundledIcon {
        id: "rust",
        name: "Rust Lang",
        category: "Brands",
        svg_data: include_str!("../assets/icons/rust.svg"),
    },
    BundledIcon {
        id: "python",
        name: "Python",
        category: "Brands",
        svg_data: include_str!("../assets/icons/python.svg"),
    },
    BundledIcon {
        id: "javascript",
        name: "JavaScript",
        category: "Brands",
        svg_data: include_str!("../assets/icons/javascript.svg"),
    },
    BundledIcon {
        id: "docker",
        name: "Docker",
        category: "Brands",
        svg_data: include_str!("../assets/icons/docker.svg"),
    },
    BundledIcon {
        id: "kubernetes",
        name: "Kubernetes",
        category: "Brands",
        svg_data: include_str!("../assets/icons/kubernetes.svg"),
    },
    BundledIcon {
        id: "linux",
        name: "Linux Tux",
        category: "Brands",
        svg_data: include_str!("../assets/icons/linux.svg"),
    },
    BundledIcon {
        id: "apple",
        name: "Apple",
        category: "Brands",
        svg_data: include_str!("../assets/icons/apple.svg"),
    },
    BundledIcon {
        id: "android",
        name: "Android",
        category: "Brands",
        svg_data: include_str!("../assets/icons/android.svg"),
    },
    BundledIcon {
        id: "windows",
        name: "Windows",
        category: "Brands",
        svg_data: include_str!("../assets/icons/windows.svg"),
    },
    BundledIcon {
        id: "youtube",
        name: "YouTube",
        category: "Brands",
        svg_data: include_str!("../assets/icons/youtube.svg"),
    },
    BundledIcon {
        id: "spotify",
        name: "Spotify",
        category: "Brands",
        svg_data: include_str!("../assets/icons/spotify.svg"),
    },
    BundledIcon {
        id: "discord",
        name: "Discord",
        category: "Brands",
        svg_data: include_str!("../assets/icons/discord.svg"),
    },
    BundledIcon {
        id: "whatsapp",
        name: "WhatsApp",
        category: "Brands",
        svg_data: include_str!("../assets/icons/whatsapp.svg"),
    },
    BundledIcon {
        id: "slack",
        name: "Slack",
        category: "Brands",
        svg_data: include_str!("../assets/icons/slack.svg"),
    },
    BundledIcon {
        id: "telegram",
        name: "Telegram",
        category: "Brands",
        svg_data: include_str!("../assets/icons/telegram.svg"),
    },
    BundledIcon {
        id: "twitter-x",
        name: "X / Twitter",
        category: "Brands",
        svg_data: include_str!("../assets/icons/twitter-x.svg"),
    },
    BundledIcon {
        id: "reddit",
        name: "Reddit",
        category: "Brands",
        svg_data: include_str!("../assets/icons/reddit.svg"),
    },
    BundledIcon {
        id: "google",
        name: "Google",
        category: "Brands",
        svg_data: include_str!("../assets/icons/google.svg"),
    },
    BundledIcon {
        id: "figma",
        name: "Figma",
        category: "Brands",
        svg_data: include_str!("../assets/icons/figma.svg"),
    },
    BundledIcon {
        id: "notion",
        name: "Notion",
        category: "Brands",
        svg_data: include_str!("../assets/icons/notion.svg"),
    },
    BundledIcon {
        id: "obsidian",
        name: "Obsidian",
        category: "Brands",
        svg_data: include_str!("../assets/icons/obsidian.svg"),
    },
    BundledIcon {
        id: "vscode",
        name: "VS Code",
        category: "Brands",
        svg_data: include_str!("../assets/icons/vscode.svg"),
    },
    BundledIcon {
        id: "claude",
        name: "Claude / Anthropic",
        category: "Brands",
        svg_data: include_str!("../assets/icons/claude.svg"),
    },
    BundledIcon {
        id: "chatgpt",
        name: "ChatGPT",
        category: "Brands",
        svg_data: include_str!("../assets/icons/chatgpt.svg"),
    },
];

/// Returns all available icon categories in display order.
pub fn get_icon_categories() -> &'static [&'static str] {
    &["All", "Folders", "Files", "Web", "Development", "Media", "Communication", "Productivity", "System", "Brands"]
}

/// Returns all bundled icons in the registry.
pub fn get_all_bundled_icons() -> &'static [BundledIcon] {
    BUNDLED_ICONS
}

/// Looks up a bundled icon by identifier (case-insensitive).
pub fn get_bundled_icon(id: &str) -> Option<&'static BundledIcon> {
    let clean = id.trim();
    BUNDLED_ICONS.iter().find(|i| i.id.eq_ignore_ascii_case(clean))
}

/// Looks up the raw SVG string for a bundled icon ID.
pub fn get_bundled_icon_svg(id: &str) -> Option<&'static str> {
    get_bundled_icon(id).map(|i| i.svg_data)
}

thread_local! {
    static ICON_CACHE: RefCell<HashMap<(String, i32), Option<Pixbuf>>> = RefCell::new(HashMap::new());
}

/// Renders a bundled icon into a GTK Pixbuf at the requested pixel size.
pub fn get_bundled_icon_pixbuf(id: &str, size: i32) -> Option<Pixbuf> {
    let key = (id.to_string(), size);
    ICON_CACHE.with(|cache| {
        let mut map = cache.borrow_mut();
        if let Some(cached) = map.get(&key) {
            return cached.clone();
        }
        let svg = get_bundled_icon_svg(id)?;
        let pixbuf = load_svg_pixbuf(svg, size);
        map.insert(key, pixbuf.clone());
        pixbuf
    })
}

/// Loads an SVG string into a Pixbuf at the requested size.
pub fn load_svg_pixbuf(svg_data: &str, size: i32) -> Option<Pixbuf> {
    let loader = PixbufLoader::with_type("svg").unwrap_or_else(|_| PixbufLoader::new());
    loader.set_size(size, size);
    if loader.write(svg_data.as_bytes()).is_ok() && loader.close().is_ok() {
        loader.pixbuf()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundled_icons_registry() {
        assert!(BUNDLED_ICONS.len() >= 100, "Expected >= 100 icons, found {}", BUNDLED_ICONS.len());
        for icon in BUNDLED_ICONS {
            assert!(!icon.id.is_empty(), "Icon ID is empty");
            assert!(!icon.name.is_empty(), "Icon name is empty");
            assert!(!icon.category.is_empty(), "Icon category is empty");
            assert!(icon.svg_data.contains("<svg"), "Invalid SVG for {}", icon.id);
            assert!(icon.svg_data.contains("</svg>"), "Unclosed SVG for {}", icon.id);
        }

        assert!(get_bundled_icon("folder").is_some());
        assert!(get_bundled_icon("folder-code").is_some());
        assert!(get_bundled_icon("globe").is_some());
        assert!(get_bundled_icon("terminal").is_some());
        assert!(get_bundled_icon("github").is_some());
    }
}
