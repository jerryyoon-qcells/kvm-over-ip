//! UpdateLayoutUseCase: applies layout changes and persists configuration.
//!
//! The main entry point is [`build_layout`], which converts a list of
//! [`ClientLayoutConfig`] structs (typically loaded from TOML or sent from the
//! UI drag-and-drop editor) into a validated [`VirtualLayout`] with adjacencies
//! automatically detected from touching screen edges.
//!
//! # Automatic adjacency detection (for beginners)
//!
//! When the user drags a client screen directly to the right of the master in
//! the layout editor, the two screens share an edge (master's right == client's
//! left).  The [`detect_and_add_adjacencies`] function scans all screen pairs
//! and calls `layout.set_adjacency()` for each touching edge pair.
//!
//! This saves the user from having to manually configure adjacencies: just
//! position screens correctly and the connections are inferred automatically.

use kvm_core::domain::layout::{Adjacency, ClientId, ClientScreen, ScreenRegion, VirtualLayout};
use thiserror::Error;

/// Error type for layout update operations.
#[derive(Debug, Error, PartialEq)]
pub enum UpdateLayoutError {
    #[error("layout validation failed: {0}")]
    ValidationFailed(String),
    #[error("persistence failed: {0}")]
    PersistFailed(String),
}

/// Builds a new [`VirtualLayout`] from a configuration description.
///
/// Validates all client positions for overlaps before applying.
///
/// # Errors
///
/// Returns [`UpdateLayoutError::ValidationFailed`] if any screen regions overlap.
pub fn build_layout(
    master_width: u32,
    master_height: u32,
    clients: Vec<ClientLayoutConfig>,
) -> Result<VirtualLayout, UpdateLayoutError> {
    let mut layout = VirtualLayout::new(master_width, master_height);
    for client_cfg in clients {
        let screen = ClientScreen {
            client_id: client_cfg.client_id,
            region: ScreenRegion {
                virtual_x: client_cfg.x_offset,
                virtual_y: client_cfg.y_offset,
                width: client_cfg.width,
                height: client_cfg.height,
            },
            name: client_cfg.name,
        };
        layout
            .add_client(screen)
            .map_err(|e| UpdateLayoutError::ValidationFailed(e.to_string()))?;
    }

    // Auto-detect adjacencies from touching edges
    detect_and_add_adjacencies(&mut layout);

    Ok(layout)
}

/// Configuration for one client in a layout update.
#[derive(Debug, Clone)]
pub struct ClientLayoutConfig {
    pub client_id: ClientId,
    pub name: String,
    pub x_offset: i32,
    pub y_offset: i32,
    pub width: u32,
    pub height: u32,
}

/// Automatically detects touching edges between all screen pairs and adds adjacencies.
fn detect_and_add_adjacencies(layout: &mut VirtualLayout) {
    use kvm_core::domain::layout::{Edge, ScreenId};

    let master = layout.master.clone();
    let clients: Vec<_> = layout.clients().cloned().collect();

    // Check master right edge against each client left edge
    for client in &clients {
        if master.right() == client.region.virtual_x
            && ranges_overlap(
                master.virtual_y,
                master.virtual_y + master.height as i32,
                client.region.virtual_y,
                client.region.virtual_y + client.region.height as i32,
            )
        {
            let _ = layout.set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Right,
                to_screen: ScreenId::Client(client.client_id),
                to_edge: Edge::Left,
            });
            let _ = layout.set_adjacency(Adjacency {
                from_screen: ScreenId::Client(client.client_id),
                from_edge: Edge::Left,
                to_screen: ScreenId::Master,
                to_edge: Edge::Right,
            });
        }

        // Check master left edge
        if master.virtual_x == client.region.right()
            && ranges_overlap(
                master.virtual_y,
                master.virtual_y + master.height as i32,
                client.region.virtual_y,
                client.region.virtual_y + client.region.height as i32,
            )
        {
            let _ = layout.set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Left,
                to_screen: ScreenId::Client(client.client_id),
                to_edge: Edge::Right,
            });
        }

        // Check master bottom edge
        if master.bottom() == client.region.virtual_y
            && ranges_overlap(
                master.virtual_x,
                master.right(),
                client.region.virtual_x,
                client.region.right(),
            )
        {
            let _ = layout.set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Bottom,
                to_screen: ScreenId::Client(client.client_id),
                to_edge: Edge::Top,
            });
        }

        // Check master top edge
        if master.virtual_y == client.region.bottom()
            && ranges_overlap(
                master.virtual_x,
                master.right(),
                client.region.virtual_x,
                client.region.right(),
            )
        {
            let _ = layout.set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Top,
                to_screen: ScreenId::Client(client.client_id),
                to_edge: Edge::Bottom,
            });
        }
    }
}

/// Returns `true` if the two 1-D intervals `[a_start, a_end)` and `[b_start, b_end)` overlap.
///
/// This is used to check whether two screen edges share any vertical (or horizontal)
/// extent.  Two screens are considered adjacent only if their touching edges
/// actually overlap — a screen that merely shares a corner point is not adjacent.
///
/// # Example
///
/// ```
/// // Screens A (y: 0..1080) and B (y: 0..1080) share the full height: overlap.
/// // Screens A (y: 0..1080) and C (y: 1080..2160) are adjacent but don't overlap.
/// ```
fn ranges_overlap(a_start: i32, a_end: i32, b_start: i32, b_end: i32) -> bool {
    // Two intervals overlap when neither is entirely before the other.
    // (a_start < b_end) means A starts before B ends.
    // (b_start < a_end) means B starts before A ends.
    // Both conditions together mean the intervals intersect.
    a_start < b_end && b_start < a_end
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_client_cfg(x: i32, y: i32, w: u32, h: u32) -> ClientLayoutConfig {
        ClientLayoutConfig {
            client_id: Uuid::new_v4(),
            name: "test".to_string(),
            x_offset: x,
            y_offset: y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn test_build_layout_succeeds_with_non_overlapping_clients() {
        let client = make_client_cfg(1920, 0, 1920, 1080);
        let result = build_layout(1920, 1080, vec![client]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_layout_fails_with_overlapping_clients() {
        let c1 = make_client_cfg(1920, 0, 1920, 1080);
        let c2 = ClientLayoutConfig {
            client_id: Uuid::new_v4(),
            name: "overlap".to_string(),
            x_offset: 2000, // overlaps c1
            y_offset: 0,
            width: 1920,
            height: 1080,
        };
        let result = build_layout(1920, 1080, vec![c1, c2]);
        assert!(matches!(
            result,
            Err(UpdateLayoutError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_build_layout_empty_clients_produces_master_only() {
        let result = build_layout(1920, 1080, vec![]);
        assert!(result.is_ok());
        let layout = result.unwrap();
        assert_eq!(layout.clients().count(), 0);
    }

    #[test]
    fn test_detect_adjacency_adds_right_edge_when_client_is_directly_to_right() {
        let client_cfg = make_client_cfg(1920, 0, 1920, 1080);
        let cid = client_cfg.client_id;
        let layout = build_layout(1920, 1080, vec![client_cfg]).unwrap();

        // Verify that a transition at the master's right edge reaches the client
        let transition = layout.check_edge_transition(
            &kvm_core::domain::layout::ScreenId::Master,
            1919, // within 2px of right edge
            540,
        );
        assert!(transition.is_some());
        assert_eq!(
            transition.unwrap().to_screen,
            kvm_core::domain::layout::ScreenId::Client(cid)
        );
    }

    #[test]
    fn test_ranges_overlap_returns_true_for_overlapping_ranges() {
        assert!(ranges_overlap(0, 100, 50, 150));
    }

    #[test]
    fn test_ranges_overlap_returns_false_for_adjacent_ranges() {
        assert!(!ranges_overlap(0, 100, 100, 200));
    }

    #[test]
    fn test_ranges_overlap_returns_false_for_separated_ranges() {
        assert!(!ranges_overlap(0, 100, 200, 300));
    }
}
