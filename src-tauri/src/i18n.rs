//! Bangs speaks Chinese or English. It follows the system by default; the tray
//! menu can pin it to one language.
//!
//! Call sites pass both texts rather than a key, so the words stay where they
//! are read: `t("退出", "Quit")`.

use std::sync::atomic::{AtomicBool, Ordering};

static CHINESE: AtomicBool = AtomicBool::new(true);

pub fn t(zh: &'static str, en: &'static str) -> &'static str {
    if chinese() {
        zh
    } else {
        en
    }
}

pub fn chinese() -> bool {
    CHINESE.load(Ordering::Relaxed)
}

/// The language actually in use, for the webview: WKWebView reports the app's
/// own language rather than the system's, so it cannot work this out itself.
pub fn code() -> &'static str {
    t("zh", "en")
}

/// `Some("zh")`, `Some("en")`, or `None` to follow the system.
pub fn apply(preference: Option<&str>) {
    let chinese = match preference {
        Some("zh") => true,
        Some("en") => false,
        _ => crate::platform::system_language().to_lowercase().starts_with("zh"),
    };
    CHINESE.store(chinese, Ordering::Relaxed);
}
