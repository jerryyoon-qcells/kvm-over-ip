//! Criterion benchmarks for key code translation tables.
//!
//! Measures the latency of all translation directions (VK→HID, HID→VK,
//! HID→X11 KeySym, HID→macOS CGKeyCode, HID→DOM code) to verify compliance
//! with the 100µs-class budget expected for a table lookup on the hot path.
//!
//! Run with:
//! ```bash
//! cargo bench --package kvm-core --bench keymap_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use kvm_core::keymap::hid::HidKeyCode;
use kvm_core::keymap::KeyMapper;

// ── Representative key codes for benchmarking ─────────────────────────────────

/// A slice of well-known HID key codes that cover the most common keys.
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

/// A slice of Windows VK codes that map to common keys.
const BENCH_VK_CODES: &[u8] = &[
    0x41, // 'A'
    0x5A, // 'Z'
    0x0D, // VK_RETURN
    0x1B, // VK_ESCAPE
    0x08, // VK_BACK
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
    0xFF, // No mapping (unmapped VK)
];

// ── Benchmarks: Windows VK translation ───────────────────────────────────────

fn bench_windows_vk_to_hid(c: &mut Criterion) {
    let mut group = c.benchmark_group("keymap_windows_vk");

    // Single lookup (typical per-event cost)
    group.bench_function("vk_to_hid_single", |b| {
        b.iter(|| KeyMapper::windows_vk_to_hid(black_box(0x41)))
    });

    // Batch of 19 diverse VK codes (simulates a burst of key events)
    group.bench_function("vk_to_hid_batch_19", |b| {
        b.iter(|| {
            BENCH_VK_CODES
                .iter()
                .map(|&vk| KeyMapper::windows_vk_to_hid(black_box(vk)))
                .collect::<Vec<_>>()
        })
    });

    group.finish();
}

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

criterion_group!(
    benches,
    bench_windows_vk_to_hid,
    bench_hid_to_windows_vk,
    bench_hid_to_x11_keysym,
    bench_hid_to_macos_cgkeycode,
    bench_hid_to_dom_code,
);
criterion_main!(benches);
