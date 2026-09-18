//! Auto-fetching and local caching of website favicons for Hyprvyl (Stage 4).
//!
//! When URL items are added or displayed:
//! - Extracts the domain name.
//! - Checks local cache at `~/.cache/hyprvyl/favicons/<hash>.png`.
//! - If uncached and `auto_fetch_favicons = true`, spawns a detached background thread
//!   to fetch `https://<domain>/favicon.ico` or scrape HTML `<link rel="icon">`.
//! - Converts downloaded favicon to PNG and saves it in the local disk cache.
//! - Completely non-blocking with zero UI or daemon startup latency.

use gdk_pixbuf::PixbufLoader;
use gtk4::prelude::*;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::Duration;

static IN_FLIGHT_FETCHES: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static FAILED_FETCHES: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// Clears in-flight and failed fetch tracking (used on --reload).
pub fn clear_favicon_fetch_state() {
    if let Ok(mut in_flight) = IN_FLIGHT_FETCHES.lock() {
        in_flight.clear();
    }
    if let Ok(mut failed) = FAILED_FETCHES.lock() {
        failed.clear();
    }
}

/// Returns the standard favicon cache directory: ~/.cache/hyprvyl/favicons
pub fn get_favicon_cache_dir() -> PathBuf {
    let base = match env::var("XDG_CACHE_HOME") {
        Ok(val) if !val.is_empty() => PathBuf::from(val),
        _ => {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".cache")
        }
    };
    base.join("hyprvyl").join("favicons")
}

/// Computes a stable hash string for a domain.
pub fn hash_domain(domain: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let clean = domain.trim().to_lowercase();
    let mut hasher = DefaultHasher::new();
    clean.hash(&mut hasher);
    let hash_val = hasher.finish();

    // Clean name prefix + 64-bit hex hash
    let safe_prefix: String = clean
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .take(24)
        .collect();
    format!("{}_{:016x}.png", safe_prefix, hash_val)
}

/// Returns the cache file path for a domain.
pub fn get_cached_favicon_path(domain: &str) -> PathBuf {
    let filename = hash_domain(domain);
    get_favicon_cache_dir().join(filename)
}

/// Parses the domain name (hostname) from a URL string.
pub fn parse_domain(url_str: &str) -> Option<String> {
    let trimmed = url_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_scheme = if let Some(stripped) = trimmed.strip_prefix("https://") {
        stripped
    } else if let Some(stripped) = trimmed.strip_prefix("http://") {
        stripped
    } else if let Some(stripped) = trimmed.strip_prefix("//") {
        stripped
    } else {
        trimmed
    };

    let host_part = without_scheme
        .split('/')
        .next()?
        .split('?')
        .next()?
        .split('#')
        .next()?
        .split(':')
        .next()?;

    let domain = host_part.trim().to_lowercase();
    if domain.is_empty() || !domain.contains('.') {
        // Must have at least one dot or be localhost
        if domain == "localhost" {
            Some("localhost".to_string())
        } else {
            None
        }
    } else {
        Some(domain)
    }
}

/// Extracts favicon href from HTML source code using regex patterns for <link rel="...icon" href="...">.
pub fn extract_icon_href_from_html(html: &str) -> Option<String> {
    let lower_html = html.to_lowercase();
    
    // Look for <link ... rel="icon" ...> or <link ... rel="shortcut icon" ...> or <link ... rel="apple-touch-icon" ...>
    let link_tags = find_all_link_tags(&lower_html, html);
    for tag in link_tags {
        let tag_lower = tag.to_lowercase();
        if tag_lower.contains("rel=")
            && (tag_lower.contains("icon") || tag_lower.contains("shortcut icon") || tag_lower.contains("apple-touch-icon"))
            && let Some(href) = extract_attribute_value(tag, "href")
        {
            let trimmed_href = href.trim();
            if !trimmed_href.is_empty() && !trimmed_href.starts_with("data:") {
                return Some(trimmed_href.to_string());
            }
        }
    }

    None
}

/// Helper to parse all <link ...> tags from an HTML document.
fn find_all_link_tags<'a>(lower_html: &str, original_html: &'a str) -> Vec<&'a str> {
    let mut tags = Vec::new();
    let mut start = 0;

    while let Some(link_pos) = lower_html[start..].find("<link") {
        let abs_start = start + link_pos;
        if let Some(end_pos) = original_html[abs_start..].find('>') {
            let abs_end = abs_start + end_pos + 1;
            tags.push(&original_html[abs_start..abs_end]);
            start = abs_end;
        } else {
            break;
        }
    }

    tags
}

/// Helper to extract an attribute value from an HTML tag string.
fn extract_attribute_value(tag: &str, attr_name: &str) -> Option<String> {
    let lower_tag = tag.to_lowercase();
    let search = format!("{}=", attr_name);
    let attr_pos = lower_tag.find(&search)?;
    let val_start = attr_pos + search.len();
    let chars: Vec<char> = tag[val_start..].chars().collect();
    if chars.is_empty() {
        return None;
    }

    let quote = chars[0];
    if quote == '"' || quote == '\'' {
        let mut result = String::new();
        for &c in &chars[1..] {
            if c == quote {
                break;
            }
            result.push(c);
        }
        Some(result)
    } else {
        let mut result = String::new();
        for &c in &chars {
            if c.is_whitespace() || c == '>' {
                break;
            }
            result.push(c);
        }
        Some(result)
    }
}

/// Resolves a relative or absolute favicon URL against a base domain.
pub fn resolve_relative_url(base_domain: &str, href: &str) -> String {
    let trimmed = href.trim();
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        trimmed.to_string()
    } else if let Some(stripped) = trimmed.strip_prefix("//") {
        format!("https://{}", stripped)
    } else if let Some(stripped) = trimmed.strip_prefix('/') {
        format!("https://{}/{}", base_domain, stripped)
    } else {
        format!("https://{}/{}", base_domain, trimmed)
    }
}

/// Resolves the favicon path for a URL.
///
/// If cached, returns `Some(path)`.
/// If not cached and `auto_fetch = true`, spawns a detached background thread
/// to fetch and cache the favicon without blocking caller.
pub fn resolve_or_fetch_favicon(url: &str, auto_fetch: bool) -> Option<PathBuf> {
    let domain = parse_domain(url)?;
    let cache_path = get_cached_favicon_path(&domain);

    if cache_path.is_file() {
        // Cache hit
        return Some(cache_path);
    }

    if !auto_fetch {
        return None;
    }

    // Check if already failed or in-flight
    if let Ok(failed) = FAILED_FETCHES.lock()
        && failed.contains(&domain)
    {
        return None;
    }

    if let Ok(mut in_flight) = IN_FLIGHT_FETCHES.lock()
        && !in_flight.insert(domain.clone())
    {
        // Already in-flight
        return None;
    }

    // Spawn background worker thread
    let domain_clone = domain.clone();
    let url_clone = url.to_string();
    let cache_path_clone = cache_path.clone();

    let _ = thread::Builder::new()
        .name(format!("favicon-fetch-{}", domain))
        .spawn(move || {
            let success = fetch_and_cache_favicon_sync(&domain_clone, &url_clone, &cache_path_clone);
            
            if let Ok(mut in_flight) = IN_FLIGHT_FETCHES.lock() {
                in_flight.remove(&domain_clone);
            }

            if !success
                && let Ok(mut failed) = FAILED_FETCHES.lock()
            {
                failed.insert(domain_clone);
            }
        });

    None
}

/// Synchronous worker to fetch and save favicon image bytes.
fn fetch_and_cache_favicon_sync(domain: &str, original_url: &str, target_path: &Path) -> bool {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(4))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0 (Hyprvyl Favicon Fetcher)")
        .build();

    println!("[Hyprvyl Favicon] Starting favicon resolution for domain: '{}' (original URL: '{}')", domain, original_url);

    // 1. First attempt: Direct /favicon.ico fetch
    let direct_url = format!("https://{}/favicon.ico", domain);
    println!("[Hyprvyl Favicon] Attempting Stage 1 direct fetch: {}", direct_url);
    match agent.get(&direct_url).call() {
        Ok(resp) => {
            let status = resp.status();
            println!("[Hyprvyl Favicon] Stage 1 HTTP response status: {} for {}", status, direct_url);
            if status == 200 {
                let mut bytes = Vec::new();
                if resp.into_reader().read_to_end(&mut bytes).is_ok()
                    && is_valid_image_payload(&bytes)
                    && save_image_bytes_as_png(&bytes, target_path)
                {
                    let size = fs::metadata(target_path).map(|m| m.len()).unwrap_or(0);
                    println!(
                        "[Hyprvyl Favicon] Stage 1 SUCCESS: Saved {} bytes to {:?}",
                        size, target_path
                    );
                    return true;
                }
                println!("[Hyprvyl Favicon] Stage 1 payload was not a valid convertable image. Falling through to Stage 2.");
            } else {
                println!("[Hyprvyl Favicon] Stage 1 non-200 status {}. Falling through to Stage 2.", status);
            }
        }
        Err(err) => {
            println!("[Hyprvyl Favicon] Stage 1 request error: {}. Falling through to Stage 2.", err);
        }
    }

    // 2. Second attempt: Fetch HTML page and scrape <link rel="icon">
    let page_url = if original_url.starts_with("http://") || original_url.starts_with("https://") {
        original_url.to_string()
    } else {
        format!("https://{}/", domain)
    };

    println!("[Hyprvyl Favicon] Attempting Stage 2 HTML page scrape: {}", page_url);
    match agent.get(&page_url).call() {
        Ok(resp) => {
            let status = resp.status();
            println!("[Hyprvyl Favicon] Stage 2 HTML page HTTP status: {} for {}", status, page_url);
            if status == 200
                && let Ok(html_text) = resp.into_string()
            {
                if let Some(icon_href) = extract_icon_href_from_html(&html_text) {
                    let resolved_icon_url = resolve_relative_url(domain, &icon_href);
                    println!(
                        "[Hyprvyl Favicon] Stage 2 found icon href '{}', resolved to '{}'",
                        icon_href, resolved_icon_url
                    );
                    match agent.get(&resolved_icon_url).call() {
                        Ok(icon_resp) => {
                            let icon_status = icon_resp.status();
                            println!(
                                "[Hyprvyl Favicon] Stage 2 icon download status: {} for {}",
                                icon_status, resolved_icon_url
                            );
                            if icon_status == 200 {
                                let mut bytes = Vec::new();
                                if icon_resp.into_reader().read_to_end(&mut bytes).is_ok()
                                    && is_valid_image_payload(&bytes)
                                    && save_image_bytes_as_png(&bytes, target_path)
                                {
                                    let size = fs::metadata(target_path).map(|m| m.len()).unwrap_or(0);
                                    println!(
                                        "[Hyprvyl Favicon] Stage 2 SUCCESS: Saved {} bytes to {:?}",
                                        size, target_path
                                    );
                                    return true;
                                }
                            }
                        }
                        Err(err) => {
                            println!("[Hyprvyl Favicon] Stage 2 icon download error: {}", err);
                        }
                    }
                } else {
                    println!("[Hyprvyl Favicon] Stage 2 no <link rel=\"icon\"> tags found in HTML.");
                }
            }
        }
        Err(err) => {
            println!("[Hyprvyl Favicon] Stage 2 HTML fetch error: {}", err);
        }
    }

    println!("[Hyprvyl Favicon] No valid favicon found for domain '{}'. Falling back to bundled icon.", domain);
    false
}

/// Checks if downloaded bytes resemble a valid image (not HTML error pages or empty files).
fn is_valid_image_payload(bytes: &[u8]) -> bool {
    if bytes.len() < 16 {
        return false;
    }
    // Check if HTML document returned instead of image
    let prefix = String::from_utf8_lossy(&bytes[..bytes.len().min(64)]).to_lowercase();
    if prefix.contains("<!doctype") || prefix.contains("<html") || prefix.contains("<head") {
        return false;
    }

    // Common magic bytes:
    // PNG: \x89PNG
    // ICO: \x00\x00\x01\x00
    // GIF: GIF87a / GIF89a
    // JPEG: \xFF\xD8\xFF
    // SVG: <svg / <?xml
    // WEBP: RIFF....WEBP
    true
}

/// Converts raw image bytes (ICO, PNG, SVG, WEBP, JPEG) into PNG and writes to disk.
fn save_image_bytes_as_png(bytes: &[u8], target_path: &Path) -> bool {
    if let Some(parent) = target_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Try loading via gdk_pixbuf PixbufLoader
    let loader = PixbufLoader::new();

    if loader.write(bytes).is_ok()
        && loader.close().is_ok()
        && let Some(pixbuf) = loader.pixbuf()
    {
        // Save as PNG
        let target_str = target_path.to_string_lossy().to_string();
        return pixbuf.savev(&target_str, "png", &[]).is_ok();
    }

    // Fallback: If loader couldn't convert, but bytes are valid PNG, write directly
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return fs::write(target_path, bytes).is_ok();
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_parsing() {
        assert_eq!(parse_domain("https://web.whatsapp.com/").unwrap(), "web.whatsapp.com");
        assert_eq!(parse_domain("http://github.com/rust-lang/rust").unwrap(), "github.com");
        assert_eq!(parse_domain("https://anthropic.com:443/pricing?utm=test").unwrap(), "anthropic.com");
        assert_eq!(parse_domain("youtube.com").unwrap(), "youtube.com");
        assert_eq!(parse_domain("http://localhost:3000/dashboard").unwrap(), "localhost");
        assert_eq!(parse_domain("").is_none(), true);
        assert_eq!(parse_domain("invalid_no_dot").is_none(), true);
    }

    #[test]
    fn test_html_icon_href_extraction() {
        let html_1 = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Test Page</title>
            <link rel="icon" type="image/png" href="/favicon-32x32.png">
        </head>
        <body>Hello</body>
        </html>
        "#;
        assert_eq!(extract_icon_href_from_html(html_1).unwrap(), "/favicon-32x32.png");

        let html_2 = r#"
        <html>
        <head>
            <link href="https://cdn.example.com/assets/shortcut-icon.ico" rel="shortcut icon" />
        </head>
        </html>
        "#;
        assert_eq!(
            extract_icon_href_from_html(html_2).unwrap(),
            "https://cdn.example.com/assets/shortcut-icon.ico"
        );

        let html_3 = r#"
        <html>
        <head>
            <link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">
        </head>
        </html>
        "#;
        assert_eq!(extract_icon_href_from_html(html_3).unwrap(), "/apple-touch-icon.png");
    }

    #[test]
    fn test_relative_url_resolution() {
        assert_eq!(
            resolve_relative_url("example.com", "/favicon.ico"),
            "https://example.com/favicon.ico"
        );
        assert_eq!(
            resolve_relative_url("example.com", "assets/icon.png"),
            "https://example.com/assets/icon.png"
        );
        assert_eq!(
            resolve_relative_url("example.com", "//cdn.example.com/logo.svg"),
            "https://cdn.example.com/logo.svg"
        );
        assert_eq!(
            resolve_relative_url("example.com", "https://cdn.other.com/fav.png"),
            "https://cdn.other.com/fav.png"
        );
    }

    #[test]
    fn test_cache_path_generation() {
        let path = get_cached_favicon_path("anthropic.com");
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("anthropic.com"));
        assert!(path_str.ends_with(".png"));
    }
}
