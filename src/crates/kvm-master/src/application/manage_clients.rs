//! ManageClientsUseCase: client registry and pairing state management.
//!
//! The `ClientRegistry` is the master's in-memory database of every KVM client
//! it has discovered or previously connected to.  Each entry tracks:
//!
//! - The client's UUID, name, and current `ConnectionState`.
//! - Performance metrics (latency, events per second) updated in real-time.
//!
//! # Connection lifecycle (for beginners)
//!
//! Clients progress through these states:
//!
//! ```text
//! Discovered  ──►  Connecting  ──►  Pairing  ──►  Paired  ──►  Connected
//!                                                                  │
//!                                                           Disconnected
//! ```
//!
//! - `Discovered`: the master received a UDP `AnnounceMessage` from this client.
//! - `Connecting`: the master is establishing a TCP control channel.
//! - `Pairing`: a PIN exchange is in progress.
//! - `Paired`: PIN verified; the relationship is stored.
//! - `Connected`: the TCP channel is open and input events are flowing.
//! - `Disconnected`: the TCP channel closed; the entry is kept for reconnection.

use kvm_core::ClientId;
use std::collections::HashMap;

/// Current state of a client connection.
///
/// This enum drives the UI colour coding in `ClientList.tsx`:
/// - Green = `Connected` or `Paired`
/// - Yellow = `Discovered`, `Connecting`, or `Pairing`
/// - Grey = `Disconnected`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// UDP `Announce` received; no TCP connection yet.
    Discovered,
    /// TCP handshake in progress.
    Connecting,
    /// TCP channel open; input events flowing.
    Connected,
    /// PIN exchange in progress.
    Pairing,
    /// PIN exchange complete; relationship stored on disk.
    Paired,
    /// TCP channel closed.
    Disconnected,
}

/// Runtime state for a client tracked by the master.
#[derive(Debug, Clone)]
pub struct ClientRuntimeState {
    pub id: ClientId,
    pub name: String,
    pub connection_state: ConnectionState,
    pub latency_ms: f32,
    pub events_per_second: u32,
}

/// In-memory registry of all known clients.
///
/// The registry is stored behind a `Mutex` in `AppState` so it can be shared
/// between the Tokio async tasks (discovery pump, Tauri commands).
///
/// # HashMap choice
///
/// A `HashMap<ClientId, ClientRuntimeState>` provides O(1) lookup by UUID.
/// Iteration order is not guaranteed but that is fine — the UI sorts the
/// list alphabetically before displaying it.
#[derive(Default)]
pub struct ClientRegistry {
    clients: HashMap<ClientId, ClientRuntimeState>,
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers or updates a client.
    pub fn upsert(&mut self, state: ClientRuntimeState) {
        self.clients.insert(state.id, state);
    }

    /// Returns a snapshot of all clients.
    pub fn all(&self) -> Vec<ClientRuntimeState> {
        self.clients.values().cloned().collect()
    }

    /// Returns the state for a specific client.
    pub fn get(&self, id: ClientId) -> Option<&ClientRuntimeState> {
        self.clients.get(&id)
    }

    /// Updates connection state for a specific client.
    pub fn set_state(&mut self, id: ClientId, state: ConnectionState) {
        if let Some(client) = self.clients.get_mut(&id) {
            client.connection_state = state;
        }
    }

    /// Removes a client from the registry.
    pub fn remove(&mut self, id: ClientId) {
        self.clients.remove(&id);
    }

    /// Updates the rolling latency average for a client.
    pub fn update_latency(&mut self, id: ClientId, latency_ms: f32) {
        if let Some(client) = self.clients.get_mut(&id) {
            client.latency_ms = latency_ms;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client(name: &str) -> ClientRuntimeState {
        ClientRuntimeState {
            id: Uuid::new_v4(),
            name: name.to_string(),
            connection_state: ConnectionState::Discovered,
            latency_ms: 0.0,
            events_per_second: 0,
        }
    }

    #[test]
    fn test_registry_starts_empty() {
        let registry = ClientRegistry::new();
        assert!(registry.all().is_empty());
    }

    #[test]
    fn test_upsert_adds_client() {
        let mut registry = ClientRegistry::new();
        let client = make_client("dev-linux");
        let id = client.id;
        registry.upsert(client);
        assert!(registry.get(id).is_some());
    }

    #[test]
    fn test_upsert_updates_existing_client() {
        let mut registry = ClientRegistry::new();
        let client = make_client("dev-linux");
        let id = client.id;
        registry.upsert(client);

        let updated = ClientRuntimeState {
            id,
            name: "dev-linux-updated".to_string(),
            connection_state: ConnectionState::Connected,
            latency_ms: 2.5,
            events_per_second: 100,
        };
        registry.upsert(updated);

        let state = registry.get(id).unwrap();
        assert_eq!(state.name, "dev-linux-updated");
        assert_eq!(state.connection_state, ConnectionState::Connected);
    }

    #[test]
    fn test_set_state_updates_connection_state() {
        let mut registry = ClientRegistry::new();
        let client = make_client("test");
        let id = client.id;
        registry.upsert(client);
        registry.set_state(id, ConnectionState::Connected);
        assert_eq!(
            registry.get(id).unwrap().connection_state,
            ConnectionState::Connected
        );
    }

    #[test]
    fn test_remove_deletes_client() {
        let mut registry = ClientRegistry::new();
        let client = make_client("test");
        let id = client.id;
        registry.upsert(client);
        registry.remove(id);
        assert!(registry.get(id).is_none());
    }

    #[test]
    fn test_update_latency_changes_latency_value() {
        let mut registry = ClientRegistry::new();
        let client = make_client("test");
        let id = client.id;
        registry.upsert(client);
        registry.update_latency(id, 3.7);
        assert!((registry.get(id).unwrap().latency_ms - 3.7).abs() < f32::EPSILON);
    }
}
