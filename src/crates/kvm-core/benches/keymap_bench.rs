//! Criterion benchmarks for key code translation tables.
//!
//! # Purpose
//!
//! Every input event that travels over the network must be translated from the
//! master's native key representation (e.g. Windows Virtual Key code) into the
//! shared HID Usage ID format, and then back to the client's native format
//! (e.g. X11 KeySym, macOS CGKeyCode).  This translation happens **on the hot
//! path** — once per key event, potentially thousands of times per second
//! during rapid typing.
//!
//! These benchmarks measure the latency of each translation direction to verify
//! compliance with the **100 µs-class budget** expected for a table lookup:
//!
//! | Direction               | Lookup type        | Expected order |
//! |-------------------------|--------------------|----------------|
//! | Windows VK → HID        | Direct array index | < 1 µs         |
//! | HID → Windows VK        | Linear scan        | < 5 µs         |
//! | HID → X11 KeySym        | Direct array index | < 1 µs         |
//! | HID → macOS CGKeyCode   | Direct array index | < 1 µs         |
//! | HID → DOM code          | Direct array index | < 1 µs         |
//!
//! # How translation works
//!
//! The master captures a raw platform key code (e.g. `VK_A = 0x41` on
//! Windows).  `KeyMapper::windows_vk_to_hid(0x41)` returns
//! `HidKeyCode::KeyA`.  The HID code is embedded in the `KeyEventMessage`
//! and sent over TCP to the client.  On the client, the appropriate
//! `hid_to_*` function converts it back to the platform's native code
//! before injecting the synthetic key event.
//!
//! # How to run
//!
//! ```bash
//! cargo bench --package kvm-core --bench keymap_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use kvm_core::keymap::hid::HidKeyCode;
use kvm_core::keymap::KeyMapper;

// ── Representative key codes for benchmarking ─────────────────────────────────
//
// We benchmark against a realistic spread of 19-20 key codes rather than just
// one.  This exercises both common keys (letters, digits) and special keys
// (arrows, function keys, modifiers) that have different array indices.

/// A slice of well-known HID key codes that cover the most common keys.
///
/// The set deliberately includes:
/// - Letters at different positions (`KeyA` at index 4, `KeyZ` at index 29)
/// - Function keys (`F1` at index 58, `F12` at index 69)
/// - Modifier keys (Ctrl, Shift, Alt, Meta)
/// - Arrow keys (which have separate HID Usage IDs from letters)
/// - `Unknown` as the boundary/error case for unmapped keys
const BENCH_HID_CODES: &[HidKeyCode] = &[
    HidKeyCode::KeyA,
    HidKeyCode::KeyZ,
    HidKeyCode::Enter,
    HidKeyCode::Escape,
    HidKeyCode::Backspace,
    HidKeyCode::Tab,
    HidKeyCode::Space,
    HidKeyCode::F1,
    HidKeyCode::F12,
    HidKeyCode::ControlLeft,
    HidKeyCode::ShiftLeft,
    HidKeyCode::AltLeft,
    HidKeyCode::MetaLeft,
    HidKeyCode::ArrowLeft,
    HidKeyCode::ArrowRight,
    HidKeyCode::ArrowUp,
    HidKeyCode::ArrowDown,
    HidKeyCode::Digit1,
    HidKeyCode::Digit0,
    HidKeyCode::Unknown,
];

/// A slice of Windows Virtual Key (VK) codes that map to common keys.
///
/// Windows VK codes are single-byte values (0x00–0xFF) defined in `<winuser.h>`.
/// The master uses these codes when capturing keyboard events via the low-level
/// `WH_KEYBOARD_LL` hook.
///
/// `0xFF` is used as the "no-mapping" entry to test the fallback path that
/// returns `HidKeyCode::Unknown`.
const BENCH_VK_CODES: &[u8] = &[
    0x41, // 'A'
    0x5A, // 'Z'
    0x0D, // VK_RETURN
    0x1B, // VK_ESCAPE
    0x08, // VK_BACK (Backspace)
    0x09, // VK_TAB
    0x20, // VK_SPACE
    0x70, // VK_F1
    0x7B, // VK_F12
    0x11, // VK_CONTROL
    0x10, // VK_SHIFT
    0x12, // VK_MENU (Alt)
    0x25, // VK_LEFT
    0x27, // VK_RIGHT
    0x26, // VK_UP
    0x28, // VK_DOWN
    0x31, // '1'
    0x30, // '0'
    0xFF, // No mapping (returns HidKeyCode::Unknown)
];

// ── Benchmarks: Windows VK translation ───────────────────────────────────────

/// Benchmarks `KeyMapper::windows_vk_to_hid` for single and batch lookups.
///
/// `windows_vk_to_hid` is a direct array index (`TABLE[vk as usize]`), which
/// is O(1) and cache-friendly.  Both sub-benchmarks should be in the single-
/// digit nanosecond range on modern hardware.
///
/// - `vk_to_hid_single` — one lookup, representative of a single keypress.
/// - `vk_to_hid_batch_19` — 19 lookups in sequence, representative of a
///   burst of key events during fast typing.
fn bench_windows_vk_to_hid(c: &mut Criterion) {
    let mut group = c.benchmark_group("keymap_windows_vk");

    // Single lookup (typical per-event cost)
    group.bench_function("vk_to_hid_single", |b| {
        // 0x41 = VK_A.  black_box prevents the compiler from inlining the
        // constant and short-circuiting the table lookup.
        b.iter(|| KeyMapper::windows_vk_to_hid(black_box(0x41)))
    });

    // Batch of 19 diverse VK codes (simulates a burst of key events)
    group.bench_function("vk_to_hid_batch_19", |b| {
        b.iter(|| {
            BENCH_VK_CODES
                .iter()
                .map(|&vk| KeyMapper::windows_vk_to_hid(black_box(vk)))
                .collect::<Vec<_>>()
            // Note: collect() allocates.  If allocation dominates, consider
            // using a stack-allocated array or fold into a sum for pure lookup
            // measurement.
        })
    });

    group.finish();
}

/// Benchmarks `KeyMapper::hid_to_windows_vk` for best-case and worst-case inputs.
///
/// `hid_to_windows_vk` performs a **linear scan** of the HID→VK mapping table,
/// so its cost grows with the position of the key in the table.
///
/// - `KeyA` appears early in the table (best case, fewest comparisons).
/// - `Unknown` appears last (worst case, most comparisons before the fallback).
///
/// If the worst case is significantly slower than the best case, it may be
/// worth switching to a hash map or sorted-array binary search.
fn bench_hid_to_windows_vk(c: &mut Criterion) {
    let mut group = c.benchmark_group("keymap_windows_vk");

    // HID→VK is a linear scan; benchmark worst-case (Unknown, last entry) and best-case (KeyA)
    group.bench_with_input(
        BenchmarkId::new("hid_to_vk", "KeyA"),
        &HidKeyCode::KeyA,
        |b, &hid| b.iter(|| KeyMapper::hid_to_windows_vk(black_box(hid))),
    );

    group.bench_with_input(
        BenchmarkId::new("hid_to_vk", "Unknown"),
        &HidKeyCode::Unknown,
        |b, &hid| b.iter(|| KeyMapper::hid_to_windows_vk(black_box(hid))),
    );

    group.finish();
}

// ── Benchmarks: X11 KeySym translation ───────────────────────────────────────

/// Benchmarks `KeyMapper::hid_to_x11_keysym` for single and batch lookups.
///
/// X11 KeySym codes are 32-bit integers defined in `<X11/keysymdef.h>`.
/// The XTest extension uses them to synthesise key events on Linux.
///
/// - `hid_to_keysym_single` — single lookup representative of one key event.
/// - `hid_to_keysym_batch_20` — 20-key batch representative of fast typing.
fn bench_hid_to_x11_keysym(c: &mut Criterion) {
    let mut group = c.benchmark_group("keymap_x11");

    group.bench_function("hid_to_keysym_single", |b| {
        b.iter(|| KeyMapper::hid_to_x11_keysym(black_box(HidKeyCode::KeyA)))
    });

    group.bench_function("hid_to_keysym_batch_20", |b| {
        b.iter(|| {
            BENCH_HID_CODES
                .iter()
                .map(|&hid| KeyMapper::hid_to_x11_keysym(black_box(hid)))
                .collect::<Vec<_>>()
        })
    });

    group.finish();
}

// ── Benchmarks: macOS CGKeyCode translation ───────────────────────────────────

/// Benchmarks `KeyMapper::hid_to_macos_cgkeycode` for single and batch lookups.
///
/// macOS CoreGraphics uses `CGKeyCode` (a `u16`) to identify keys.
/// These codes differ significantly from both USB HID codes and Windows VK
/// codes — for example, 'A' is `CGKeyCode::A = 0` on macOS but `0x41` on
/// Windows.  The translation table corrects for this.
fn bench_hid_to_macos_cgkeycode(c: &mut Criterion) {
    let mut group = c.benchmark_group("keymap_macos");

    group.bench_function("hid_to_cgkeycode_single", |b| {
        b.iter(|| KeyMapper::hid_to_macos_cgkeycode(black_box(HidKeyCode::KeyA)))
    });

    group.bench_function("hid_to_cgkeycode_batch_20", |b| {
        b.iter(|| {
            BENCH_HID_CODES
                .iter()
                .map(|&hid| KeyMapper::hid_to_macos_cgkeycode(black_box(hid)))
                .collect::<Vec<_>>()
        })
    });

    group.finish();
}

// ── Benchmarks: DOM code translation (web client) ────────────────────────────

/// Benchmarks `KeyMapper::hid_to_dom_code` for single and batch lookups.
///
/// The web bridge (kvm-web-bridge) forwards key events to browsers.
/// Browsers use the W3C `KeyboardEvent.code` property — a string like
/// `"KeyA"`, `"ArrowLeft"`, `"F12"` — which is what `hid_to_dom_code`
/// returns.  Since these are `&'static str` references into a table, the
/// allocation cost is zero; only the table lookup itself is measured.
fn bench_hid_to_dom_code(c: &mut Criterion) {
    let mut group = c.benchmark_group("keymap_dom");

    group.bench_function("hid_to_dom_code_single", |b| {
        b.iter(|| KeyMapper::hid_to_dom_code(black_box(HidKeyCode::KeyA)))
    });

    group.bench_function("hid_to_dom_code_batch_20", |b| {
        b.iter(|| {
            BENCH_HID_CODES
                .iter()
                .map(|&hid| KeyMapper::hid_to_dom_code(black_box(hid)))
                .collect::<Vec<_>>()
        })
    });

    group.finish();
}

// ── Criterion entry point ─────────────────────────────────────────────────────
//
// `criterion_group!` collects all five benchmark functions under the group
// name `benches`.  `criterion_main!` generates the `main` function.

criterion_group!(
    benches,
    bench_windows_vk_to_hid,
    bench_hid_to_windows_vk,
    bench_hid_to_x11_keysym,
    bench_hid_to_macos_cgkeycode,
    bench_hid_to_dom_code,
);
criterion_main!(benches);
