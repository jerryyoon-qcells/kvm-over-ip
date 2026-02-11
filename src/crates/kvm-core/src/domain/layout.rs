//! Virtual screen layout domain entity.
//!
//! The layout engine maintains a unified 2D coordinate space ("virtual screen space")
//! where all screens are positioned. The master screen is anchored at (0, 0).
//! Clients are positioned relative to the master using their virtual_x / virtual_y offsets.

use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Pixel threshold within which a cursor is considered to be "at the edge".
const EDGE_THRESHOLD: i32 = 2;

/// Unique identifier for a client, derived from UUID v4.
pub type ClientId = Uuid;

/// Identifies a screen (either master or a specific client).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScreenId {
    Master,
    Client(ClientId),
}

/// The four edges of a rectangular screen region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Errors that can occur when configuring the layout.
#[derive(Debug, Error, PartialEq)]
pub enum LayoutError {
    /// Two screen regions overlap in virtual space.
    #[error("screen regions overlap: new region conflicts with existing layout")]
    Overlap,

    /// The specified client does not exist in the layout.
    #[error("client not found: {0}")]
    ClientNotFound(ClientId),

    /// An adjacency references a screen that does not exist.
    #[error("invalid adjacency: referenced screen does not exist")]
    InvalidAdjacency,

    /// The two edges in an adjacency are not compatible (e.g., both Left edges).
    #[error("invalid adjacency: edges must be on opposite sides (Left↔Right or Top↔Bottom)")]
    IncompatibleEdges,
}

/// A rectangular region in the virtual screen coordinate system.
///
/// `virtual_x` and `virtual_y` are the top-left corner coordinates in virtual space.
/// The master screen is always at (0, 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenRegion {
    /// X coordinate of the top-left corner in virtual space (may be negative).
    pub virtual_x: i32,
    /// Y coordinate of the top-left corner in virtual space (may be negative).
    pub virtual_y: i32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl ScreenRegion {
    /// Returns the rightmost X coordinate (exclusive).
    pub fn right(&self) -> i32 {
        self.virtual_x + self.width as i32
    }

    /// Returns the bottommost Y coordinate (exclusive).
    pub fn bottom(&self) -> i32 {
        self.virtual_y + self.height as i32
    }

    /// Returns `true` if this region overlaps with `other`.
    pub fn overlaps(&self, other: &ScreenRegion) -> bool {
        self.virtual_x < other.right()
            && self.right() > other.virtual_x
            && self.virtual_y < other.bottom()
            && self.bottom() > other.virtual_y
    }
}

/// A client screen positioned in virtual space.
#[derive(Debug, Clone)]
pub struct ClientScreen {
    /// Unique identifier of the client.
    pub client_id: ClientId,
    /// Position and size in virtual space.
    pub region: ScreenRegion,
    /// Human-readable name for the client (e.g., hostname).
    pub name: String,
}

/// An adjacency relationship between two screen edges.
///
/// Defines that when the cursor crosses `from_edge` of `from_screen`, it should
/// appear on `to_edge` of `to_screen` at the proportionally equivalent position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adjacency {
    /// The screen the cursor is leaving.
    pub from_screen: ScreenId,
    /// The edge of `from_screen` that is being crossed.
    pub from_edge: Edge,
    /// The screen the cursor is entering.
    pub to_screen: ScreenId,
    /// The edge of `to_screen` where the cursor enters.
    pub to_edge: Edge,
}

impl Adjacency {
    /// Returns `true` if the edge combination is valid (opposite sides).
    fn is_valid(&self) -> bool {
        matches!(
            (&self.from_edge, &self.to_edge),
            (Edge::Right, Edge::Left)
                | (Edge::Left, Edge::Right)
                | (Edge::Bottom, Edge::Top)
                | (Edge::Top, Edge::Bottom)
        )
    }
}

/// Where the cursor currently resides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorLocation {
    /// The cursor is on the master screen.
    OnMaster {
        /// X position in master screen local coordinates.
        local_x: i32,
        /// Y position in master screen local coordinates.
        local_y: i32,
    },
    /// The cursor is on a client screen.
    OnClient {
        client_id: ClientId,
        /// X position in the client screen's local coordinate space.
        local_x: i32,
        /// Y position in the client screen's local coordinate space.
        local_y: i32,
    },
}

/// Result of a cursor edge transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeTransition {
    /// The screen the cursor is transitioning to.
    pub to_screen: ScreenId,
    /// Entry X position in the destination screen's local coordinate space.
    pub entry_x: i32,
    /// Entry Y position in the destination screen's local coordinate space.
    pub entry_y: i32,
    /// Where to teleport the master physical cursor after the transition, so
    /// further movement continues to flow naturally toward the client.
    pub master_teleport_x: i32,
    /// Y coordinate for the master cursor teleport.
    pub master_teleport_y: i32,
}

/// The virtual screen layout.
///
/// Maintains the master screen, all client screens, and their adjacency relationships.
/// All cursor position calculations are performed in virtual screen space.
///
/// The master screen is always anchored at virtual position (0, 0).
pub struct VirtualLayout {
    /// The master screen region (always at virtual_x=0, virtual_y=0).
    pub master: ScreenRegion,
    /// All client screens indexed by client ID.
    clients: HashMap<ClientId, ClientScreen>,
    /// Adjacency relationships between screen edges.
    adjacencies: Vec<Adjacency>,
}

impl VirtualLayout {
    /// Creates a new layout with the master screen at (0, 0).
    pub fn new(master_width: u32, master_height: u32) -> Self {
        Self {
            master: ScreenRegion {
                virtual_x: 0,
                virtual_y: 0,
                width: master_width,
                height: master_height,
            },
            clients: HashMap::new(),
            adjacencies: Vec::new(),
        }
    }

    /// Updates the master screen dimensions.
    pub fn set_master_dimensions(&mut self, width: u32, height: u32) {
        self.master.width = width;
        self.master.height = height;
    }

    /// Adds a client screen to the layout.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::Overlap`] if the new client region overlaps with any
    /// existing screen (master or other clients).
    pub fn add_client(&mut self, client: ClientScreen) -> Result<(), LayoutError> {
        if client.region.overlaps(&self.master) {
            return Err(LayoutError::Overlap);
        }
        for existing in self.clients.values() {
            if client.region.overlaps(&existing.region) {
                return Err(LayoutError::Overlap);
            }
        }
        self.clients.insert(client.client_id, client);
        Ok(())
    }

    /// Removes a client and all adjacencies referencing it.
    pub fn remove_client(&mut self, client_id: ClientId) {
        self.clients.remove(&client_id);
        self.adjacencies.retain(|adj| {
            adj.from_screen != ScreenId::Client(client_id)
                && adj.to_screen != ScreenId::Client(client_id)
        });
    }

    /// Updates the virtual position and size of an existing client screen.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::ClientNotFound`] if no client with that ID exists.
    /// Returns [`LayoutError::Overlap`] if the new region overlaps with another screen.
    pub fn update_client_region(
        &mut self,
        client_id: ClientId,
        region: ScreenRegion,
    ) -> Result<(), LayoutError> {
        if !self.clients.contains_key(&client_id) {
            return Err(LayoutError::ClientNotFound(client_id));
        }

        // Check for overlaps with master and other clients (excluding the client being updated)
        if region.overlaps(&self.master) {
            return Err(LayoutError::Overlap);
        }
        for (id, existing) in &self.clients {
            if *id != client_id && region.overlaps(&existing.region) {
                return Err(LayoutError::Overlap);
            }
        }

        if let Some(client) = self.clients.get_mut(&client_id) {
            client.region = region;
        }
        Ok(())
    }

    /// Defines an adjacency between two screen edges.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::InvalidAdjacency`] if either referenced screen does not exist.
    /// Returns [`LayoutError::IncompatibleEdges`] if the edge combination is invalid.
    pub fn set_adjacency(&mut self, adj: Adjacency) -> Result<(), LayoutError> {
        if !adj.is_valid() {
            return Err(LayoutError::IncompatibleEdges);
        }
        self.validate_screen_id(&adj.from_screen)?;
        self.validate_screen_id(&adj.to_screen)?;

        // Replace any existing adjacency for the same from_screen + from_edge pair
        self.adjacencies.retain(|a| {
            !(a.from_screen == adj.from_screen && a.from_edge == adj.from_edge)
        });
        self.adjacencies.push(adj);
        Ok(())
    }

    /// Removes all adjacencies.
    pub fn clear_adjacencies(&mut self) {
        self.adjacencies.clear();
    }

    /// Returns all client screens as an iterator.
    pub fn clients(&self) -> impl Iterator<Item = &ClientScreen> {
        self.clients.values()
    }

    /// Resolves a cursor position in virtual screen coordinates to a [`CursorLocation`].
    ///
    /// The `virtual_x` and `virtual_y` parameters are in the unified virtual coordinate space
    /// (where master occupies 0..master_width × 0..master_height).
    ///
    /// Returns [`CursorLocation::OnMaster`] when the cursor is within the master region,
    /// [`CursorLocation::OnClient`] when within a client region, or falls back to
    /// [`CursorLocation::OnMaster`] when the position is outside all known regions.
    pub fn resolve_cursor(&self, virtual_x: i32, virtual_y: i32) -> CursorLocation {
        // Check master first
        if virtual_x >= self.master.virtual_x
            && virtual_x < self.master.right()
            && virtual_y >= self.master.virtual_y
            && virtual_y < self.master.bottom()
        {
            return CursorLocation::OnMaster {
                local_x: virtual_x - self.master.virtual_x,
                local_y: virtual_y - self.master.virtual_y,
            };
        }

        // Check each client
        for client in self.clients.values() {
            if virtual_x >= client.region.virtual_x
                && virtual_x < client.region.right()
                && virtual_y >= client.region.virtual_y
                && virtual_y < client.region.bottom()
            {
                return CursorLocation::OnClient {
                    client_id: client.client_id,
                    local_x: virtual_x - client.region.virtual_x,
                    local_y: virtual_y - client.region.virtual_y,
                };
            }
        }

        // Default to master when outside all regions
        CursorLocation::OnMaster {
            local_x: virtual_x,
            local_y: virtual_y,
        }
    }

    /// Checks whether the cursor is within [`EDGE_THRESHOLD`] pixels of any configured
    /// transition edge on the given screen.
    ///
    /// `current_screen` identifies the screen the cursor is currently on.
    /// `local_x` and `local_y` are the cursor's position in that screen's local coordinate space.
    ///
    /// Returns `Some(EdgeTransition)` when a transition should occur, `None` otherwise.
    pub fn check_edge_transition(
        &self,
        current_screen: &ScreenId,
        local_x: i32,
        local_y: i32,
    ) -> Option<EdgeTransition> {
        let from_region = self.get_region(current_screen)?;

        for adj in &self.adjacencies {
            if &adj.from_screen != current_screen {
                continue;
            }

            let at_edge = match adj.from_edge {
                Edge::Right => local_x >= from_region.width as i32 - EDGE_THRESHOLD,
                Edge::Left => local_x <= EDGE_THRESHOLD - 1,
                Edge::Bottom => local_y >= from_region.height as i32 - EDGE_THRESHOLD,
                Edge::Top => local_y <= EDGE_THRESHOLD - 1,
            };

            if !at_edge {
                continue;
            }

            let to_region = self.get_region(&adj.to_screen)?;

            // Map the perpendicular cursor coordinate proportionally to the target edge
            let (entry_x, entry_y) = match (&adj.from_edge, &adj.to_edge) {
                (Edge::Right, Edge::Left) | (Edge::Left, Edge::Right) => {
                    let t = local_y as f64 / from_region.height as f64;
                    let mapped_y = (t * to_region.height as f64) as i32;
                    let entry_x = match adj.to_edge {
                        Edge::Left => 0,
                        Edge::Right => to_region.width as i32 - 1,
                        _ => 0,
                    };
                    (entry_x, mapped_y.clamp(0, to_region.height as i32 - 1))
                }
                (Edge::Bottom, Edge::Top) | (Edge::Top, Edge::Bottom) => {
                    let t = local_x as f64 / from_region.width as f64;
                    let mapped_x = (t * to_region.width as f64) as i32;
                    let entry_y = match adj.to_edge {
                        Edge::Top => 0,
                        Edge::Bottom => to_region.height as i32 - 1,
                        _ => 0,
                    };
                    (mapped_x.clamp(0, to_region.width as i32 - 1), entry_y)
                }
                _ => continue, // already validated as incompatible, skip
            };

            // Determine where to teleport the master physical cursor
            let (master_teleport_x, master_teleport_y) = match adj.from_edge {
                Edge::Right => (1, local_y),
                Edge::Left => (from_region.width as i32 - 2, local_y),
                Edge::Bottom => (local_x, 1),
                Edge::Top => (local_x, from_region.height as i32 - 2),
            };

            return Some(EdgeTransition {
                to_screen: adj.to_screen.clone(),
                entry_x,
                entry_y,
                master_teleport_x,
                master_teleport_y,
            });
        }

        None
    }

    /// Maps a position along one edge proportionally to the corresponding position
    /// on another edge.
    ///
    /// `from_length` is the total length of the source edge (e.g., screen height for Left/Right edges).
    /// `to_length` is the total length of the target edge.
    /// `pos` is the cursor's position along the source edge (0 = start, from_length = end).
    ///
    /// Returns the proportionally equivalent position on the target edge.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if `from_length` is zero.
    pub fn map_edge_position(from_length: u32, to_length: u32, pos: i32) -> i32 {
        if from_length == 0 {
            return 0;
        }
        let t = pos.clamp(0, from_length as i32) as f64 / from_length as f64;
        (t * to_length as f64) as i32
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn validate_screen_id(&self, id: &ScreenId) -> Result<(), LayoutError> {
        match id {
            ScreenId::Master => Ok(()),
            ScreenId::Client(cid) => {
                if self.clients.contains_key(cid) {
                    Ok(())
                } else {
                    Err(LayoutError::InvalidAdjacency)
                }
            }
        }
    }

    fn get_region(&self, id: &ScreenId) -> Option<&ScreenRegion> {
        match id {
            ScreenId::Master => Some(&self.master),
            ScreenId::Client(cid) => self.clients.get(cid).map(|c| &c.region),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_layout(mw: u32, mh: u32) -> VirtualLayout {
        VirtualLayout::new(mw, mh)
    }

    fn make_client(x: i32, y: i32, w: u32, h: u32) -> ClientScreen {
        ClientScreen {
            client_id: Uuid::new_v4(),
            region: ScreenRegion {
                virtual_x: x,
                virtual_y: y,
                width: w,
                height: h,
            },
            name: "test-client".to_string(),
        }
    }

    // ── ScreenRegion helpers ──────────────────────────────────────────────────

    #[test]
    fn test_screen_region_right_returns_virtual_x_plus_width() {
        let region = ScreenRegion { virtual_x: 100, virtual_y: 0, width: 1920, height: 1080 };
        assert_eq!(region.right(), 2020);
    }

    #[test]
    fn test_screen_region_bottom_returns_virtual_y_plus_height() {
        let region = ScreenRegion { virtual_x: 0, virtual_y: 50, width: 1920, height: 1080 };
        assert_eq!(region.bottom(), 1130);
    }

    #[test]
    fn test_screen_region_overlaps_when_regions_share_area() {
        let a = ScreenRegion { virtual_x: 0, virtual_y: 0, width: 100, height: 100 };
        let b = ScreenRegion { virtual_x: 50, virtual_y: 50, width: 100, height: 100 };
        assert!(a.overlaps(&b));
    }

    #[test]
    fn test_screen_region_does_not_overlap_when_adjacent() {
        let a = ScreenRegion { virtual_x: 0, virtual_y: 0, width: 100, height: 100 };
        let b = ScreenRegion { virtual_x: 100, virtual_y: 0, width: 100, height: 100 };
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn test_screen_region_does_not_overlap_when_separated() {
        let a = ScreenRegion { virtual_x: 0, virtual_y: 0, width: 100, height: 100 };
        let b = ScreenRegion { virtual_x: 200, virtual_y: 200, width: 100, height: 100 };
        assert!(!a.overlaps(&b));
    }

    // ── add_client ────────────────────────────────────────────────────────────

    #[test]
    fn test_add_client_succeeds_when_no_overlap() {
        let mut layout = make_layout(1920, 1080);
        let client = make_client(1920, 0, 2560, 1440);
        assert!(layout.add_client(client).is_ok());
    }

    #[test]
    fn test_add_client_fails_when_overlaps_master() {
        let mut layout = make_layout(1920, 1080);
        let client = make_client(0, 0, 1920, 1080); // exact same as master
        assert_eq!(layout.add_client(client), Err(LayoutError::Overlap));
    }

    #[test]
    fn test_add_client_fails_when_overlaps_existing_client() {
        let mut layout = make_layout(1920, 1080);
        let c1 = make_client(1920, 0, 1920, 1080);
        let c2 = make_client(2400, 0, 1920, 1080); // overlaps c1
        layout.add_client(c1).unwrap();
        assert_eq!(layout.add_client(c2), Err(LayoutError::Overlap));
    }

    #[test]
    fn test_add_multiple_non_overlapping_clients_succeeds() {
        let mut layout = make_layout(1920, 1080);
        layout.add_client(make_client(1920, 0, 1920, 1080)).unwrap();
        layout.add_client(make_client(0, 1080, 1920, 1080)).unwrap();
        assert_eq!(layout.clients().count(), 2);
    }

    // ── remove_client ─────────────────────────────────────────────────────────

    #[test]
    fn test_remove_client_removes_client_and_its_adjacencies() {
        let mut layout = make_layout(1920, 1080);
        let client = make_client(1920, 0, 1920, 1080);
        let cid = client.client_id;
        layout.add_client(client).unwrap();
        layout
            .set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Right,
                to_screen: ScreenId::Client(cid),
                to_edge: Edge::Left,
            })
            .unwrap();

        layout.remove_client(cid);

        assert_eq!(layout.clients().count(), 0);
        assert!(layout.adjacencies.is_empty());
    }

    // ── resolve_cursor ────────────────────────────────────────────────────────

    #[test]
    fn test_resolve_cursor_returns_on_master_when_within_master_region() {
        let layout = make_layout(1920, 1080);
        let loc = layout.resolve_cursor(960, 540);
        assert_eq!(loc, CursorLocation::OnMaster { local_x: 960, local_y: 540 });
    }

    #[test]
    fn test_resolve_cursor_returns_on_master_at_origin() {
        let layout = make_layout(1920, 1080);
        let loc = layout.resolve_cursor(0, 0);
        assert_eq!(loc, CursorLocation::OnMaster { local_x: 0, local_y: 0 });
    }

    #[test]
    fn test_resolve_cursor_returns_on_master_at_bottom_right_corner() {
        let layout = make_layout(1920, 1080);
        let loc = layout.resolve_cursor(1919, 1079); // last pixel inside master
        assert_eq!(loc, CursorLocation::OnMaster { local_x: 1919, local_y: 1079 });
    }

    #[test]
    fn test_resolve_cursor_returns_on_client_when_within_client_region() {
        let mut layout = make_layout(1920, 1080);
        let client = make_client(1920, 0, 2560, 1440);
        let cid = client.client_id;
        layout.add_client(client).unwrap();

        let loc = layout.resolve_cursor(1920 + 100, 200);
        assert_eq!(
            loc,
            CursorLocation::OnClient {
                client_id: cid,
                local_x: 100,
                local_y: 200,
            }
        );
    }

    #[test]
    fn test_resolve_cursor_returns_on_master_when_outside_all_regions() {
        let layout = make_layout(1920, 1080);
        // Position to the right of the master with no client there
        let loc = layout.resolve_cursor(5000, 5000);
        assert!(matches!(loc, CursorLocation::OnMaster { .. }));
    }

    // ── check_edge_transition ─────────────────────────────────────────────────

    #[test]
    fn test_check_edge_transition_returns_none_when_cursor_far_from_edge() {
        let mut layout = make_layout(1920, 1080);
        let client = make_client(1920, 0, 1920, 1080);
        let cid = client.client_id;
        layout.add_client(client).unwrap();
        layout
            .set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Right,
                to_screen: ScreenId::Client(cid),
                to_edge: Edge::Left,
            })
            .unwrap();

        let result = layout.check_edge_transition(&ScreenId::Master, 960, 540);
        assert!(result.is_none(), "cursor far from edge should not trigger transition");
    }

    #[test]
    fn test_check_edge_transition_returns_some_when_cursor_at_right_edge() {
        let mut layout = make_layout(1920, 1080);
        let client = make_client(1920, 0, 1920, 1080);
        let cid = client.client_id;
        layout.add_client(client).unwrap();
        layout
            .set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Right,
                to_screen: ScreenId::Client(cid),
                to_edge: Edge::Left,
            })
            .unwrap();

        // Cursor is within EDGE_THRESHOLD (2px) of the right edge
        let result = layout.check_edge_transition(&ScreenId::Master, 1919, 540);
        assert!(result.is_some(), "cursor within threshold of right edge should trigger transition");
    }

    #[test]
    fn test_check_edge_transition_proportionally_maps_y_coordinate() {
        // Master: 1920x1080, client: 2560x1440 placed to the right
        let mut layout = make_layout(1920, 1080);
        let client = make_client(1920, 0, 2560, 1440);
        let cid = client.client_id;
        layout.add_client(client).unwrap();
        layout
            .set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Right,
                to_screen: ScreenId::Client(cid),
                to_edge: Edge::Left,
            })
            .unwrap();

        // Cursor at 50% height of master (y=540) should map to 50% of client height (y=720)
        let transition = layout
            .check_edge_transition(&ScreenId::Master, 1919, 540)
            .expect("should transition");

        assert_eq!(transition.to_screen, ScreenId::Client(cid));
        assert_eq!(transition.entry_x, 0, "entering left edge at x=0");
        // 540/1080 * 1440 = 720
        assert_eq!(transition.entry_y, 720, "y should be proportionally mapped to 720");
    }

    #[test]
    fn test_check_edge_transition_returns_correct_master_teleport_position() {
        let mut layout = make_layout(1920, 1080);
        let client = make_client(1920, 0, 1920, 1080);
        let cid = client.client_id;
        layout.add_client(client).unwrap();
        layout
            .set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Right,
                to_screen: ScreenId::Client(cid),
                to_edge: Edge::Left,
            })
            .unwrap();

        let transition = layout
            .check_edge_transition(&ScreenId::Master, 1919, 540)
            .expect("should transition");

        // After right-edge transition, master cursor should teleport to x=1 (near left edge)
        assert_eq!(transition.master_teleport_x, 1);
        assert_eq!(transition.master_teleport_y, 540);
    }

    #[test]
    fn test_check_edge_transition_top_to_bottom_proportional_mapping() {
        // Client above master
        let mut layout = make_layout(1920, 1080);
        let client = make_client(0, -1440, 2560, 1440);
        let cid = client.client_id;
        layout.add_client(client).unwrap();
        layout
            .set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Top,
                to_screen: ScreenId::Client(cid),
                to_edge: Edge::Bottom,
            })
            .unwrap();

        // Cursor at left edge (x=0) of master top should map to left edge of client bottom
        let transition = layout
            .check_edge_transition(&ScreenId::Master, 0, 0)
            .expect("should transition at top edge");

        assert_eq!(transition.entry_y, 1439, "entering bottom edge at last row");
        // x=0 of 1920 → proportional x of 2560: 0/1920 * 2560 = 0
        assert_eq!(transition.entry_x, 0);
    }

    #[test]
    fn test_check_edge_transition_returns_none_when_no_adjacency_for_edge() {
        let mut layout = make_layout(1920, 1080);
        let client = make_client(1920, 0, 1920, 1080);
        let cid = client.client_id;
        layout.add_client(client).unwrap();
        // Only add right-edge adjacency
        layout
            .set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Right,
                to_screen: ScreenId::Client(cid),
                to_edge: Edge::Left,
            })
            .unwrap();

        // Cursor at bottom edge of master - no adjacency defined there
        let result = layout.check_edge_transition(&ScreenId::Master, 960, 1079);
        assert!(result.is_none());
    }

    // ── set_adjacency ─────────────────────────────────────────────────────────

    #[test]
    fn test_set_adjacency_rejects_incompatible_edges() {
        let mut layout = make_layout(1920, 1080);
        let client = make_client(1920, 0, 1920, 1080);
        let cid = client.client_id;
        layout.add_client(client).unwrap();

        let result = layout.set_adjacency(Adjacency {
            from_screen: ScreenId::Master,
            from_edge: Edge::Right,
            to_screen: ScreenId::Client(cid),
            to_edge: Edge::Right, // same side – invalid
        });
        assert_eq!(result, Err(LayoutError::IncompatibleEdges));
    }

    #[test]
    fn test_set_adjacency_rejects_unknown_client_id() {
        let mut layout = make_layout(1920, 1080);
        let unknown_id = Uuid::new_v4();

        let result = layout.set_adjacency(Adjacency {
            from_screen: ScreenId::Master,
            from_edge: Edge::Right,
            to_screen: ScreenId::Client(unknown_id),
            to_edge: Edge::Left,
        });
        assert_eq!(result, Err(LayoutError::InvalidAdjacency));
    }

    #[test]
    fn test_set_adjacency_replaces_existing_adjacency_for_same_edge() {
        let mut layout = make_layout(1920, 1080);
        let c1 = make_client(1920, 0, 1920, 1080);
        let c2 = make_client(3840, 0, 1920, 1080);
        let cid1 = c1.client_id;
        let cid2 = c2.client_id;
        layout.add_client(c1).unwrap();
        layout.add_client(c2).unwrap();

        layout
            .set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Right,
                to_screen: ScreenId::Client(cid1),
                to_edge: Edge::Left,
            })
            .unwrap();
        layout
            .set_adjacency(Adjacency {
                from_screen: ScreenId::Master,
                from_edge: Edge::Right,
                to_screen: ScreenId::Client(cid2),
                to_edge: Edge::Left,
            })
            .unwrap();

        // Should have only one adjacency for master's right edge
        let count = layout
            .adjacencies
            .iter()
            .filter(|a| a.from_screen == ScreenId::Master && a.from_edge == Edge::Right)
            .count();
        assert_eq!(count, 1);
    }

    // ── map_edge_position ─────────────────────────────────────────────────────

    #[test]
    fn test_map_edge_position_maps_center_proportionally_for_same_size_screens() {
        // 50% of 1080 maps to 50% of 1080
        let result = VirtualLayout::map_edge_position(1080, 1080, 540);
        assert_eq!(result, 540);
    }

    #[test]
    fn test_map_edge_position_maps_proportionally_for_different_size_screens() {
        // 540/1080 * 1440 = 720
        let result = VirtualLayout::map_edge_position(1080, 1440, 540);
        assert_eq!(result, 720);
    }

    #[test]
    fn test_map_edge_position_zero_pos_maps_to_zero() {
        let result = VirtualLayout::map_edge_position(1080, 1440, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_map_edge_position_full_length_maps_to_full_target_length() {
        let result = VirtualLayout::map_edge_position(1080, 1440, 1080);
        assert_eq!(result, 1440);
    }

    #[test]
    fn test_map_edge_position_clamps_negative_pos_to_zero() {
        let result = VirtualLayout::map_edge_position(1080, 1440, -50);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_map_edge_position_with_zero_from_length_returns_zero() {
        let result = VirtualLayout::map_edge_position(0, 1440, 100);
        assert_eq!(result, 0);
    }

    // ── update_client_region ──────────────────────────────────────────────────

    #[test]
    fn test_update_client_region_succeeds_for_valid_move() {
        let mut layout = make_layout(1920, 1080);
        let client = make_client(1920, 0, 1920, 1080);
        let cid = client.client_id;
        layout.add_client(client).unwrap();

        let new_region = ScreenRegion {
            virtual_x: 1920,
            virtual_y: 1080,
            width: 1920,
            height: 1080,
        };
        assert!(layout.update_client_region(cid, new_region).is_ok());
    }

    #[test]
    fn test_update_client_region_fails_for_unknown_client() {
        let mut layout = make_layout(1920, 1080);
        let unknown_id = Uuid::new_v4();

        let new_region = ScreenRegion {
            virtual_x: 1920,
            virtual_y: 0,
            width: 1920,
            height: 1080,
        };
        assert_eq!(
            layout.update_client_region(unknown_id, new_region),
            Err(LayoutError::ClientNotFound(unknown_id))
        );
    }

    #[test]
    fn test_update_client_region_fails_when_new_region_overlaps_master() {
        let mut layout = make_layout(1920, 1080);
        let client = make_client(1920, 0, 1920, 1080);
        let cid = client.client_id;
        layout.add_client(client).unwrap();

        let overlapping_region = ScreenRegion {
            virtual_x: 0, // overlaps master
            virtual_y: 0,
            width: 1920,
            height: 1080,
        };
        assert_eq!(
            layout.update_client_region(cid, overlapping_region),
            Err(LayoutError::Overlap)
        );
    }
}
