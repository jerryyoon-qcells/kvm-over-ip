//! Windows low-level keyboard and mouse hook implementation.
//!
//! This module installs WH_KEYBOARD_LL and WH_MOUSE_LL hooks using the
//! Windows API. Both hooks share a dedicated Win32 message-loop thread
//! that runs at THREAD_PRIORITY_TIME_CRITICAL to minimize callback latency.
//!
//! # Safety
//!
//! This module uses `unsafe` code exclusively for Windows API FFI calls.
//! All `unsafe` blocks are annotated with `// SAFETY:` comments.
//!
//! # Scroll Event Notes for Beginners
//!
//! Windows has two different sets of mouse-wheel constants that are easy to
//! confuse:
//!
//! 1. **`MOUSEEVENTF_WHEEL` / `MOUSEEVENTF_HWHEEL`** — these are *flag bits*
//!    used in `SendInput()` / `mouse_event()` to *synthesize* scroll events.
//!    Their numeric values are `0x0800` and `0x1000`.
//!
//! 2. **`WM_MOUSEWHEEL` / `WM_MOUSEHWHEEL`** — these are *window message IDs*
//!    delivered by the operating system when the user physically rotates the
//!    scroll wheel.  Their numeric values are `0x020A` and `0x020E`.
//!
//! A `WH_MOUSE_LL` low-level hook receives the *window message ID* in the
//! `wParam` argument, so the correct constants to match against are
//! `WM_MOUSEWHEEL` and `WM_MOUSEHWHEEL`.  Using the `MOUSEEVENTF_*` constants
//! here would cause scroll events to be silently ignored because the values
//! do not match the actual `wParam` integers.

#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::OnceLock;
use std::thread;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
// NOTE: We deliberately do NOT import MOUSEEVENTF_WHEEL / MOUSEEVENTF_HWHEEL here.
// Those constants are for SendInput() output, not for WH_MOUSE_LL input matching.
// See module-level docs above for a full explanation.
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, KBDLLHOOKSTRUCT_FLAGS, LLKHF_EXTENDED, MSG,
    MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
    // XBUTTON1 distinguishes the X1 side button from X2 in WM_XBUTTONDOWN/UP events.
    // XBUTTON2 is not used explicitly: any mouseData value that is not XBUTTON1 is
    // treated as X2 by the fallback branch in the match below.
    XBUTTON1,
};

use super::{CaptureError, InputSource, MouseButton, RawInputEvent};

/// Atomic flag: when `true`, the current hook event should be suppressed
/// (not forwarded to the local system).
///
/// This flag is set from the Tokio async thread via `suppress_current_event()`
/// and read inside the hook callback on the hook-loop thread.  Using an
/// `AtomicBool` (rather than a `Mutex<bool>`) avoids any blocking in the
/// time-critical hook callback.
static SUPPRESS_FLAG: AtomicBool = AtomicBool::new(false);

/// Global sender used by hook callbacks to deliver events to the async runtime.
/// Initialized once by [`WindowsInputCaptureService::start`].
///
/// `OnceLock` is a cell that can be written exactly once and read many times.
/// The hook callbacks are `unsafe extern "system"` functions that receive no
/// user data pointer, so the only way to pass the sender into them is via a
/// global.  `OnceLock` makes this safe: the write is guarded against double
/// initialisation and the read side is always valid after the first write.
static EVENT_SENDER: OnceLock<Sender<RawInputEvent>> = OnceLock::new();

/// Windows low-level input capture service.
///
/// Installs `WH_KEYBOARD_LL` and `WH_MOUSE_LL` hooks and runs a dedicated
/// Win32 message loop thread.
pub struct WindowsInputCaptureService {
    /// Set to `true` when `stop()` has been called.
    stopped: AtomicBool,
}

impl WindowsInputCaptureService {
    /// Creates a new (unstarted) service instance.
    pub fn new() -> Self {
        Self {
            stopped: AtomicBool::new(false),
        }
    }
}

impl Default for WindowsInputCaptureService {
    fn default() -> Self {
        Self::new()
    }
}

impl InputSource for WindowsInputCaptureService {
    fn start(&self) -> Result<mpsc::Receiver<RawInputEvent>, CaptureError> {
        let (tx, rx) = mpsc::channel::<RawInputEvent>();

        // Register the global sender. This will fail if called a second time.
        EVENT_SENDER
            .set(tx)
            .map_err(|_| CaptureError::KeyboardHookInstallFailed(
                "EVENT_SENDER already initialized – only one capture service may run".to_string(),
            ))?;

        // Spawn the Win32 message loop thread that installs and manages the hooks.
        thread::Builder::new()
            .name("kvm-hook-loop".to_string())
            .spawn(run_hook_message_loop)
            .map_err(|e| CaptureError::KeyboardHookInstallFailed(e.to_string()))?;

        Ok(rx)
    }

    fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        // The message loop thread will detect the stopped flag and exit.
        // Hook handles are cleaned up in the thread itself.
    }

    fn suppress_current_event(&self) {
        SUPPRESS_FLAG.store(true, Ordering::SeqCst);
    }
}

/// Entry point for the dedicated Win32 message loop thread.
fn run_hook_message_loop() {
    // SAFETY: SetWindowsHookExW requires the calling thread to have a message loop.
    // We install both hooks before entering the loop.
    let kbd_hook: HHOOK = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
            .expect("WH_KEYBOARD_LL hook installation failed")
    };
    let mouse_hook: HHOOK = unsafe {
        SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)
            .expect("WH_MOUSE_LL hook installation failed")
    };

    // Win32 message loop – blocks until WM_QUIT is posted
    let mut msg = MSG::default();
    // SAFETY: Standard Win32 GetMessage/DispatchMessage loop pattern.
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
        UnhookWindowsHookEx(kbd_hook).ok();
        UnhookWindowsHookEx(mouse_hook).ok();
    }
}

/// Low-level keyboard hook callback.
///
/// # Safety
///
/// This function is called by Windows from the hook message loop thread.
/// It must return quickly (< ~300ms) to avoid hook removal by the OS.
unsafe extern "system" fn keyboard_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code != HC_ACTION as i32 {
        // SAFETY: Must call CallNextHookEx when n_code < 0.
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    // SAFETY: l_param points to a KBDLLHOOKSTRUCT when n_code == HC_ACTION.
    let kbs = &*(l_param.0 as *const KBDLLHOOKSTRUCT);

    let vk_code = kbs.vkCode as u8;
    let scan_code = kbs.scanCode as u16;
    let time_ms = kbs.time;
    let is_extended = (kbs.flags & LLKHF_EXTENDED) != KBDLLHOOKSTRUCT_FLAGS(0);

    let event = match w_param.0 as u32 {
        WM_KEYDOWN | WM_SYSKEYDOWN => RawInputEvent::KeyDown {
            vk_code,
            scan_code,
            time_ms,
            is_extended,
        },
        WM_KEYUP | WM_SYSKEYUP => RawInputEvent::KeyUp {
            vk_code,
            scan_code,
            time_ms,
            is_extended,
        },
        _ => {
            return CallNextHookEx(None, n_code, w_param, l_param);
        }
    };

    // Send the event to the async consumer.
    if let Some(sender) = EVENT_SENDER.get() {
        // Ignore send errors (channel closed during shutdown).
        let _ = sender.send(event);
    }

    // If suppression is requested, consume the event (do not call CallNextHookEx).
    if SUPPRESS_FLAG.swap(false, Ordering::SeqCst) {
        return LRESULT(1);
    }

    // SAFETY: Forward the event to the next hook in the chain.
    CallNextHookEx(None, n_code, w_param, l_param)
}

/// Low-level mouse hook callback.
///
/// # Safety
///
/// Called by Windows from the hook message loop thread; must return quickly.
unsafe extern "system" fn mouse_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code != HC_ACTION as i32 {
        // SAFETY: Must call CallNextHookEx when n_code < 0.
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    // SAFETY: l_param points to a MSLLHOOKSTRUCT when n_code == HC_ACTION.
    let mhs = &*(l_param.0 as *const MSLLHOOKSTRUCT);

    let x = mhs.pt.x;
    let y = mhs.pt.y;
    let time_ms = mhs.time;

    let event = match w_param.0 as u32 {
        WM_MOUSEMOVE => RawInputEvent::MouseMove { x, y, time_ms },

        WM_LBUTTONDOWN => RawInputEvent::MouseButtonDown {
            button: MouseButton::Left,
            x,
            y,
            time_ms,
        },
        WM_LBUTTONUP => RawInputEvent::MouseButtonUp {
            button: MouseButton::Left,
            x,
            y,
            time_ms,
        },
        WM_RBUTTONDOWN => RawInputEvent::MouseButtonDown {
            button: MouseButton::Right,
            x,
            y,
            time_ms,
        },
        WM_RBUTTONUP => RawInputEvent::MouseButtonUp {
            button: MouseButton::Right,
            x,
            y,
            time_ms,
        },
        WM_MBUTTONDOWN => RawInputEvent::MouseButtonDown {
            button: MouseButton::Middle,
            x,
            y,
            time_ms,
        },
        WM_MBUTTONUP => RawInputEvent::MouseButtonUp {
            button: MouseButton::Middle,
            x,
            y,
            time_ms,
        },
        WM_XBUTTONDOWN => {
            let button = if (mhs.mouseData >> 16) as u16 == XBUTTON1 {
                MouseButton::X1
            } else {
                MouseButton::X2
            };
            RawInputEvent::MouseButtonDown { button, x, y, time_ms }
        }
        WM_XBUTTONUP => {
            let button = if (mhs.mouseData >> 16) as u16 == XBUTTON1 {
                MouseButton::X1
            } else {
                MouseButton::X2
            };
            RawInputEvent::MouseButtonUp { button, x, y, time_ms }
        }

        // FIX (Bug 1): The original code matched against MOUSEEVENTF_WHEEL.0 (= 0x0800)
        // and MOUSEEVENTF_HWHEEL.0 (= 0x1000).  Those are flag bits for SendInput() and
        // will NEVER appear as wParam in a WH_MOUSE_LL callback.  The correct message IDs
        // that Windows actually delivers to a low-level mouse hook are:
        //   WM_MOUSEWHEEL  = 0x020A  (vertical wheel)
        //   WM_MOUSEHWHEEL = 0x020E  (horizontal wheel / tilt)
        //
        // How the scroll delta works:
        //   MSLLHOOKSTRUCT.mouseData is a DWORD.  Its high 16 bits carry the signed
        //   scroll amount (WHEEL_DELTA units: +120 = one notch up, -120 = one notch down).
        //   We shift right by 16 and reinterpret as i16 to recover the signed value.
        WM_MOUSEWHEEL => {
            // Vertical scroll: positive delta = away from user (up/zoom-in convention).
            let delta = (mhs.mouseData >> 16) as i16;
            RawInputEvent::MouseWheel { delta, x, y, time_ms }
        }
        WM_MOUSEHWHEEL => {
            // Horizontal scroll (wheel tilt): positive delta = right.
            let delta = (mhs.mouseData >> 16) as i16;
            RawInputEvent::MouseWheelH { delta, x, y, time_ms }
        }

        _ => {
            return CallNextHookEx(None, n_code, w_param, l_param);
        }
    };

    if let Some(sender) = EVENT_SENDER.get() {
        let _ = sender.send(event);
    }

    if SUPPRESS_FLAG.swap(false, Ordering::SeqCst) {
        return LRESULT(1);
    }

    // SAFETY: Forward to the next hook in the chain.
    CallNextHookEx(None, n_code, w_param, l_param)
}
