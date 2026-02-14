//! Integration tests for the input routing pipeline.
//!
//! # Purpose
//!
//! These tests exercise the application layer of kvm-master through its
//! *public* API to verify that the routing and client management components
//! work correctly end-to-end.  They cover:
//!
//! - **Layout building**: `build_layout` creates a valid `VirtualLayout` from
//!   a list of client position configs.
//! - **Layout validation**: `build_layout` rejects configurations where two
//!   screens overlap in virtual coordinate space.
//! - **Client registry**: `ClientRegistry` stores, retrieves, and removes
//!   `ClientRuntimeState` records correctly.
//! - **Pairing lifecycle**: re-verifies the `ConnectionManager` pairing flow
//!   as a cross-cutting integration test.
//!
//! # How routing works (background)
//!
//! The master maintains a `VirtualLayout` — a 2D map of all screens in virtual
//! coordinate space.  When the cursor approaches a screen edge that has a
//! configured adjacency, the master switches its input routing target from the
//! current screen to the neighbouring screen.
//!
//! `build_layout` constructs this `VirtualLayout` from the user's configuration
//! (stored in `~/.config/kvm-master/config.toml`).  If two screens are placed
//! at overlapping positions, the layout is rejected immediately at build time
//! rather than causing silent bugs at runtime.

use uuid::Uuid;

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Tests that `build_layout` successfully creates a layout containing a single
/// client screen placed to the right of the master.
///
/// The master is always implicitly at virtual (0, 0) with the dimensions
/// passed as the first two arguments.  The client at (1920, 0) forms a
/// seamless pair with the master on a dual-monitor-like setup.
#[test]
fn test_build_layout_with_client_to_the_right_of_master() {
    use kvm_master::application::update_layout::{build_layout, ClientLayoutConfig};

    let client_id = Uuid::new_v4();
    let configs = vec![ClientLayoutConfig {
        client_id,
        name: "right-screen".to_string(),
        // Place the client immediately to the right of the 1920-pixel-wide master.
        x_offset: 1920,
        y_offset: 0,
        width: 1920,
        height: 1080,
    }];

    // `build_layout(master_width, master_height, client_configs)`
    let layout = build_layout(1920, 1080, configs).expect("layout must be valid");

    // Verify the client appears in the layout's client iterator.
    let found = layout.clients().any(|c| c.client_id == client_id);
    assert!(found, "client must be present in the layout after build");
}

/// Tests that `build_layout` rejects a configuration where two clients are
/// placed at the same virtual coordinates as the master (0, 0).
///
/// Overlapping screens would make `resolve_cursor` non-deterministic: the
/// cursor could be "on" two screens simultaneously, so the routing target
/// would be ambiguous.  `build_layout` must catch this and return an error.
#[test]
fn test_build_layout_rejects_overlapping_clients() {
    use kvm_master::application::update_layout::{build_layout, ClientLayoutConfig};

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let configs = vec![
        ClientLayoutConfig {
            client_id: id1,
            name: "screen1".to_string(),
            x_offset: 0,
            y_offset: 0,
            width: 1920,
            height: 1080,
        },
        ClientLayoutConfig {
            client_id: id2,
            name: "screen2".to_string(),
            // Same position as master (0,0) — this overlaps the master screen.
            x_offset: 0,
            y_offset: 0,
            width: 1920,
            height: 1080,
        },
    ];

    let result = build_layout(1920, 1080, configs);
    // The function must return Err; any error variant is acceptable here.
    assert!(result.is_err(), "overlapping layout must be rejected");
}

/// Tests that `ClientRegistry::upsert` inserts a new client and that
/// `ClientRegistry::get` retrieves it correctly.
///
/// # What is ClientRegistry?
///
/// `ClientRegistry` is an in-memory store of `ClientRuntimeState` records,
/// keyed by client UUID.  It is the master's source of truth for which clients
/// are known, their connection state (Discovered / Connecting / Connected /
/// Paired / Disconnected), and their current latency/event-rate statistics.
///
/// `upsert` inserts the record if the UUID is new, or replaces it if the UUID
/// already exists (hence "upsert" = insert-or-update).
#[test]
fn test_client_registry_upsert_and_retrieve() {
    use kvm_master::application::manage_clients::{
        ClientRegistry, ClientRuntimeState, ConnectionState,
    };

    let mut registry = ClientRegistry::new();
    let id = Uuid::new_v4();

    // Build a minimal ClientRuntimeState for a newly discovered client.
    // `latency_ms: 0.0` and `events_per_second: 0` are the defaults before
    // any connection has been made.
    let state = ClientRuntimeState {
        id,
        name: "test-client".to_string(),
        connection_state: ConnectionState::Discovered,
        latency_ms: 0.0,
        events_per_second: 0,
    };

    // Insert the record.
    registry.upsert(state.clone());

    // Retrieve it and check individual fields.
    let retrieved = registry.get(id).expect("client must be present after upsert");
    assert_eq!(retrieved.name, "test-client");
    assert!(matches!(retrieved.connection_state, ConnectionState::Discovered));
}

/// Tests that `ClientRegistry::remove` deletes a client record so that a
/// subsequent `get` returns `None`.
///
/// A client is removed from the registry when it disconnects and is not
/// expected to reconnect (e.g., the user explicitly unpairs it from the UI).
/// After removal, `is_paired` and `get` must both return `false`/`None`.
#[test]
fn test_client_registry_remove() {
    use kvm_master::application::manage_clients::{
        ClientRegistry, ClientRuntimeState, ConnectionState,
    };

    let mut registry = ClientRegistry::new();
    let id = Uuid::new_v4();

    // Insert a client in the Connected state.
    registry.upsert(ClientRuntimeState {
        id,
        name: "removable".to_string(),
        connection_state: ConnectionState::Connected,
        latency_ms: 0.0,
        events_per_second: 0,
    });

    // Remove it.
    registry.remove(id);

    // After removal, get() must return None.
    assert!(registry.get(id).is_none(), "client must be absent after remove");
}

/// Tests the full pairing lifecycle: initiate → verify with correct hash →
/// confirm `is_paired` is true.
///
/// This is a cross-cutting test that exercises both `ConnectionManager` (in the
/// infrastructure layer) and the UUID/hash types from `kvm-core`.  It acts as
/// an integration checkpoint to ensure the layers compose correctly.
///
/// See `connection_integration.rs` for more focused tests on pairing error
/// cases (wrong PIN, lockout, session not found).
#[test]
fn test_connection_manager_pairing_lifecycle() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig,
    };

    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());
    let client_id = Uuid::new_v4();
    let addr: std::net::IpAddr = "192.168.1.100".parse().unwrap();

    // Step 1: Confirm the client is not yet paired.
    assert!(!mgr.is_paired(client_id));

    // Step 2: Initiate pairing — master generates a session UUID and a PIN.
    let (session_id, pin) = mgr.initiate_pairing(client_id, addr).expect("initiate");

    // Step 3: Compute the PIN hash the same way the client code would.
    // In production, the client receives the session_id in the PairingRequest
    // message and combines it with the PIN the user types.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    pin.hash(&mut hasher);
    session_id.as_bytes().hash(&mut hasher);
    let pin_hash = format!("{:016x}", hasher.finish());

    // Step 4: Submit the correct hash — must succeed and return the client_id.
    let paired_id = mgr
        .verify_pairing_pin(session_id, &pin_hash, addr)
        .expect("verify must succeed");

    // Step 5: Confirm the client is now recognised as paired.
    assert_eq!(paired_id, client_id);
    assert!(mgr.is_paired(client_id));
}
