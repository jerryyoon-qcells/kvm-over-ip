//! KVM-Over-IP Master Application entry point.
//!
//! Wires together all infrastructure services and starts the Tokio async runtime.
//! The Tauri application is created here; Tauri commands are registered and
//! routed to the [`AppState`] via the `infrastructure::ui_bridge` module.
//!
//! # Architecture
//!
//! ```text
//! main()
//!  └─ AppState::new()       -- loads config, creates registries
//!  └─ start services
//!       ├─ InputCaptureService (Windows hook thread)
//!       ├─ DiscoveryResponder  (UDP background thread)
//!       └─ RouteInputUseCase   (Tokio task)
//! ```

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use kvm_master::application::manage_clients::{ClientRuntimeState, ConnectionState};
use kvm_master::infrastructure;
use kvm_master::infrastructure::ui_bridge::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise structured logging.  Level is overridden by `RUST_LOG`.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("KVM-Over-IP Master starting");

    // Load configuration and initialise shared state.
    let state = AppState::new();

    // Shutdown flag shared across all background services.
    let running = Arc::new(AtomicBool::new(true));

    // ── Discovery responder ────────────────────────────────────────────────────
    let discovery_port = {
        let cfg = state.config.lock().await;
        cfg.network.discovery_port
    };

    let discovery_rx = match infrastructure::network::discovery::start_discovery_responder(
        discovery_port,
        Arc::clone(&running),
    ) {
        Ok(rx) => {
            info!("discovery responder started on UDP {discovery_port}");
            Some(rx)
        }
        Err(e) => {
            error!("failed to start discovery responder: {e}");
            None
        }
    };

    // ── Discovery event pump ──────────────────────────────────────────────────
    if let Some(mut rx) = discovery_rx {
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                info!(
                    "discovered client: {} ({})",
                    event.name, event.client_id
                );
                let mut registry = state_clone.client_registry.lock().await;
                registry.upsert(ClientRuntimeState {
                    id: event.client_id,
                    name: event.name,
                    connection_state: ConnectionState::Discovered,
                    latency_ms: 0.0,
                    events_per_second: 0,
                });
            }
        });
    }

    // ── Ctrl-C / SIGTERM handler ──────────────────────────────────────────────
    let running_clone = Arc::clone(&running);
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("shutdown signal received");
            running_clone.store(false, Ordering::Relaxed);
        }
    });

    info!("KVM-Over-IP Master ready.  Press Ctrl-C to exit.");

    // In a full Tauri build, `tauri::Builder::default()` would be invoked here
    // to open the native window and register commands.  For the CLI/headless
    // variant we simply block until the shutdown flag is cleared.
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if !running.load(Ordering::Relaxed) {
            break;
        }
    }

    info!("KVM-Over-IP Master stopped");
    Ok(())
}
