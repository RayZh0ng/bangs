#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
pub use mac::*;

#[cfg(windows)]
mod win;
#[cfg(windows)]
mod win_drop;
#[cfg(windows)]
pub use win::*;
#[cfg(windows)]
pub use win_drop::set_catching;

/// Hardware notch and menu bar measurements of a monitor, in logical px.
#[derive(Debug, Clone, Copy, Default)]
pub struct NotchMetrics {
    pub has_notch: bool,
    pub notch_width: f64,
    pub notch_height: f64,
    pub menu_bar_height: f64,
}
