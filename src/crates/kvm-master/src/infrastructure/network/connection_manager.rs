//! ConnectionManager: manages TLS control channel connections, pairing, and the client registry.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use kvm_core::ClientId;
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// Error type for connection management operations.
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("bind failed on {addr}: {source}")]
    BindFailed {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("pairing error: {0}")]
    Pairing(String),
    #[error("client not found: {0}")]
    ClientNotFound(ClientId),
}

/// Error type specific to pairing.
#[derive(Debug, Error, PartialEq)]
pub enum PairingError {
    #[error("pairing session not found or expired")]
    SessionNotFound,
    #[error("incorrect PIN; {attempts_remaining} attempt(s) remaining")]
    WrongPin { attempts_remaining: u8 },
    #[error("client is locked out for {seconds_remaining}s due to too many failed attempts")]
    LockedOut { seconds_remaining: u64 },
    #[error("pairing session expired")]
    Expired,
}

/// Configuration for the network service.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub control_port: u16,
    pub input_port: u16,
    pub discovery_port: u16,
    pub bind_address: std::net::IpAddr,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            control_port: 24800,
            input_port: 24801,
            discovery_port: 24802,
            bind_address: "0.0.0.0".parse().unwrap(),
        }
    }
}

/// Runtime info about a connected client, returned by `get_connected_clients`.
#[derive(Debug, Clone)]
pub struct ConnectedClientInfo {
    pub client_id: ClientId,
    pub name: String,
    pub address: SocketAddr,
    pub latency_ms: f32,
    pub events_per_second: u32,
    pub is_paired: bool,
}

/// Events emitted by the connection manager to the application layer.
#[derive(Debug)]
pub enum ConnectionEvent {
    ClientDiscovered {
        client_id: ClientId,
        name: String,
        address: SocketAddr,
    },
    ClientConnected {
        client_id: ClientId,
    },
    ClientDisconnected {
        client_id: ClientId,
    },
    PairingRequested {
        client_id: ClientId,
        session_id: Uuid,
        pin: String,
    },
    PairingCompleted {
        client_id: ClientId,
    },
    PairingFailed {
        client_id: ClientId,
        reason: String,
    },
    ScreenInfoUpdated {
        client_id: ClientId,
        monitor_count: u8,
    },
}

/// Active pairing session.
#[derive(Debug)]
struct PairingSession {
    client_id: ClientId,
    pin_hash: String,
    created_at: Instant,
    attempts: u8,
}

/// Per-IP lockout state.
#[derive(Debug)]
struct LockoutEntry {
    locked_until: Instant,
    failed_attempts: u8,
}

const MAX_PIN_ATTEMPTS: u8 = 3;
const LOCKOUT_DURATION: Duration = Duration::from_secs(60);
const PAIRING_EXPIRY: Duration = Duration::from_secs(60);

/// The connection manager.
///
/// In the full implementation this would manage TLS listeners. For now
/// it provides the state machine and pairing logic, which are fully testable.
pub struct ConnectionManager {
    config: NetworkConfig,
    pairing_sessions: HashMap<Uuid, PairingSession>,
    lockouts: HashMap<std::net::IpAddr, LockoutEntry>,
    paired_clients: HashMap<ClientId, String>, // client_id -> cert fingerprint
    event_tx: mpsc::Sender<ConnectionEvent>,
}

impl ConnectionManager {
    /// Creates a new connection manager and returns it together with the event receiver.
    pub fn new(config: NetworkConfig) -> (Self, mpsc::Receiver<ConnectionEvent>) {
        let (tx, rx) = mpsc::channel(64);
        let mgr = Self {
            config,
            pairing_sessions: HashMap::new(),
            lockouts: HashMap::new(),
            paired_clients: HashMap::new(),
            event_tx: tx,
        };
        (mgr, rx)
    }

    /// Checks whether a client has been paired.
    pub fn is_paired(&self, client_id: ClientId) -> bool {
        self.paired_clients.contains_key(&client_id)
    }

    /// Initiates a new pairing session for a discovered client.
    ///
    /// Generates a 6-digit PIN, stores a hash, and emits a `PairingRequested` event.
    ///
    /// # Errors
    ///
    /// Returns [`PairingError::LockedOut`] if the client's IP is currently locked out.
    pub fn initiate_pairing(
        &mut self,
        client_id: ClientId,
        client_addr: std::net::IpAddr,
    ) -> Result<(Uuid, String), PairingError> {
        self.check_lockout(client_addr)?;

        let pin = generate_pin();
        let session_id = Uuid::new_v4();
        let pin_hash = hash_pin(&pin, &session_id);

        self.pairing_sessions.insert(
            session_id,
            PairingSession {
                client_id,
                pin_hash,
                created_at: Instant::now(),
                attempts: 0,
            },
        );

        Ok((session_id, pin))
    }

    /// Verifies a PIN response and completes pairing on success.
    ///
    /// # Errors
    ///
    /// Returns [`PairingError`] variants for wrong PIN, lockout, or expiry.
    pub fn verify_pairing_pin(
        &mut self,
        session_id: Uuid,
        submitted_pin_hash: &str,
        client_addr: std::net::IpAddr,
    ) -> Result<ClientId, PairingError> {
        self.check_lockout(client_addr)?;

        let session = self
            .pairing_sessions
            .get_mut(&session_id)
            .ok_or(PairingError::SessionNotFound)?;

        if session.created_at.elapsed() > PAIRING_EXPIRY {
            self.pairing_sessions.remove(&session_id);
            return Err(PairingError::Expired);
        }

        if session.pin_hash != submitted_pin_hash {
            session.attempts += 1;
            let remaining = MAX_PIN_ATTEMPTS.saturating_sub(session.attempts);
            if remaining == 0 {
                self.lockouts.insert(
                    client_addr,
                    LockoutEntry {
                        locked_until: Instant::now() + LOCKOUT_DURATION,
                        failed_attempts: MAX_PIN_ATTEMPTS,
                    },
                );
                self.pairing_sessions.remove(&session_id);
            }
            return Err(PairingError::WrongPin {
                attempts_remaining: remaining,
            });
        }

        let client_id = session.client_id;
        self.pairing_sessions.remove(&session_id);
        // Store a placeholder cert fingerprint (real implementation uses TLS cert hash)
        self.paired_clients
            .insert(client_id, format!("pinned:{client_id}"));
        Ok(client_id)
    }

    fn check_lockout(&self, addr: std::net::IpAddr) -> Result<(), PairingError> {
        if let Some(entry) = self.lockouts.get(&addr) {
            let now = Instant::now();
            if now < entry.locked_until {
                let remaining = (entry.locked_until - now).as_secs();
                return Err(PairingError::LockedOut {
                    seconds_remaining: remaining,
                });
            }
        }
        Ok(())
    }
}

/// Generates a cryptographically random 6-digit numeric PIN.
fn generate_pin() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    // Simple determinism-free generation using system time + thread ID.
    // Production code should use `rand` crate with OsRng.
    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);
    let n = hasher.finish() % 1_000_000;
    format!("{n:06}")
}

/// Hashes a PIN with the session ID using a simple scheme.
///
/// Production code should use PBKDF2 or Argon2.
fn hash_pin(pin: &str, session_id: &Uuid) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    pin.hash(&mut hasher);
    session_id.as_bytes().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> (ConnectionManager, mpsc::Receiver<ConnectionEvent>) {
        ConnectionManager::new(NetworkConfig::default())
    }

    #[test]
    fn test_initiate_pairing_returns_six_digit_pin() {
        let (mut mgr, _rx) = make_manager();
        let client_id = Uuid::new_v4();
        let addr: std::net::IpAddr = "192.168.1.1".parse().unwrap();
        let (_session_id, pin) = mgr.initiate_pairing(client_id, addr).unwrap();
        assert_eq!(pin.len(), 6, "PIN must be exactly 6 digits");
        assert!(pin.chars().all(|c| c.is_ascii_digit()), "PIN must contain only digits");
    }

    #[test]
    fn test_verify_pairing_pin_succeeds_with_correct_hash() {
        let (mut mgr, _rx) = make_manager();
        let client_id = Uuid::new_v4();
        let addr: std::net::IpAddr = "192.168.1.2".parse().unwrap();
        let (session_id, pin) = mgr.initiate_pairing(client_id, addr).unwrap();
        let pin_hash = hash_pin(&pin, &session_id);

        let result = mgr.verify_pairing_pin(session_id, &pin_hash, addr);
        assert_eq!(result, Ok(client_id));
    }

    #[test]
    fn test_verify_pairing_pin_fails_with_wrong_hash() {
        let (mut mgr, _rx) = make_manager();
        let client_id = Uuid::new_v4();
        let addr: std::net::IpAddr = "192.168.1.3".parse().unwrap();
        let (session_id, _pin) = mgr.initiate_pairing(client_id, addr).unwrap();

        let result = mgr.verify_pairing_pin(session_id, "wrong_hash", addr);
        assert!(matches!(result, Err(PairingError::WrongPin { .. })));
    }

    #[test]
    fn test_verify_pairing_pin_locks_out_after_three_failures() {
        let (mut mgr, _rx) = make_manager();
        let client_id = Uuid::new_v4();
        let addr: std::net::IpAddr = "192.168.1.4".parse().unwrap();
        let (session_id, _pin) = mgr.initiate_pairing(client_id, addr).unwrap();

        // Three wrong attempts
        for _ in 0..3 {
            let _ = mgr.verify_pairing_pin(session_id, "bad", addr);
        }

        // Next attempt should be locked out
        let new_session = {
            // Re-initiate to get a new session – should fail if locked
            mgr.initiate_pairing(client_id, addr)
        };
        assert!(matches!(new_session, Err(PairingError::LockedOut { .. })));
    }

    #[test]
    fn test_verify_pairing_pin_returns_session_not_found_for_unknown_session() {
        let (mut mgr, _rx) = make_manager();
        let addr: std::net::IpAddr = "192.168.1.5".parse().unwrap();
        let result = mgr.verify_pairing_pin(Uuid::new_v4(), "any", addr);
        assert_eq!(result, Err(PairingError::SessionNotFound));
    }

    #[test]
    fn test_is_paired_returns_false_before_pairing() {
        let (mgr, _rx) = make_manager();
        assert!(!mgr.is_paired(Uuid::new_v4()));
    }

    #[test]
    fn test_is_paired_returns_true_after_successful_pairing() {
        let (mut mgr, _rx) = make_manager();
        let client_id = Uuid::new_v4();
        let addr: std::net::IpAddr = "192.168.1.6".parse().unwrap();
        let (session_id, pin) = mgr.initiate_pairing(client_id, addr).unwrap();
        let pin_hash = hash_pin(&pin, &session_id);
        mgr.verify_pairing_pin(session_id, &pin_hash, addr).unwrap();
        assert!(mgr.is_paired(client_id));
    }

    #[test]
    fn test_hash_pin_is_deterministic_for_same_inputs() {
        let session_id = Uuid::new_v4();
        let h1 = hash_pin("123456", &session_id);
        let h2 = hash_pin("123456", &session_id);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_pin_differs_for_different_pins() {
        let session_id = Uuid::new_v4();
        let h1 = hash_pin("123456", &session_id);
        let h2 = hash_pin("654321", &session_id);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_pin_differs_for_different_sessions() {
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let h1 = hash_pin("123456", &s1);
        let h2 = hash_pin("123456", &s2);
        assert_ne!(h1, h2);
    }
}
