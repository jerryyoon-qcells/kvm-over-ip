//! Linux screen enumeration via the X11 Xlib API.
//!
//! Queries the X11 display server to enumerate connected screens and builds
//! a list of [`MonitorInfo`] records.  If the DISPLAY environment variable is
//! not set or Xlib is unavailable the function returns an appropriate error.
//!
//! # How X11 screen enumeration works (for beginners)
//!
//! In X11 terminology, a *display* is the connection to the X server, and a
//! *screen* is a logical grouping of outputs (monitors) managed by that display.
//! Most modern desktop setups have exactly one screen that spans all physical
//! monitors (multi-monitor is handled by Xrandr at a higher level).
//!
//! The Xlib functions used here:
//!
//! - `XOpenDisplay(null)` — opens a connection to the display named in the
//!   `DISPLAY` environment variable (e.g., `:0` or `:0.0`).  Returns null on
//!   failure.
//! - `XScreenCount(display)` — returns how many logical screens exist.
//! - `XDefaultScreen(display)` — returns the index of the default (primary) screen.
//! - `XDisplayWidth(display, screen_num)` / `XDisplayHeight(...)` — pixel
//!   dimensions of the screen.
//! - `XCloseDisplay(display)` — closes the connection and frees memory.
//!
//! # Xrandr limitation
//!
//! Plain Xlib only knows about logical *screens*, not individual monitor
//! *outputs* (CRTCs).  On a typical desktop with two monitors and a single
//! Xrandr composite screen, `XScreenCount` returns 1 and `XDisplayWidth`
//! returns the total combined width.
//!
//! To enumerate individual monitors properly on Linux, the Xrandr extension
//! (`XRRGetScreenResourcesCurrent`, `XRRGetCrtcInfo`) would be needed.  That
//! is a planned enhancement; the current implementation is correct for the
//! common single-screen-per-X-display setup.
//!
//! # `DISPLAY` environment variable
//!
//! When running as a desktop application the `DISPLAY` variable is set
//! automatically by the desktop session (e.g., `DISPLAY=:0`).  In headless
//! environments (CI, SSH without X forwarding) it is unset and `XOpenDisplay`
//! fails — this is the expected failure mode.

use super::{PlatformScreenEnumerator, ScreenInfoError};
use kvm_core::protocol::messages::MonitorInfo;

/// Linux X11 implementation of [`PlatformScreenEnumerator`].
pub struct LinuxScreenEnumerator;

impl LinuxScreenEnumerator {
    /// Creates a new `LinuxScreenEnumerator`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxScreenEnumerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformScreenEnumerator for LinuxScreenEnumerator {
    fn enumerate_monitors(&self) -> Result<Vec<MonitorInfo>, ScreenInfoError> {
        enumerate_via_xlib()
    }
}

/// Enumerates monitors using Xlib's `XOpenDisplay` and `XScreenCount`.
///
/// # Errors
///
/// Returns [`ScreenInfoError::PlatformError`] if the X11 display cannot be
/// opened or if `DISPLAY` is not set.
#[cfg(target_os = "linux")]
fn enumerate_via_xlib() -> Result<Vec<MonitorInfo>, ScreenInfoError> {
    use std::ffi::CString;
    use x11::xlib;

    // SAFETY: XOpenDisplay is called with a null-terminated display string.
    // Passing null as the display name means "use the DISPLAY environment variable".
    // The returned pointer must be freed by XCloseDisplay.
    let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };

    if display.is_null() {
        // XOpenDisplay failed — most likely DISPLAY is not set.
        let display_env = std::env::var("DISPLAY").unwrap_or_else(|_| "<unset>".to_string());
        return Err(ScreenInfoError::PlatformError(format!(
            "XOpenDisplay failed; DISPLAY={display_env}"
        )));
    }

    // SAFETY: `display` is a valid non-null pointer returned by XOpenDisplay.
    let screen_count = unsafe { xlib::XScreenCount(display) };
    let default_screen = unsafe { xlib::XDefaultScreen(display) };

    let mut monitors = Vec::with_capacity(screen_count as usize);

    for screen_num in 0..screen_count {
        // SAFETY: screen_num is in [0, screen_count), which is the valid range.
        let width = unsafe { xlib::XDisplayWidth(display, screen_num) } as u32;
        let height = unsafe { xlib::XDisplayHeight(display, screen_num) } as u32;

        // Xlib does not expose per-screen pixel offsets without Xrandr.
        // For a single-screen setup (the common case) the offset is always (0, 0).
        // A future Xrandr implementation would fill these in properly.
        let x_offset = 0i32;
        let y_offset = 0i32;

        monitors.push(MonitorInfo {
            monitor_id: screen_num as u8,
            width,
            height,
            x_offset,
            y_offset,
            scale_factor: 100,
            // The default screen is treated as the primary monitor.
            is_primary: screen_num == default_screen,
        });
    }

    // SAFETY: `display` was successfully opened above and is not used after this.
    unsafe { xlib::XCloseDisplay(display) };

    if monitors.is_empty() {
        return Err(ScreenInfoError::PlatformError(
            "X11 reported zero screens".to_string(),
        ));
    }

    Ok(monitors)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke-test: if a DISPLAY is available this must succeed and return at
    /// least one monitor.  If DISPLAY is unset the error is expected.
    #[test]
    fn test_linux_screen_enumerator_smoke() {
        let enumerator = LinuxScreenEnumerator::new();
        let result = enumerator.enumerate_monitors();

        if std::env::var("DISPLAY").is_ok() {
            // DISPLAY is set: enumeration must succeed
            assert!(result.is_ok(), "enumerate must succeed when DISPLAY is set");
            let monitors = result.unwrap();
            assert!(!monitors.is_empty(), "must return at least one monitor");
        } else {
            // DISPLAY not set: expect a PlatformError
            assert!(
                result.is_err(),
                "enumerate must fail when DISPLAY is not set"
            );
        }
    }

    #[test]
    fn test_linux_screen_enumerator_monitor_ids_are_sequential_when_display_available() {
        let enumerator = LinuxScreenEnumerator::new();
        if let Ok(monitors) = enumerator.enumerate_monitors() {
            for (i, m) in monitors.iter().enumerate() {
                assert_eq!(
                    m.monitor_id as usize, i,
                    "monitor_id must match screen index"
                );
            }
        }
    }
}
