//! Integration tests for the input routing pipeline.
//!
//! These tests exercise the application layer of kvm-master end-to-end:
//! `RouteInputUseCase` + `VirtualLayout` + mock infrastructure.

use uuid::Uuid;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_build_layout_with_client_to_the_right_of_master() {
    use kvm_master::application::update_layout::{build_layout, ClientLayoutConfig};

    let client_id = Uuid::new_v4();
    let configs = vec![ClientLayoutConfig {
        client_id,
        name: "right-screen".to_string(),
        x_offset: 1920,
        y_offset: 0,
        width: 1920,
        height: 1080,
    }];

    let layout = build_layout(1920, 1080, configs).expect("layout must be valid");

    // Client should be accessible from the layout via the clients() iterator
    let found = layout.clients().any(|c| c.client_id == client_id);
    assert!(found, "client must be present in the layout after build");
}

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
            // Same position as master (0,0) -> overlaps
            x_offset: 0,
            y_offset: 0,
            width: 1920,
            height: 1080,
        },
    ];

    let result = build_layout(1920, 1080, configs);
    assert!(result.is_err(), "overlapping layout must be rejected");
}

#[test]
fn test_client_registry_upsert_and_retrieve() {
    use kvm_master::application::manage_clients::{
        ClientRegistry, ClientRuntimeState, ConnectionState,
    };

    let mut registry = ClientRegistry::new();
    let id = Uuid::new_v4();

    let state = ClientRuntimeState {
        id,
        name: "test-client".to_string(),
        connection_state: ConnectionState::Discovered,
        latency_ms: 0.0,
        events_per_second: 0,
    };

    registry.upsert(state.clone());

    let retrieved = registry.get(id).expect("client must be present after upsert");
    assert_eq!(retrieved.name, "test-client");
    assert!(matches!(retrieved.connection_state, ConnectionState::Discovered));
}

#[test]
fn test_client_registry_remove() {
    use kvm_master::application::manage_clients::{
        ClientRegistry, ClientRuntimeState, ConnectionState,
    };

    let mut registry = ClientRegistry::new();
    let id = Uuid::new_v4();

    registry.upsert(ClientRuntimeState {
        id,
        name: "removable".to_string(),
        connection_state: ConnectionState::Connected,
        latency_ms: 0.0,
        events_per_second: 0,
    });

    registry.remove(id);

    assert!(registry.get(id).is_none(), "client must be absent after remove");
}

#[test]
fn test_connection_manager_pairing_lifecycle() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig,
    };

    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());
    let client_id = Uuid::new_v4();
    let addr: std::net::IpAddr = "192.168.1.100".parse().unwrap();

    // Before pairing
    assert!(!mgr.is_paired(client_id));

    // Initiate
    let (session_id, pin) = mgr.initiate_pairing(client_id, addr).expect("initiate");

    // Compute the expected hash (mirrors the private hash_pin function via the public API)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    pin.hash(&mut hasher);
    session_id.as_bytes().hash(&mut hasher);
    let pin_hash = format!("{:016x}", hasher.finish());

    let paired_id = mgr
        .verify_pairing_pin(session_id, &pin_hash, addr)
        .expect("verify must succeed");

    assert_eq!(paired_id, client_id);
    assert!(mgr.is_paired(client_id));
}
