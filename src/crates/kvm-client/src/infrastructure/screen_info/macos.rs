//! macOS screen enumeration via Core Graphics (`CGDisplay`).
//!
//! Uses `CGGetActiveDisplayList` to enumerate all active displays and
//! `CGDisplayBounds` to obtain each display's position and size in the
//! global coordinate space.
//!
//! # How Core Graphics display enumeration works (for beginners)
//!
//! Core Graphics (part of the `CoreGraphics` framework, linked via the
//! `core-graphics` crate) provides functions to query the display hardware:
//!
//! - `CGDisplay::active_displays()` — returns a list of `CGDirectDisplayID`
//!   values (opaque u32 handles) for every active display.
//! - `CGDisplayBounds(display_id)` — returns a `CGRect` with the display's
//!   position (`origin`) and size (`size`) in the *global coordinate space*.
//! - `CGDisplay::main()` — returns the primary display (the one with the menu
//!   bar in macOS).
//!
//! # Y-axis flip
//!
//! The Core Graphics coordinate system places the origin at the **bottom-left**
//! of the primary display, with Y increasing upward.  This is the opposite of
//! the convention used in the KVM protocol (and Windows/Linux), which places the
//! origin at the **top-left** with Y increasing downward.
//!
//! To convert from macOS coordinates to the protocol's top-left convention we
//! apply the following formula for each display:
//!
//! ```text
//! protocol_y = primary_height - cg_origin_y - display_height
//! ```
//!
//! For the primary display itself (origin.y = 0), this gives:
//! ```text
//! protocol_y = primary_height - 0 - primary_height = 0  ✓
//! ```
//!
//! For a display positioned directly below the primary (origin.y = -secondary_height):
//! ```text
//! protocol_y = primary_height - (-secondary_height) - secondary_height = primary_height  ✓
//! ```
//!
//! # Primary display first
//!
//! The protocol requires the primary monitor to be at index 0.  After
//! collecting all displays we sort by `is_primary` (descending) so that the
//! primary display comes first, then reassign sequential `monitor_id` values.

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

    // Get the list of all currently active (powered-on) display IDs.
    let active_displays =
        CGDisplay::active_displays().map_err(|e| ScreenInfoError::PlatformError(e.to_string()))?;

    if active_displays.is_empty() {
        return Err(ScreenInfoError::PlatformError(
            "CGGetActiveDisplayList returned zero displays".to_string(),
        ));
    }

    // Determine the primary display height for Y-axis flip.
    // CGMainDisplayID() returns the display with the menu bar.
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
            // Flip Y from Core Graphics' bottom-left origin to the protocol's top-left origin.
            // See module-level doc for the derivation.
            let y_offset = primary_height - (bounds.origin.y as i32) - (height as i32);

            MonitorInfo {
                monitor_id: i as u8,
                width,
                height,
                x_offset,
                y_offset,
                scale_factor: 100, // Retina (HiDPI) scaling is handled at a higher level
                is_primary: display_id == primary_id,
            }
        })
        .collect();

    // Ensure primary display is at index 0.
    // `!m.is_primary` maps primary → false (0) and non-primary → true (1),
    // so sort ascending puts primary first.
    monitors.sort_by_key(|m| !m.is_primary);

    // Reassign sequential monitor_ids after sorting so primary is always id=0.
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
