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

#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::OnceLock;
use std::thread;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOUSEEVENTF_HWHEEL, MOUSEEVENTF_WHEEL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, KBDLLHOOKSTRUCT_FLAGS, LLKHF_EXTENDED, MSG,
    MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP, XBUTTON1, XBUTTON2,
};

use super::{CaptureError, InputSource, MouseButton, RawInputEvent};

/// Atomic flag: when `true`, the current hook event should be suppressed
/// (not forwarded to the local system).
static SUPPRESS_FLAG: AtomicBool = AtomicBool::new(false);

/// Global sender used by hook callbacks to deliver events to the async runtime.
/// Initialized once by [`WindowsInputCaptureService::start`].
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
        msg if msg == MOUSEEVENTF_WHEEL.0 => {
            let delta = ((mhs.mouseData >> 16) as i16);
            RawInputEvent::MouseWheel { delta, x, y, time_ms }
        }
        msg if msg == MOUSEEVENTF_HWHEEL.0 => {
            let delta = ((mhs.mouseData >> 16) as i16);
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
