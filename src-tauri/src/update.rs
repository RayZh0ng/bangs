//! Whether a newer Bangs has been released. Releases live on GitHub, so one
//! API call answers it; the tray menu is the only place the answer shows.

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

/// The repository releases are published from; every URL here derives from it.
const REPO: &str = "gxlself/bangs";
/// This build, from Cargo.toml — which the release script keeps in step with
/// package.json and tauri.conf.json.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const FIRST_CHECK: Duration = Duration::from_secs(8);
const EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Default)]
pub struct UpdateState {
    /// The newest released version, once a check has answered.
    latest: Mutex<Option<String>>,
}

impl UpdateState {
    fn set(&self, latest: Option<String>) {
        *self.latest.lock().unwrap() = latest;
    }

    pub fn latest(&self) -> Option<String> {
        self.latest.lock().unwrap().clone()
    }
}

/// The tray entry: this build, and what is waiting on GitHub.
pub fn menu_label(app: &AppHandle) -> String {
    match app.state::<UpdateState>().latest() {
        Some(latest) if is_newer(&latest, VERSION) => format!("有新版本 v{latest} — 去下载"),
        Some(_) => format!("已是最新 v{VERSION}"),
        None => format!("版本 v{VERSION}"),
    }
}

/// Clicking that entry opens the release page — downloads are manual, so the
/// app never replaces itself behind the user's back.
pub fn open_releases(app: &AppHandle) {
    let page = format!("https://github.com/{REPO}/releases/latest");
    if let Err(error) = app.opener().open_url(page, None::<&str>) {
        eprintln!("[update] could not open the release page: {error}");
    }
}

pub fn start(app: AppHandle) {
    thread::spawn(move || {
        thread::sleep(FIRST_CHECK);
        loop {
            refresh(&app);
            thread::sleep(EVERY);
        }
    });
}

/// Asks GitHub once and updates the tray if the answer changed.
pub fn refresh(app: &AppHandle) {
    let latest = fetch();
    if latest.is_none() {
        return;
    }
    let state = app.state::<UpdateState>();
    if state.latest() == latest {
        return;
    }
    state.set(latest);
    crate::tray::refresh(app);
}

fn fetch() -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(format!("Bangs/{VERSION}"))
        .build()
        .ok()?;
    let response = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.json::<serde_json::Value>());
    match response {
        // No releases yet answers 404, which is not worth logging.
        Ok(release) => Some(release["tag_name"].as_str()?.trim_start_matches('v').to_string()),
        Err(error) => {
            if !error.is_status() {
                eprintln!("[update] check failed: {error}");
            }
            None
        }
    }
}

/// Compares dotted versions numerically, so 0.10.0 beats 0.9.0.
fn is_newer(candidate: &str, current: &str) -> bool {
    let parts = |version: &str| {
        version
            .split(['.', '-', '+'])
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .take(3)
            .collect::<Vec<_>>()
    };
    let (candidate, current) = (parts(candidate), parts(current));
    for index in 0..3 {
        let (left, right) = (candidate.get(index), current.get(index));
        if left != right {
            return left.copied().unwrap_or(0) > right.copied().unwrap_or(0);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_numerically() {
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("0.1.0-beta", "0.1.0"));
    }
}
