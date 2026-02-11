//! Integration tests for the connection manager and pairing lifecycle.
//!
//! These tests exercise the network layer of kvm-master end-to-end:
//! `ConnectionManager` pairing state machine + retry/lockout logic.

use uuid::Uuid;

// ── Pairing lifecycle tests ───────────────────────────────────────────────────

#[test]
fn test_pairing_lifecycle_initiate_then_verify_succeeds() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig,
    };
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());
    let client_id = Uuid::new_v4();
    let addr: std::net::IpAddr = "10.0.0.1".parse().unwrap();

    // Arrange: initiate pairing
    let (session_id, pin) = mgr.initiate_pairing(client_id, addr).expect("initiate");

    // Compute the expected pin hash (mirrors the hash_pin private function)
    let mut hasher = DefaultHasher::new();
    pin.hash(&mut hasher);
    session_id.as_bytes().hash(&mut hasher);
    let pin_hash = format!("{:016x}", hasher.finish());

    // Act
    let paired_id = mgr
        .verify_pairing_pin(session_id, &pin_hash, addr)
        .expect("verify must succeed with correct hash");

    // Assert
    assert_eq!(paired_id, client_id);
    assert!(mgr.is_paired(client_id), "client must be marked as paired");
}

#[test]
fn test_pairing_wrong_pin_increments_failure_counter() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig, PairingError,
    };

    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());
    let client_id = Uuid::new_v4();
    let addr: std::net::IpAddr = "10.0.0.2".parse().unwrap();

    let (session_id, _pin) = mgr.initiate_pairing(client_id, addr).expect("initiate");

    // First wrong attempt: 2 attempts remaining
    let result = mgr.verify_pairing_pin(session_id, "bad_hash_1", addr);
    assert!(
        matches!(result, Err(PairingError::WrongPin { attempts_remaining: 2 })),
        "expected WrongPin with 2 remaining, got: {:?}",
        result
    );
}

#[test]
fn test_pairing_three_failures_trigger_lockout() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig, PairingError,
    };

    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());
    let client_id = Uuid::new_v4();
    let addr: std::net::IpAddr = "10.0.0.3".parse().unwrap();

    let (session_id, _pin) = mgr.initiate_pairing(client_id, addr).expect("initiate");

    // Exhaust all 3 attempts
    for i in 0..3u8 {
        let _ = mgr.verify_pairing_pin(session_id, &format!("bad_{i}"), addr);
    }

    // Any further operation from this IP must be refused
    let result = mgr.initiate_pairing(Uuid::new_v4(), addr);
    assert!(
        matches!(result, Err(PairingError::LockedOut { .. })),
        "IP must be locked out after 3 failures, got: {:?}",
        result
    );
}

#[test]
fn test_pairing_session_not_found_for_unknown_session_id() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig, PairingError,
    };

    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());
    let addr: std::net::IpAddr = "10.0.0.4".parse().unwrap();

    let result = mgr.verify_pairing_pin(Uuid::new_v4(), "irrelevant_hash", addr);
    assert_eq!(
        result,
        Err(PairingError::SessionNotFound),
        "unknown session id must return SessionNotFound"
    );
}

#[test]
fn test_multiple_clients_can_pair_independently() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig,
    };
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());

    let pairs: Vec<(Uuid, std::net::IpAddr)> = vec![
        (Uuid::new_v4(), "10.0.1.1".parse().unwrap()),
        (Uuid::new_v4(), "10.0.1.2".parse().unwrap()),
        (Uuid::new_v4(), "10.0.1.3".parse().unwrap()),
    ];

    for (client_id, addr) in &pairs {
        let (session_id, pin) = mgr
            .initiate_pairing(*client_id, *addr)
            .expect("initiate must succeed");

        let mut hasher = DefaultHasher::new();
        pin.hash(&mut hasher);
        session_id.as_bytes().hash(&mut hasher);
        let pin_hash = format!("{:016x}", hasher.finish());

        let result = mgr.verify_pairing_pin(session_id, &pin_hash, *addr);
        assert_eq!(result, Ok(*client_id));
    }

    // All three must be paired
    for (client_id, _) in &pairs {
        assert!(
            mgr.is_paired(*client_id),
            "client {} must be paired",
            client_id
        );
    }
}

#[test]
fn test_is_not_paired_for_unregistered_client() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig,
    };

    let (mgr, _rx) = ConnectionManager::new(NetworkConfig::default());

    // A fresh UUID has never been registered
    assert!(
        !mgr.is_paired(Uuid::new_v4()),
        "randomly generated UUID must not be paired"
    );
}

#[test]
fn test_network_config_default_values_are_sensible() {
    use kvm_master::infrastructure::network::connection_manager::NetworkConfig;

    let cfg = NetworkConfig::default();
    assert_eq!(cfg.control_port, 24800, "default control port must be 24800");
    assert_eq!(cfg.input_port, 24801, "default input port must be 24801");
    assert_eq!(
        cfg.discovery_port, 24802,
        "default discovery port must be 24802"
    );
}
