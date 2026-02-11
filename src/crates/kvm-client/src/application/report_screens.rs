//! ScreenInfoService: enumerates monitors and reports configuration to the master.

use kvm_core::protocol::messages::{MonitorInfo, ScreenInfoMessage};
use thiserror::Error;

/// Error type for screen enumeration.
#[derive(Debug, Error)]
pub enum ScreenError {
    #[error("platform error: {0}")]
    Platform(String),
}

/// Trait for enumerating screens on the current platform.
pub trait ScreenEnumerator: Send + Sync {
    /// Returns a [`ScreenInfoMessage`] describing all connected monitors.
    ///
    /// # Errors
    ///
    /// Returns [`ScreenError`] if monitor information cannot be retrieved.
    fn enumerate_screens(&self) -> Result<ScreenInfoMessage, ScreenError>;
}

/// Detects whether two [`ScreenInfoMessage`]s represent the same configuration.
pub fn screen_info_changed(old: &ScreenInfoMessage, new: &ScreenInfoMessage) -> bool {
    old != new
}

#[cfg(test)]
mod tests {
    use super::*;

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
