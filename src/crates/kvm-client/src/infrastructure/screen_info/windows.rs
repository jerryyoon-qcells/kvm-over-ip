//! Windows screen enumeration via `EnumDisplayMonitors` / `GetMonitorInfoW`.
//!
//! Queries the Win32 display API to enumerate all connected monitors and
//! builds a list of [`MonitorInfo`] records suitable for the protocol.

use super::{PlatformScreenEnumerator, ScreenInfoError};
use kvm_core::protocol::messages::MonitorInfo;

#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::{BOOL, LPARAM, RECT},
    Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HMONITOR, HDC, MONITORINFOEXW,
    },
};

/// Windows implementation of [`PlatformScreenEnumerator`] using Win32 APIs.
pub struct WindowsScreenEnumerator;

impl WindowsScreenEnumerator {
    /// Creates a new `WindowsScreenEnumerator`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsScreenEnumerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformScreenEnumerator for WindowsScreenEnumerator {
    fn enumerate_monitors(&self) -> Result<Vec<MonitorInfo>, ScreenInfoError> {
        let mut monitors: Vec<MonitorInfo> = Vec::new();

        // SAFETY: `lpfn` is a valid function pointer with the correct signature.
        // `lParam` is a raw pointer to `monitors` which outlives this call.
        // The callback is synchronous and called only within `EnumDisplayMonitors`.
        // `HDC::default()` (null) means enumerate all monitors on the virtual desktop.
        unsafe {
            EnumDisplayMonitors(
                HDC::default(),
                None,
                Some(monitor_enum_proc),
                LPARAM(&mut monitors as *mut Vec<MonitorInfo> as isize),
            );
        }

        if monitors.is_empty() {
            return Err(ScreenInfoError::PlatformError(
                "EnumDisplayMonitors returned no monitors".to_string(),
            ));
        }

        // Sort: primary monitor (x_offset == 0 && y_offset == 0) comes first.
        monitors.sort_by_key(|m| (m.x_offset != 0 || m.y_offset != 0) as u8);

        // Assign sequential monitor_ids after sorting.
        for (i, m) in monitors.iter_mut().enumerate() {
            m.monitor_id = i as u8;
        }

        Ok(monitors)
    }
}

/// Win32 monitor enumeration callback.
///
/// # Safety
///
/// Called by Win32 inside `EnumDisplayMonitors`. `lparam` must be a valid
/// pointer to `Vec<MonitorInfo>` for the duration of the enumeration call.
#[cfg(target_os = "windows")]
unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprc_clip: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors = &mut *(lparam.0 as *mut Vec<MonitorInfo>);

    // SAFETY: MONITORINFOEXW is a Plain Old Data struct; zero initialization is valid.
    // We are already in an `unsafe` context (this is an `unsafe extern "system"` fn).
    let mut info: MONITORINFOEXW = std::mem::zeroed();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

    // SAFETY: `hmonitor` is a valid handle provided by Win32.
    if GetMonitorInfoW(hmonitor, &mut info.monitorInfo).as_bool() {
        let rc = &info.monitorInfo.rcMonitor;
        let width = (rc.right - rc.left) as u32;
        let height = (rc.bottom - rc.top) as u32;
        let x_offset = rc.left as i32;
        let y_offset = rc.top as i32;
        let is_primary = (info.monitorInfo.dwFlags & 1) != 0; // MONITORINFOF_PRIMARY = 1

        monitors.push(MonitorInfo {
            monitor_id: 0, // assigned after sort
            width,
            height,
            x_offset,
            y_offset,
            scale_factor: 100, // DPI scaling handled separately if needed
            is_primary,
        });
    }

    BOOL(1) // continue enumeration
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke-tests that the enumerator can be constructed and called without
    /// panicking.  The actual count depends on the test machine's display
    /// configuration so we only assert a minimum of one monitor.
    #[test]
    fn test_windows_screen_enumerator_returns_at_least_one_monitor() {
        let enumerator = WindowsScreenEnumerator::new();
        let result = enumerator.enumerate_monitors();
        assert!(
            result.is_ok(),
            "enumerate_monitors must succeed: {:?}",
            result.err()
        );
        let monitors = result.unwrap();
        assert!(!monitors.is_empty(), "must find at least one monitor");
    }

    #[test]
    fn test_windows_screen_enumerator_primary_is_first() {
        let enumerator = WindowsScreenEnumerator::new();
        let monitors = enumerator.enumerate_monitors().expect("enumerate");
        if !monitors.is_empty() {
            assert!(
                monitors[0].is_primary,
                "first monitor must be primary after sort"
            );
        }
    }

    #[test]
    fn test_windows_screen_enumerator_monitor_ids_are_sequential() {
        let enumerator = WindowsScreenEnumerator::new();
        let monitors = enumerator.enumerate_monitors().expect("enumerate");
        for (i, m) in monitors.iter().enumerate() {
            assert_eq!(m.monitor_id as usize, i, "monitor_id must be sequential");
        }
    }
}
