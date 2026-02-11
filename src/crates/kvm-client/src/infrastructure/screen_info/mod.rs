//! Platform-specific screen / monitor enumeration.
//!
//! Detects the connected monitors and their resolutions so the client can
//! send accurate [`ScreenInfoMessage`] reports to the master.
//!
//! Each platform implements the [`PlatformScreenEnumerator`] trait; the
//! correct implementation is selected at compile time.

use kvm_core::protocol::messages::{MonitorInfo, ScreenInfoMessage};
use thiserror::Error;

/// Error type for screen enumeration operations.
#[derive(Debug, Error)]
pub enum ScreenInfoError {
    /// The platform API call to enumerate monitors failed.
    #[error("platform API error while enumerating monitors: {0}")]
    PlatformError(String),
}

/// Trait for enumerating monitors on the current platform.
///
/// Implementors query the OS and return an ordered list of `MonitorInfo`
/// records.  The primary monitor is always index 0.
pub trait PlatformScreenEnumerator: Send + Sync {
    /// Returns the list of connected monitors.
    ///
    /// # Errors
    ///
    /// Returns [`ScreenInfoError::PlatformError`] if the OS API call fails.
    fn enumerate_monitors(&self) -> Result<Vec<MonitorInfo>, ScreenInfoError>;
}

/// Builds a [`ScreenInfoMessage`] using the provided enumerator.
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

#[cfg(target_os = "windows")]
pub use windows::WindowsScreenEnumerator as NativeScreenEnumerator;

// ── Linux implementation ──────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "linux")]
pub use linux::LinuxScreenEnumerator as NativeScreenEnumerator;

// ── macOS implementation ──────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::MacosScreenEnumerator as NativeScreenEnumerator;

// ── Mock implementation (always compiled for tests) ───────────────────────────

/// A mock screen enumerator that returns a configurable list of monitors.
/// Used in tests and on unsupported platforms.
pub struct MockScreenEnumerator {
    pub monitors: Vec<MonitorInfo>,
}

impl MockScreenEnumerator {
    /// Creates a `MockScreenEnumerator` with a single 1920×1080 primary monitor.
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
