//! macOS screen enumeration via Core Graphics (`CGDisplay`).
//!
//! Uses `CGGetActiveDisplayList` to enumerate all active displays and
//! `CGDisplayBounds` to obtain each display's position and size in the
//! global coordinate space.
//!
//! # Implementation notes
//!
//! The Core Graphics coordinate origin is at the bottom-left of the primary
//! display.  We convert to a top-left origin by flipping the Y axis relative
//! to the main display height so that coordinates are consistent with the
//! Windows and Linux conventions used in the protocol.

use super::{PlatformScreenEnumerator, ScreenInfoError};
use kvm_core::protocol::messages::MonitorInfo;

/// macOS implementation of [`PlatformScreenEnumerator`] via Core Graphics.
pub struct MacosScreenEnumerator;

impl MacosScreenEnumerator {
    /// Creates a new `MacosScreenEnumerator`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacosScreenEnumerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformScreenEnumerator for MacosScreenEnumerator {
    fn enumerate_monitors(&self) -> Result<Vec<MonitorInfo>, ScreenInfoError> {
        enumerate_via_core_graphics()
    }
}

/// Enumerates displays using Core Graphics APIs.
///
/// # Errors
///
/// Returns [`ScreenInfoError::PlatformError`] if the Core Graphics call fails
/// or returns zero active displays.
#[cfg(target_os = "macos")]
fn enumerate_via_core_graphics() -> Result<Vec<MonitorInfo>, ScreenInfoError> {
    use core_graphics::display::{CGDisplay, CGDisplayBounds};

    let active_displays =
        CGDisplay::active_displays().map_err(|e| ScreenInfoError::PlatformError(e.to_string()))?;

    if active_displays.is_empty() {
        return Err(ScreenInfoError::PlatformError(
            "CGGetActiveDisplayList returned zero displays".to_string(),
        ));
    }

    // Determine the primary display height for Y-axis flip.
    let primary_id = CGDisplay::main().id;
    let primary_bounds = CGDisplayBounds(primary_id);
    let primary_height = primary_bounds.size.height as i32;

    let mut monitors: Vec<MonitorInfo> = active_displays
        .iter()
        .enumerate()
        .map(|(i, &display_id)| {
            let bounds = CGDisplayBounds(display_id);
            let width = bounds.size.width as u32;
            let height = bounds.size.height as u32;
            let x_offset = bounds.origin.x as i32;
            // Flip Y from bottom-left to top-left origin.
            let y_offset = primary_height - (bounds.origin.y as i32) - (height as i32);

            MonitorInfo {
                monitor_id: i as u8,
                width,
                height,
                x_offset,
                y_offset,
                scale_factor: 100, // Retina scaling handled at higher level
                is_primary: display_id == primary_id,
            }
        })
        .collect();

    // Ensure primary is first.
    monitors.sort_by_key(|m| !m.is_primary);

    // Reassign sequential monitor_ids after sorting.
    for (i, m) in monitors.iter_mut().enumerate() {
        m.monitor_id = i as u8;
    }

    Ok(monitors)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke-test: on a macOS machine with at least one display this must succeed.
    #[test]
    fn test_macos_screen_enumerator_returns_at_least_one_display() {
        let enumerator = MacosScreenEnumerator::new();
        let result = enumerator.enumerate_monitors();
        assert!(
            result.is_ok(),
            "enumerate_monitors must succeed on macOS: {:?}",
            result.err()
        );
        let monitors = result.unwrap();
        assert!(!monitors.is_empty(), "must return at least one display");
    }

    #[test]
    fn test_macos_screen_enumerator_primary_display_is_first() {
        let enumerator = MacosScreenEnumerator::new();
        let monitors = enumerator.enumerate_monitors().expect("enumerate");
        if !monitors.is_empty() {
            assert!(
                monitors[0].is_primary,
                "first entry must be the primary display"
            );
        }
    }

    #[test]
    fn test_macos_screen_enumerator_monitor_ids_are_sequential() {
        let enumerator = MacosScreenEnumerator::new();
        let monitors = enumerator.enumerate_monitors().expect("enumerate");
        for (i, m) in monitors.iter().enumerate() {
            assert_eq!(
                m.monitor_id as usize, i,
                "monitor_id must be sequential after sort"
            );
        }
    }
}
