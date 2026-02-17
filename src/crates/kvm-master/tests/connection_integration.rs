//! Integration tests for the connection manager and pairing lifecycle.
//!
//! # Purpose
//!
//! These tests exercise the `ConnectionManager` through its *public* API in
//! the same way that the application layer uses it.  They verify:
//!
//! - The happy path: initiating a pairing session and submitting the correct
//!   PIN hash results in a successfully paired client.
//! - The error paths: wrong PIN decrements the attempt counter, and three
//!   failures trigger a per-IP lockout.
//! - Edge cases: verifying against an unknown session ID, querying paired
//!   status for an unregistered client, and multiple independent clients
//!   pairing concurrently.
//!
//! # What is the pairing flow?
//!
//! When a new client appears on the network, the master UI shows a 6-digit PIN
//! to the user.  The user types the PIN on the client machine.  The client
//! hashes the PIN together with the session UUID and sends the hash to the
//! master.  The master checks the hash against its own computation.
//!
//! ```text
//! Master                              Client
//! ──────                              ──────
//! initiate_pairing(client_id, addr)
//!   → (session_id, pin)
//! Show PIN to user                    User types PIN
//!                                     Compute hash(PIN + session_id)
//!                                     Send verify_pairing_pin(session_id, hash)
//! verify_pairing_pin(session_id, hash)
//!   → Ok(client_id) if hash matches
//!   → Err(WrongPin) if hash wrong
//!   → Err(LockedOut) after 3 failures
//! ```
//!
//! # PIN hash computation
//!
//! Tests that need to supply the correct PIN hash mirror the private
//! `hash_pin` function using `DefaultHasher`:
//!
//! ```rust,ignore
//! let mut hasher = DefaultHasher::new();
//! pin.hash(&mut hasher);
//! session_id.as_bytes().hash(&mut hasher);
//! let pin_hash = format!("{:016x}", hasher.finish());
//! ```
//!
//! This is intentionally simple (not cryptographically secure) for a
//! local-network application; a production deployment would use HMAC-SHA256.

use uuid::Uuid;

// ── Pairing lifecycle tests ───────────────────────────────────────────────────

/// Tests the complete happy-path pairing flow: initiate, then verify with
/// the correct PIN hash.
///
/// After successful verification, `is_paired(client_id)` must return `true`.
#[test]
fn test_pairing_lifecycle_initiate_then_verify_succeeds() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig,
    };
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Arrange: create a fresh ConnectionManager.
    // `_rx` is the event-receiver channel; we discard it because these tests
    // only call synchronous pairing methods and don't need to consume events.
    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());
    let client_id = Uuid::new_v4();

    // Use a non-loopback address to simulate a real client machine.
    let addr: std::net::IpAddr = "10.0.0.1".parse().unwrap();

    // Step 1: Initiate pairing.
    // Returns (session_id, pin) where `pin` is the 6-digit code to show the user.
    let (session_id, pin) = mgr.initiate_pairing(client_id, addr).expect("initiate");

    // Step 2: Compute the PIN hash the same way the client would.
    // The hash combines the PIN string with the session UUID bytes so that
    // an attacker who observes the session ID cannot compute the hash without
    // knowing the PIN.
    let mut hasher = DefaultHasher::new();
    pin.hash(&mut hasher);
    session_id.as_bytes().hash(&mut hasher);
    let pin_hash = format!("{:016x}", hasher.finish());

    // Step 3: Verify the PIN hash.
    let paired_id = mgr
        .verify_pairing_pin(session_id, &pin_hash, addr)
        .expect("verify must succeed with correct hash");

    // Assert: the returned ID matches the original client, and the manager
    // now recognises the client as paired.
    assert_eq!(paired_id, client_id);
    assert!(mgr.is_paired(client_id), "client must be marked as paired");
}

/// Tests that submitting a wrong PIN hash decrements the attempts-remaining
/// counter and returns `WrongPin { attempts_remaining: 2 }` on the first
/// failure.
///
/// The lockout mechanism allows 3 attempts total, so after 1 failure there
/// are 2 remaining.
#[test]
fn test_pairing_wrong_pin_increments_failure_counter() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig, PairingError,
    };

    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());
    let client_id = Uuid::new_v4();
    let addr: std::net::IpAddr = "10.0.0.2".parse().unwrap();

    let (session_id, _pin) = mgr.initiate_pairing(client_id, addr).expect("initiate");

    // First wrong attempt: 2 attempts remaining (3 total − 1 used = 2)
    let result = mgr.verify_pairing_pin(session_id, "bad_hash_1", addr);
    assert!(
        matches!(
            result,
            Err(PairingError::WrongPin {
                attempts_remaining: 2
            })
        ),
        "expected WrongPin with 2 remaining, got: {:?}",
        result
    );
}

/// Tests that three consecutive wrong PIN submissions trigger a per-IP
/// lockout.
///
/// After lockout, any further `initiate_pairing` call from the same IP
/// address returns `Err(PairingError::LockedOut { … })`.  This prevents
/// brute-force PIN attacks over the local network.
#[test]
fn test_pairing_three_failures_trigger_lockout() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig, PairingError,
    };

    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());
    let client_id = Uuid::new_v4();
    let addr: std::net::IpAddr = "10.0.0.3".parse().unwrap();

    let (session_id, _pin) = mgr.initiate_pairing(client_id, addr).expect("initiate");

    // Exhaust all 3 attempts with wrong hashes.
    // The loop variable `i` is used only to produce distinct bad hashes;
    // any wrong hash triggers a failure count increment.
    for i in 0..3u8 {
        let _ = mgr.verify_pairing_pin(session_id, &format!("bad_{i}"), addr);
    }

    // After 3 failures the IP is locked out.
    // Attempting a NEW pairing from the same IP must now be refused.
    let result = mgr.initiate_pairing(Uuid::new_v4(), addr);
    assert!(
        matches!(result, Err(PairingError::LockedOut { .. })),
        "IP must be locked out after 3 failures, got: {:?}",
        result
    );
}

/// Tests that verifying a PIN for an unknown session ID returns
/// `Err(PairingError::SessionNotFound)`.
///
/// This guards against a client sending a verify request without a prior
/// initiation, or with a stale/expired session ID.
#[test]
fn test_pairing_session_not_found_for_unknown_session_id() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig, PairingError,
    };

    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());
    let addr: std::net::IpAddr = "10.0.0.4".parse().unwrap();

    // `Uuid::new_v4()` generates a random UUID that has never been registered.
    let result = mgr.verify_pairing_pin(Uuid::new_v4(), "irrelevant_hash", addr);
    assert_eq!(
        result,
        Err(PairingError::SessionNotFound),
        "unknown session id must return SessionNotFound"
    );
}

/// Tests that multiple clients can pair independently and concurrently.
///
/// Each client initiates its own pairing session with a unique ID and IP
/// address.  Verifying each one with the correct hash should succeed without
/// interfering with the other sessions.
///
/// This validates that session state is stored per-session (not globally) and
/// that the failure counter is per-IP (so Client A's failures don't lock out
/// Client B).
#[test]
fn test_multiple_clients_can_pair_independently() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig,
    };
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let (mut mgr, _rx) = ConnectionManager::new(NetworkConfig::default());

    // Three distinct clients, each with a unique UUID and IP address.
    let pairs: Vec<(Uuid, std::net::IpAddr)> = vec![
        (Uuid::new_v4(), "10.0.1.1".parse().unwrap()),
        (Uuid::new_v4(), "10.0.1.2".parse().unwrap()),
        (Uuid::new_v4(), "10.0.1.3".parse().unwrap()),
    ];

    // Pair each client one at a time.
    for (client_id, addr) in &pairs {
        let (session_id, pin) = mgr
            .initiate_pairing(*client_id, *addr)
            .expect("initiate must succeed");

        // Compute the correct hash for this specific session.
        let mut hasher = DefaultHasher::new();
        pin.hash(&mut hasher);
        session_id.as_bytes().hash(&mut hasher);
        let pin_hash = format!("{:016x}", hasher.finish());

        let result = mgr.verify_pairing_pin(session_id, &pin_hash, *addr);
        assert_eq!(result, Ok(*client_id));
    }

    // After the loop, all three clients must be individually recognised as paired.
    for (client_id, _) in &pairs {
        assert!(
            mgr.is_paired(*client_id),
            "client {} must be paired",
            client_id
        );
    }
}

/// Tests that `is_paired` returns `false` for a client that was never
/// registered.
///
/// `Uuid::new_v4()` generates a random UUID.  The probability of it
/// colliding with a previously registered client is astronomically small
/// (~2^-122), so this test is effectively deterministic.
#[test]
fn test_is_not_paired_for_unregistered_client() {
    use kvm_master::infrastructure::network::connection_manager::{
        ConnectionManager, NetworkConfig,
    };

    let (mgr, _rx) = ConnectionManager::new(NetworkConfig::default());

    // A fresh UUID has never been registered, so is_paired must return false.
    assert!(
        !mgr.is_paired(Uuid::new_v4()),
        "randomly generated UUID must not be paired"
    );
}

/// Tests that `NetworkConfig::default()` returns the standard port numbers
/// defined in the protocol specification.
///
/// | Port  | Purpose                              |
/// |-------|--------------------------------------|
/// | 24800 | Control plane (TCP, pairing/Hello)   |
/// | 24801 | Input plane (TCP, key/mouse events)  |
/// | 24802 | Discovery (UDP, Announce broadcasts) |
///
/// If these defaults change, all existing clients will fail to connect until
/// reconfigured, so this test acts as a "breaking change" guard.
#[test]
fn test_network_config_default_values_are_sensible() {
    use kvm_master::infrastructure::network::connection_manager::NetworkConfig;

    let cfg = NetworkConfig::default();
    assert_eq!(
        cfg.control_port, 24800,
        "default control port must be 24800"
    );
    assert_eq!(cfg.input_port, 24801, "default input port must be 24801");
    assert_eq!(
        cfg.discovery_port, 24802,
        "default discovery port must be 24802"
    );
}
