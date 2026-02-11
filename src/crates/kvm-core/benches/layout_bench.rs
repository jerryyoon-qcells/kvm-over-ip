//! Criterion benchmarks for [`VirtualLayout`] critical path operations.
//!
//! Measures latency for cursor resolution and edge transition checks to verify
//! compliance with the 0.5ms routing decision budget defined in the project
//! constitution §7.1.
//!
//! Run with:
//! ```bash
//! cargo bench --package kvm-core --bench layout_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use kvm_core::domain::layout::{
    Adjacency, ClientScreen, Edge, ScreenId, ScreenRegion, VirtualLayout,
};
use uuid::Uuid;

// ── Layout fixture builders ───────────────────────────────────────────────────

/// Creates a layout with `n` clients arranged horizontally to the right of the master.
///
/// Master: 1920×1080 at (0, 0)
/// Client i: 1920×1080 at (1920 * (i+1), 0)
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

        // Chain remaining clients
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

/// Benchmarks [`VirtualLayout::resolve_cursor`] for a cursor on the master screen.
fn bench_resolve_cursor_on_master(c: &mut Criterion) {
    let (layout, _) = build_layout_with_n_clients(4);
    let mut group = c.benchmark_group("resolve_cursor");

    group.bench_function("on_master_center", |b| {
        b.iter(|| layout.resolve_cursor(black_box(960), black_box(540)))
    });

    group.bench_function("on_master_top_left", |b| {
        b.iter(|| layout.resolve_cursor(black_box(0), black_box(0)))
    });

    group.finish();
}

/// Benchmarks [`VirtualLayout::resolve_cursor`] for a cursor on a client screen.
fn bench_resolve_cursor_on_client(c: &mut Criterion) {
    let (layout, ids) = build_layout_with_n_clients(4);
    let mut group = c.benchmark_group("resolve_cursor");

    // Client 0 starts at virtual_x=1920
    group.bench_function("on_client0_center", |b| {
        b.iter(|| layout.resolve_cursor(black_box(1920 + 960), black_box(540)))
    });

    // Client 3 is the furthest from master – worst case for sequential scan
    group.bench_function("on_client3_center", |b| {
        b.iter(|| layout.resolve_cursor(black_box(1920 * 4 + 960), black_box(540)))
    });

    group.finish();
}

/// Benchmarks [`VirtualLayout::resolve_cursor`] scaling with number of clients.
fn bench_resolve_cursor_scaling(c: &mut Criterion) {
    let client_counts = [1usize, 4, 8, 16];
    let mut group = c.benchmark_group("resolve_cursor_scaling");

    for &count in &client_counts {
        let (layout, _) = build_layout_with_n_clients(count);

        // Worst case: cursor is in the last client (furthest from master in linear scan)
        let last_client_center_x = 1920 * (count as i32) + 960;

        group.bench_with_input(
            BenchmarkId::new("clients", count),
            &last_client_center_x,
            |b, &x| b.iter(|| layout.resolve_cursor(black_box(x), black_box(540))),
        );
    }

    group.finish();
}

// ── Benchmarks: check_edge_transition ────────────────────────────────────────

/// Benchmarks [`VirtualLayout::check_edge_transition`] when the cursor is NOT near an edge
/// (expected `None` result – no-transition hot path).
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

/// Benchmarks [`VirtualLayout::check_edge_transition`] when the cursor IS near the right edge
/// (expected `Some(EdgeTransition)` – transition hot path).
fn bench_check_edge_with_transition(c: &mut Criterion) {
    let (layout, _) = build_layout_with_n_clients(4);
    let mut group = c.benchmark_group("check_edge_transition");

    // Cursor at x=1919 (1 pixel from right edge of 1920-wide master) triggers transition
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

/// Benchmarks edge transition check scaling with adjacency list size.
fn bench_check_edge_scaling(c: &mut Criterion) {
    let client_counts = [1usize, 4, 8, 16];
    let mut group = c.benchmark_group("check_edge_transition_scaling");

    for &count in &client_counts {
        let (layout, _) = build_layout_with_n_clients(count);

        // Check master right edge (first adjacency in list, best case for linear scan)
        group.bench_with_input(
            BenchmarkId::new("adjacencies", count),
            &count,
            |b, _| {
                b.iter(|| {
                    layout.check_edge_transition(
                        black_box(&ScreenId::Master),
                        black_box(1919),
                        black_box(540),
                    )
                })
            },
        );
    }

    group.finish();
}

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
