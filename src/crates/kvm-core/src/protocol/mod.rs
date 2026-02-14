//! Protocol module containing message types and the binary codec.
//!
//! # How the protocol works (for beginners)
//!
//! Every piece of information sent between the master and a client travels as a
//! **KVM message**.  A message consists of two parts:
//!
//! 1. **Header** (24 bytes, always the same structure)
//!    Contains the protocol version, message type code, payload length,
//!    a sequence number (monotonically increasing counter), and a timestamp.
//!
//! 2. **Payload** (variable length)
//!    The actual data for that message type.  For example, a `KeyEvent` payload
//!    carries which key was pressed, whether it was pressed or released, and
//!    which modifier keys were held at the time.
//!
//! The `codec` sub-module provides two public functions that do all the work:
//! - `encode_message` – takes a `KvmMessage` value and returns a `Vec<u8>`
//! - `decode_message` – takes a `&[u8]` and returns the typed `KvmMessage`
//!
//! # Sub-modules
//!
//! - **`messages`** – All message type definitions (enums, structs).
//! - **`codec`**    – Binary encoding and decoding logic.
//! - **`sequence`** – Thread-safe incrementing counter for sequence numbers.

// Declare the sub-modules.  Rust compiles these from separate source files.
pub mod codec;
pub mod messages;
pub mod sequence;

// Re-export the most commonly needed items at the protocol module level,
// so callers can write `kvm_core::protocol::encode_message` instead of
// `kvm_core::protocol::codec::encode_message`.
pub use codec::{decode_message, encode_message, ProtocolError};
pub use messages::*;
pub use sequence::SequenceCounter;
