//! KVM-Over-IP Client Application entry point.
//!
//! Wires together the network connection, input emulation service, screen
//! reporter, and UI bridge, then runs the Tokio async event loop.
//!
//! # Architecture
//!
//! ```text
//! main()
//!  └─ ClientAppState::new()    -- initialises shared state
//!  └─ ClientConnection::start() -- TCP reconnect loop
//!  └─ message dispatch loop
//!       ├─ KeyEvent / MouseMove / etc.  -> EmulateInputUseCase
//!       ├─ ScreenInfoAck                -> re-enumerate monitors
//!       └─ Disconnect                   -> reconnect
//! ```
//!
//! # Message dispatch loop (for beginners)
//!
//! The `while let Some(event) = network_rx.recv().await` loop is the heart
//! of the client.  It processes one network event at a time:
//!
//! - `NetworkEvent::Connected` – update the UI status to "Connected".
//! - `NetworkEvent::Disconnected` – update status to "Disconnected";
//!   the `ClientConnection` handles automatic reconnection.
//! - `NetworkEvent::MessageReceived(msg)` – route the message to the
//!   appropriate handler (key emulation, mouse emulation, etc.).
//!
//! # Platform input emulator
//!
//! The `MockInputEmulator` used here records all injected events rather than
//! actually synthesising OS input.  In a production build it is replaced by:
//! - `WindowsInputEmulator` (calls `SendInput` Win32 API)
//! - `LinuxXTestEmulator`   (calls XTest extension)
//! - `MacosInputEmulator`   (calls CoreGraphics framework)

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use kvm_client::application::emulate_input::EmulateInputUseCase;
use kvm_client::infrastructure::{
    input_emulation::mock::MockInputEmulator,
    network::{ClientConnection, ClientConnectionConfig, NetworkEvent},
    screen_info::{build_screen_info, MockScreenEnumerator},
    ui_bridge::{ClientAppState, ClientConnectionStatus},
};
use kvm_core::protocol::messages::{InputEvent, KvmMessage};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("KVM-Over-IP Client starting");

    // Shared UI state.
    let app_state = ClientAppState::new();

    // Shutdown flag.
    let running = Arc::new(AtomicBool::new(true));

    // ── Platform input emulator ───────────────────────────────────────────────
    // In production: replace MockInputEmulator with WindowsInputEmulator,
    // LinuxXTestEmulator, or MacosInputEmulator based on compile target.
    let emulator = Arc::new(MockInputEmulator::new());
    // EmulateInputUseCase has mutable methods (dedup filter), so we wrap in Mutex.
    let emulate_use_case = Arc::new(tokio::sync::Mutex::new(EmulateInputUseCase::new(emulator)));

    // ── Network connection ────────────────────────────────────────────────────
    let client_id = Uuid::new_v4();
    let master_addr = {
        let guard = app_state.master_address.lock().await;
        if guard.is_empty() {
            "127.0.0.1:24800".to_string()
        } else {
            guard.clone()
        }
    };

    let net_cfg = ClientConnectionConfig {
        master_addr: master_addr
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:24800".parse().unwrap()),
        client_id,
        client_name: app_state.client_name.lock().await.clone(),
        ..Default::default()
    };

    let connection = Arc::new(ClientConnection::new(net_cfg));
    let mut network_rx = connection.clone().start(Arc::clone(&running)).await;

    // ── Initial screen report ─────────────────────────────────────────────────
    {
        let enumerator = MockScreenEnumerator::single_1080p();
        if let Ok(screen_info) = build_screen_info(&enumerator) {
            let count = screen_info.monitors.len() as u8;
            {
                let mut guard = app_state.monitor_count.lock().await;
                *guard = count;
            }
            connection.send_screen_info(screen_info).await;
        }
    }

    // ── Ctrl-C handler ────────────────────────────────────────────────────────
    let running_clone = Arc::clone(&running);
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("shutdown signal received");
            running_clone.store(false, Ordering::Relaxed);
        }
    });

    // ── Main message dispatch loop ────────────────────────────────────────────
    info!("KVM-Over-IP Client ready. Connecting to master…");

    while let Some(event) = network_rx.recv().await {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        match event {
            NetworkEvent::Connected { master_addr } => {
                info!("control channel connected to {master_addr}");
                let mut status = app_state.connection_status.lock().await;
                *status = ClientConnectionStatus::Connected;
            }

            NetworkEvent::Disconnected => {
                warn!("control channel disconnected; reconnect in progress");
                let mut status = app_state.connection_status.lock().await;
                *status = ClientConnectionStatus::Disconnected;
            }

            NetworkEvent::MessageReceived(msg) => match msg {
                KvmMessage::KeyEvent(key_event) => {
                    let uc = emulate_use_case.lock().await;
                    if let Err(e) = uc.handle_key_event(&key_event) {
                        error!("key emulation error: {e}");
                    }
                }
                KvmMessage::MouseMove(mouse_move) => {
                    let mut uc = emulate_use_case.lock().await;
                    if let Err(e) = uc.handle_mouse_move(&mouse_move) {
                        error!("mouse move emulation error: {e}");
                    }
                }
                KvmMessage::MouseButton(mouse_btn) => {
                    let uc = emulate_use_case.lock().await;
                    if let Err(e) = uc.handle_mouse_button(&mouse_btn) {
                        error!("mouse button emulation error: {e}");
                    }
                }
                KvmMessage::MouseScroll(scroll) => {
                    let uc = emulate_use_case.lock().await;
                    if let Err(e) = uc.handle_mouse_scroll(&scroll) {
                        error!("mouse scroll emulation error: {e}");
                    }
                }
                KvmMessage::InputBatch(batch) => {
                    // Dispatch each event in the batch individually.
                    let mut uc = emulate_use_case.lock().await;
                    for event in &batch {
                        let result = match event {
                            InputEvent::Key(k) => uc.handle_key_event(k),
                            InputEvent::MouseMove(m) => uc.handle_mouse_move(m),
                            InputEvent::MouseButton(b) => uc.handle_mouse_button(b),
                            InputEvent::MouseScroll(s) => uc.handle_mouse_scroll(s),
                        };
                        if let Err(e) = result {
                            error!("input batch event error: {e}");
                        }
                    }
                }
                KvmMessage::HelloAck(ack) => {
                    if ack.accepted {
                        info!("master accepted connection");
                        let mut status = app_state.connection_status.lock().await;
                        *status = ClientConnectionStatus::Active;
                    } else {
                        warn!("master rejected connection (reason code {})", ack.reject_reason);
                    }
                }
                KvmMessage::PairingRequest(req) => {
                    info!("pairing requested (session {}); PIN display not yet implemented", req.pairing_session_id);
                    let mut status = app_state.connection_status.lock().await;
                    *status = ClientConnectionStatus::Pairing;
                }
                KvmMessage::Disconnect { reason } => {
                    info!("master sent disconnect: {reason:?}");
                    break;
                }
                KvmMessage::Ping(_) => { /* handled by ClientConnection::read_loop */ }
                _ => {}
            },
        }
    }

    info!("KVM-Over-IP Client stopped");
    Ok(())
}
