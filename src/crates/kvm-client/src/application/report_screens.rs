//! ScreenInfoService: enumerates monitors and reports configuration to the master.
//!
//! # Purpose
//!
//! When the client connects to the master, the master needs to know the size and
//! arrangement of the client's monitors so it can:
//!
//! 1. Set up the virtual layout (which part of the master screen maps to which
//!    part of the client screen).
//! 2. Scale mouse coordinates proportionally when moving the cursor between screens.
//!
//! This module owns the logic for detecting "did the monitor configuration change?"
//! so the client can re-report whenever the user plugs in or removes a monitor.
//!
//! # Data flow
//!
//! ```text
//! OS display API (Win32 / X11 / Core Graphics)
//!   └─ ScreenEnumerator::enumerate_screens()
//!        └─ ScreenInfoMessage { monitors: [MonitorInfo { width, height, x_offset, ... }] }
//!             └─ ClientConnection::send_screen_info(msg)
//!                  └─ TCP → master
//! ```
//!
//! # Change detection
//!
//! The client polls for monitor changes periodically (e.g., every 5 seconds).
//! [`screen_info_changed`] compares the new reading against the last known
//! configuration.  Only when a difference is detected does the client send a
//! new report, avoiding unnecessary network traffic.

use kvm_core::protocol::messages::ScreenInfoMessage;
use thiserror::Error;

/// Error type for screen enumeration.
#[derive(Debug, Error)]
pub enum ScreenError {
    /// The OS API call to enumerate monitors failed.
    ///
    /// This can happen if the display server is unavailable (e.g., no DISPLAY
    /// environment variable on Linux) or if the Win32 call returns no monitors.
    #[error("platform error: {0}")]
    Platform(String),
}

/// Trait for enumerating screens on the current platform.
///
/// Each supported OS provides an implementation in the `screen_info`
/// infrastructure module.  A [`MockScreenEnumerator`] is also provided for tests.
///
/// This trait is declared here in the *application* layer so that the
/// application layer does not import from the infrastructure layer — keeping the
/// Clean Architecture dependency rule intact.
pub trait ScreenEnumerator: Send + Sync {
    /// Returns a [`ScreenInfoMessage`] describing all connected monitors.
    ///
    /// The returned message contains one [`MonitorInfo`] per physical display,
    /// each with its pixel width, height, and position relative to the primary
    /// monitor (top-left origin, positive X rightwards, positive Y downwards).
    ///
    /// # Errors
    ///
    /// Returns [`ScreenError`] if monitor information cannot be retrieved.
    fn enumerate_screens(&self) -> Result<ScreenInfoMessage, ScreenError>;
}

/// Detects whether two [`ScreenInfoMessage`]s represent the same configuration.
///
/// Returns `true` if the configurations differ, `false` if they are identical.
///
/// # When to use
///
/// Call this after every periodic poll.  If it returns `true`, send the new
/// `ScreenInfoMessage` to the master; otherwise skip the network round trip.
///
/// # Example
///
/// ```ignore
/// let new_info = enumerator.enumerate_screens()?;
/// if screen_info_changed(&last_known, &new_info) {
///     connection.send_screen_info(new_info.clone()).await;
///     last_known = new_info;
/// }
/// ```
pub fn screen_info_changed(old: &ScreenInfoMessage, new: &ScreenInfoMessage) -> bool {
    old != new
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: builds a single-monitor ScreenInfoMessage with the given dimensions.
    fn make_screen_info(w: u32, h: u32) -> ScreenInfoMessage {
        ScreenInfoMessage {
            monitors: vec![MonitorInfo {
                monitor_id: 0,
                x_offset: 0,
                y_offset: 0,
                width: w,
                height: h,
                scale_factor: 100,
                is_primary: true,
            }],
        }
    }

    #[test]
    fn test_screen_info_changed_returns_false_for_identical_configurations() {
        let a = make_screen_info(1920, 1080);
        let b = make_screen_info(1920, 1080);
        assert!(!screen_info_changed(&a, &b));
    }

    #[test]
    fn test_screen_info_changed_returns_true_when_resolution_differs() {
        let a = make_screen_info(1920, 1080);
        let b = make_screen_info(2560, 1440);
        assert!(screen_info_changed(&a, &b));
    }

    #[test]
    fn test_screen_info_changed_returns_true_when_monitor_count_differs() {
        let a = make_screen_info(1920, 1080);
        let b = ScreenInfoMessage {
            monitors: vec![
                MonitorInfo {
                    monitor_id: 0,
                    x_offset: 0,
                    y_offset: 0,
                    width: 1920,
                    height: 1080,
                    scale_factor: 100,
                    is_primary: true,
                },
                MonitorInfo {
                    monitor_id: 1,
                    x_offset: 1920,
                    y_offset: 0,
                    width: 1920,
                    height: 1080,
                    scale_factor: 100,
                    is_primary: false,
                },
            ],
        };
        assert!(screen_info_changed(&a, &b));
    }
}
