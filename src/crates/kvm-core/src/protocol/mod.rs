//! Protocol module containing message types and the binary codec.

pub mod codec;
pub mod messages;
pub mod sequence;

pub use codec::{decode_message, encode_message, ProtocolError};
pub use messages::*;
pub use sequence::SequenceCounter;
