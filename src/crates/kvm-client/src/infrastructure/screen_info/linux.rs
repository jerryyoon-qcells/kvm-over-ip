//! Linux screen enumeration via the X11 Xlib API.
//!
//! Queries the X11 display server to enumerate connected screens and builds
//! a list of [`MonitorInfo`] records.  If the DISPLAY environment variable is
//! not set or Xlib is unavailable the function returns an appropriate error.
//!
//! # Implementation notes
//!
//! This implementation uses the plain Xlib screen API which is always
//! available without Xrandr.  A future enhancement can add Xrandr support for
//! proper multi-monitor layouts when multiple CRTC outputs are attached to a
//! single X screen.

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
    // The returned pointer must be freed by XCloseDisplay.
    let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };

    if display.is_null() {
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
        // SAFETY: screen_num is in [0, screen_count).
        let width = unsafe { xlib::XDisplayWidth(display, screen_num) } as u32;
        let height = unsafe { xlib::XDisplayHeight(display, screen_num) } as u32;

        // Xlib does not expose per-screen offsets without Xrandr.
        // For a single-screen setup (common case) the offset is always (0, 0).
        let x_offset = 0i32;
        let y_offset = 0i32;

        monitors.push(MonitorInfo {
            monitor_id: screen_num as u8,
            width,
            height,
            x_offset,
            y_offset,
            scale_factor: 100,
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
