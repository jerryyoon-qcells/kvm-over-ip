//! Platform-specific screen / monitor enumeration.
//!
//! Detects the connected monitors and their resolutions so the client can
//! send accurate [`ScreenInfoMessage`] reports to the master.
//!
//! # Why does the master need screen information?
//!
//! When the cursor transitions from the master to this client, the master must
//! know:
//! - How large the client's screen is (so it can scale mouse coordinates).
//! - How many monitors the client has and where they are positioned relative to
//!   each other (so the virtual layout can include multi-monitor clients).
//!
//! This module queries the OS for that information and packages it into the
//! [`ScreenInfoMessage`] wire format.
//!
//! # Platform implementations
//!
//! Each platform implements [`PlatformScreenEnumerator`]; the correct one is
//! selected at compile time via `#[cfg(target_os = ...)]` and re-exported as
//! `NativeScreenEnumerator`:
//!
//! | Module    | OS      | API used                                     |
//! |-----------|---------|----------------------------------------------|
//! | `windows` | Windows | `EnumDisplayMonitors` + `GetMonitorInfoW`    |
//! | `linux`   | Linux   | `XOpenDisplay` + `XScreenCount` (Xlib)       |
//! | `macos`   | macOS   | `CGGetActiveDisplayList` + `CGDisplayBounds` |
//!
//! A [`MockScreenEnumerator`] is always compiled (not guarded by `#[cfg]`) so
//! tests on any platform can use it without a physical display.
//!
//! # Usage example
//!
//! ```ignore
//! // In the client's main loop:
//! let enumerator = NativeScreenEnumerator::new();
//! let screen_info = build_screen_info(&enumerator)?;
//! connection.send_screen_info(screen_info).await;
//! ```

use kvm_core::protocol::messages::{MonitorInfo, ScreenInfoMessage};
use thiserror::Error;

/// Error type for screen enumeration operations.
#[derive(Debug, Error)]
pub enum ScreenInfoError {
    /// The platform API call to enumerate monitors failed.
    ///
    /// The inner string contains a human-readable description of the OS error,
    /// e.g., "XOpenDisplay failed; DISPLAY=<unset>" or
    /// "EnumDisplayMonitors returned no monitors".
    #[error("platform API error while enumerating monitors: {0}")]
    PlatformError(String),
}

/// Trait for enumerating monitors on the current platform.
///
/// Implementors query the OS and return an ordered list of `MonitorInfo`
/// records.  The primary monitor is always index 0.
///
/// This trait is defined in the infrastructure layer because it is an OS-facing
/// adapter.  The application layer sees it through the [`ScreenEnumerator`]
/// trait defined in `application::report_screens`.
pub trait PlatformScreenEnumerator: Send + Sync {
    /// Returns the list of connected monitors.
    ///
    /// The first element in the returned `Vec` MUST be the primary monitor
    /// (the monitor that Windows calls the "primary display" or that macOS
    /// calls `CGMainDisplayID`).
    ///
    /// # Errors
    ///
    /// Returns [`ScreenInfoError::PlatformError`] if the OS API call fails.
    fn enumerate_monitors(&self) -> Result<Vec<MonitorInfo>, ScreenInfoError>;
}

/// Builds a [`ScreenInfoMessage`] using the provided enumerator.
///
/// This is a thin adapter that calls `enumerate_monitors` and wraps the result
/// in the protocol message type.  It exists so callers do not need to construct
/// the message manually.
///
/// # Errors
///
/// Propagates any error from the enumerator.
pub fn build_screen_info(
    enumerator: &dyn PlatformScreenEnumerator,
) -> Result<ScreenInfoMessage, ScreenInfoError> {
    let monitors = enumerator.enumerate_monitors()?;
    Ok(ScreenInfoMessage { monitors })
}

// ── Windows implementation ────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub mod windows;

/// Re-export the Windows enumerator as `NativeScreenEnumerator` on Windows.
///
/// This alias lets the rest of the codebase reference `NativeScreenEnumerator`
/// without knowing the OS at compile time — only this module contains the
/// platform-conditional logic.
#[cfg(target_os = "windows")]
pub use windows::WindowsScreenEnumerator as NativeScreenEnumerator;

// ── Linux implementation ──────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
pub mod linux;

/// Re-export the Linux enumerator as `NativeScreenEnumerator` on Linux.
#[cfg(target_os = "linux")]
pub use linux::LinuxScreenEnumerator as NativeScreenEnumerator;

// ── macOS implementation ──────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
pub mod macos;

/// Re-export the macOS enumerator as `NativeScreenEnumerator` on macOS.
#[cfg(target_os = "macos")]
pub use macos::MacosScreenEnumerator as NativeScreenEnumerator;

// ── Mock implementation (always compiled for tests) ───────────────────────────

/// A mock screen enumerator that returns a configurable list of monitors.
///
/// Used in unit tests and on unsupported platforms.  Does not make any OS
/// calls — the monitor list is provided at construction time.
///
/// # Example
///
/// ```ignore
/// let enumerator = MockScreenEnumerator::single_1080p();
/// let info = build_screen_info(&enumerator).unwrap();
/// assert_eq!(info.monitors.len(), 1);
/// ```
pub struct MockScreenEnumerator {
    /// The fixed list of monitors that this enumerator will always return.
    pub monitors: Vec<MonitorInfo>,
}

impl MockScreenEnumerator {
    /// Creates a `MockScreenEnumerator` with a single 1920×1080 primary monitor.
    ///
    /// This is the most common test fixture; it mirrors a typical single-monitor
    /// desktop setup.
    pub fn single_1080p() -> Self {
        Self {
            monitors: vec![MonitorInfo {
                monitor_id: 0,
                width: 1920,
                height: 1080,
                x_offset: 0,
                y_offset: 0,
                scale_factor: 100,
                is_primary: true,
            }],
        }
    }

    /// Creates a `MockScreenEnumerator` with two 2560×1440 monitors side by side.
    ///
    /// The primary monitor is at x_offset=0; the secondary monitor is at
    /// x_offset=2560 (immediately to the right of the primary).
    pub fn dual_1440p() -> Self {
        Self {
            monitors: vec![
                MonitorInfo {
                    monitor_id: 0,
                    width: 2560,
                    height: 1440,
                    x_offset: 0,
                    y_offset: 0,
                    scale_factor: 100,
                    is_primary: true,
                },
                MonitorInfo {
                    monitor_id: 1,
                    width: 2560,
                    height: 1440,
                    x_offset: 2560,
                    y_offset: 0,
                    scale_factor: 100,
                    is_primary: false,
                },
            ],
        }
    }
}

impl PlatformScreenEnumerator for MockScreenEnumerator {
    /// Returns the monitors provided at construction time (never fails).
    fn enumerate_monitors(&self) -> Result<Vec<MonitorInfo>, ScreenInfoError> {
        Ok(self.monitors.clone())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_enumerator_single_1080p_returns_one_monitor() {
        // Arrange
        let enumerator = MockScreenEnumerator::single_1080p();

        // Act
        let monitors = enumerator.enumerate_monitors().expect("enumerate");

        // Assert
        assert_eq!(monitors.len(), 1);
        assert_eq!(monitors[0].width, 1920);
        assert_eq!(monitors[0].height, 1080);
        assert!(monitors[0].is_primary);
    }

    #[test]
    fn test_mock_enumerator_dual_1440p_returns_two_monitors() {
        // Arrange
        let enumerator = MockScreenEnumerator::dual_1440p();

        // Act
        let monitors = enumerator.enumerate_monitors().expect("enumerate");

        // Assert
        assert_eq!(monitors.len(), 2);
        assert!(monitors[0].is_primary);
        assert!(!monitors[1].is_primary);
        assert_eq!(monitors[1].x_offset, 2560);
    }

    #[test]
    fn test_build_screen_info_returns_one_monitor_for_single_1080p() {
        // Arrange
        let enumerator = MockScreenEnumerator::single_1080p();

        // Act
        let info = build_screen_info(&enumerator).expect("build");

        // Assert
        assert_eq!(info.monitors.len(), 1);
    }

    #[test]
    fn test_build_screen_info_with_dual_monitors_includes_all_monitors() {
        // Arrange
        let enumerator = MockScreenEnumerator::dual_1440p();

        // Act
        let info = build_screen_info(&enumerator).expect("build");

        // Assert
        assert_eq!(info.monitors.len(), 2);
    }

    #[test]
    fn test_build_screen_info_primary_monitor_has_monitor_id_zero() {
        // Arrange
        let enumerator = MockScreenEnumerator::dual_1440p();
        let info = build_screen_info(&enumerator).expect("build");

        // Assert
        assert_eq!(info.monitors[0].monitor_id, 0);
    }
}
