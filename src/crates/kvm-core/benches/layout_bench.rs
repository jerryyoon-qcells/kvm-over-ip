//! Criterion benchmarks for [`VirtualLayout`] critical path operations.
//!
//! # Purpose
//!
//! When the user moves the mouse across a screen edge, the master must decide
//! in real time whether to route further input to a different machine.  This
//! routing decision has two steps:
//!
//! 1. **`resolve_cursor(x, y)`** — determine *which screen* the cursor is
//!    currently on (master or one of the clients).
//! 2. **`check_edge_transition(screen, x, y)`** — determine whether the cursor
//!    is within 2 pixels of an edge that connects to another screen.
//!
//! Both operations run on every mouse-move event, so their latency directly
//! impacts the user-perceived KVM response time.  The project constitution
//! §7.1 sets a budget of **0.5 ms** for the complete routing decision.  These
//! benchmarks verify that the two layout operations together stay well within
//! that budget.
//!
//! # Layout topology used in benchmarks
//!
//! All benchmarks build a "horizontal strip" layout where clients are placed
//! to the right of the master in a line:
//!
//! ```text
//! [Master 1920×1080] → [Client 0] → [Client 1] → … → [Client N-1]
//!  (0, 0)               (1920, 0)    (3840, 0)           (N*1920, 0)
//! ```
//!
//! This topology stresses the *worst-case scan depth* because `resolve_cursor`
//! and `check_edge_transition` may iterate through all client records in order.
//! Benchmarking with 1, 4, 8, and 16 clients reveals how the operations scale.
//!
//! # How to run
//!
//! ```bash
//! cargo bench --package kvm-core --bench layout_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use kvm_core::domain::layout::{
    Adjacency, ClientScreen, Edge, ScreenId, ScreenRegion, VirtualLayout,
};
use uuid::Uuid;

// ── Layout fixture builders ───────────────────────────────────────────────────

/// Creates a `VirtualLayout` with `n` clients arranged horizontally to the
/// right of the master, and returns it together with the client UUIDs.
///
/// # Layout geometry
///
/// - Master: 1920×1080 at virtual (0, 0).
/// - Client `i`: 1920×1080 at virtual (`1920 * (i + 1)`, 0).
///
/// # Adjacency wiring
///
/// After placing all clients, the function wires up the adjacency list:
/// - Master's right edge connects to Client 0's left edge.
/// - Client `i`'s right edge connects to Client `i+1`'s left edge (chained).
///
/// Without adjacency entries, `check_edge_transition` would always return
/// `None` because it only fires when an edge has a configured neighbour.
fn build_layout_with_n_clients(n: usize) -> (VirtualLayout, Vec<Uuid>) {
    let mut layout = VirtualLayout::new(1920, 1080);
    let mut ids = Vec::with_capacity(n);

    for i in 0..n {
        let id = Uuid::new_v4();
        ids.push(id);
        layout
            .add_client(ClientScreen {
                client_id: id,
                region: ScreenRegion {
                    // Each client is 1920 pixels to the right of the previous one.
                    virtual_x: 1920 * (i as i32 + 1),
                    virtual_y: 0,
                    width: 1920,
                    height: 1080,
                },
                name: format!("client-{}", i),
            })
            .expect("non-overlapping clients must be added without error");
    }

    // Connect master right edge to client[0] left edge
    if !ids.is_empty() {
        layout
            .set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Right,
                to_screen: ScreenId::Client(ids[0]),
                to_edge: Edge::Left,
            })
            .expect("adjacency must be valid");

        // Chain remaining clients: client[i].Right → client[i+1].Left
        for i in 0..(ids.len() - 1) {
            layout
                .set_adjacency(Adjacency {
                    from_screen: ScreenId::Client(ids[i]),
                    from_edge: Edge::Right,
                    to_screen: ScreenId::Client(ids[i + 1]),
                    to_edge: Edge::Left,
                })
                .expect("adjacency must be valid");
        }
    }

    (layout, ids)
}

// ── Benchmarks: resolve_cursor ────────────────────────────────────────────────
//
// `resolve_cursor(x, y)` returns the `ScreenId` (Master or Client(uuid)) for
// the screen that contains the given virtual coordinate.
//
// The current implementation does a linear scan through all screens and checks
// whether the point falls inside each screen's bounding rectangle.  This means
// the cost grows linearly with the number of clients.

/// Benchmarks `resolve_cursor` for a cursor in the centre of the master screen.
///
/// With the cursor on the master, the very first bounding-box check succeeds
/// (best case: O(1) in the current implementation).  This sub-benchmark
/// establishes the floor latency.
fn bench_resolve_cursor_on_master(c: &mut Criterion) {
    let (layout, _) = build_layout_with_n_clients(4);
    let mut group = c.benchmark_group("resolve_cursor");

    // Centre of master screen: (960, 540) is unambiguously on the master.
    group.bench_function("on_master_center", |b| {
        b.iter(|| layout.resolve_cursor(black_box(960), black_box(540)))
    });

    // Top-left corner of master: still on master, tests boundary condition.
    group.bench_function("on_master_top_left", |b| {
        b.iter(|| layout.resolve_cursor(black_box(0), black_box(0)))
    });

    group.finish();
}

/// Benchmarks `resolve_cursor` for a cursor on a client screen.
///
/// - `on_client0_center` — Client 0 is the first client checked after master
///   (index 1 in the scan), so the overhead is minimal.
/// - `on_client3_center` — Client 3 is the last in a 4-client layout
///   (index 4 in the scan), exercising the worst-case scan depth for this
///   fixture.
fn bench_resolve_cursor_on_client(c: &mut Criterion) {
    let (layout, _ids) = build_layout_with_n_clients(4);
    let mut group = c.benchmark_group("resolve_cursor");

    // Client 0 starts at virtual_x = 1920.  Centre = (1920 + 960, 540).
    group.bench_function("on_client0_center", |b| {
        b.iter(|| layout.resolve_cursor(black_box(1920 + 960), black_box(540)))
    });

    // Client 3 starts at virtual_x = 4 * 1920 = 7680.  Centre = (7680 + 960, 540).
    // This is the furthest screen from master — worst case for sequential scan.
    group.bench_function("on_client3_center", |b| {
        b.iter(|| layout.resolve_cursor(black_box(1920 * 4 + 960), black_box(540)))
    });

    group.finish();
}

/// Benchmarks `resolve_cursor` scaling with the number of clients.
///
/// Runs with 1, 4, 8, and 16 clients, always checking the cursor in the
/// **last** client (worst case for a linear scan).  The resulting data points
/// show how the operation scales and whether a constant-time structure (like a
/// spatial index) would be worthwhile.
fn bench_resolve_cursor_scaling(c: &mut Criterion) {
    // Test with 1, 4, 8, and 16 clients.
    let client_counts = [1usize, 4, 8, 16];
    let mut group = c.benchmark_group("resolve_cursor_scaling");

    for &count in &client_counts {
        let (layout, _) = build_layout_with_n_clients(count);

        // Worst case: cursor is in the last client (furthest from master in linear scan).
        // Last client starts at virtual_x = 1920 * count; centre is 960 pixels in.
        let last_client_center_x = 1920 * (count as i32) + 960;

        group.bench_with_input(
            // The report will show entries like "resolve_cursor_scaling/clients/4".
            BenchmarkId::new("clients", count),
            &last_client_center_x,
            |b, &x| b.iter(|| layout.resolve_cursor(black_box(x), black_box(540))),
        );
    }

    group.finish();
}

// ── Benchmarks: check_edge_transition ────────────────────────────────────────
//
// `check_edge_transition(screen_id, x, y)` returns `Some(EdgeTransition)` if
// the cursor is within `EDGE_THRESHOLD` (2 pixels) of an edge that has an
// adjacency, or `None` otherwise.
//
// On the hot path, the cursor is almost never near an edge, so the `None`
// ("no transition") case is measured separately from the `Some` ("transition")
// case.

/// Benchmarks `check_edge_transition` when the cursor is NOT near any edge.
///
/// This is the **hot path** — most mouse-move events land in the interior of a
/// screen and should return `None` as quickly as possible.
///
/// Cursor at (960, 540) is near no edge of the 1920×1080 master, so the
/// function simply checks all four edge thresholds and returns `None`.
fn bench_check_edge_no_transition(c: &mut Criterion) {
    let (layout, _) = build_layout_with_n_clients(4);
    let mut group = c.benchmark_group("check_edge_transition");

    group.bench_function("no_transition_master_center", |b| {
        b.iter(|| {
            layout.check_edge_transition(
                black_box(&ScreenId::Master),
                black_box(960),
                black_box(540),
            )
        })
    });

    group.finish();
}

/// Benchmarks `check_edge_transition` when the cursor IS near a wired edge.
///
/// Cursor at x=1919 is 1 pixel from the right edge of the 1920-pixel-wide
/// master.  Since master's right edge is wired to Client 0's left edge, the
/// function returns `Some(EdgeTransition { … })`.
///
/// This benchmark measures the **transition-triggered path**, which includes
/// looking up the adjacency entry and computing the new virtual coordinates.
fn bench_check_edge_with_transition(c: &mut Criterion) {
    let (layout, _) = build_layout_with_n_clients(4);
    let mut group = c.benchmark_group("check_edge_transition");

    // Cursor at x=1919 (1 pixel from right edge of 1920-wide master) triggers transition.
    group.bench_function("transition_master_right_edge", |b| {
        b.iter(|| {
            layout.check_edge_transition(
                black_box(&ScreenId::Master),
                black_box(1919),
                black_box(540),
            )
        })
    });

    group.finish();
}

/// Benchmarks edge transition check scaling with the number of adjacency entries.
///
/// `check_edge_transition` scans the adjacency list for entries that match the
/// current screen.  Adding more clients adds more adjacency entries, which
/// could make the scan slower.
///
/// This benchmark checks master's right edge across layouts with 1, 4, 8, and
/// 16 clients.  Because master has only one adjacency (master.Right → client[0].Left)
/// regardless of the total client count, the scaling should be flat — if it is
/// not, the implementation is scanning too broadly.
fn bench_check_edge_scaling(c: &mut Criterion) {
    let client_counts = [1usize, 4, 8, 16];
    let mut group = c.benchmark_group("check_edge_transition_scaling");

    for &count in &client_counts {
        let (layout, _) = build_layout_with_n_clients(count);

        // Check master right edge (first adjacency in list, best case for linear scan).
        group.bench_with_input(BenchmarkId::new("adjacencies", count), &count, |b, _| {
            b.iter(|| {
                layout.check_edge_transition(
                    black_box(&ScreenId::Master),
                    black_box(1919),
                    black_box(540),
                )
            })
        });
    }

    group.finish();
}

// ── Criterion entry point ─────────────────────────────────────────────────────

criterion_group!(
    benches,
    bench_resolve_cursor_on_master,
    bench_resolve_cursor_on_client,
    bench_resolve_cursor_scaling,
    bench_check_edge_no_transition,
    bench_check_edge_with_transition,
    bench_check_edge_scaling,
);
criterion_main!(benches);
