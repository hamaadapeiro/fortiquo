//! libp2p-oriented message framing for Fortiquo (wire payloads use [`postcard`]).
//!
//! Higher-level swarm setup lives in the node crate; this module defines the canonical
//! [`NetworkMessage`] types peers exchange.

use fortiquo_types::{Block, PohEntry, SignedTransaction};
use serde::{Deserialize, Serialize};

/// Gossip and request/response payloads for the Fortiquo P2P layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Propagate a newly seen signed transaction.
    NewTransaction(SignedTransaction),
    /// Propagate a newly produced block.
    NewBlock(Block),
    /// Request a block by height.
    RequestBlock {
        /// Block height (number).
        number: u64,
    },
    /// Reply to [`NetworkMessage::RequestBlock`].
    ResponseBlock(Block),
    /// Request a contiguous PoH tick range.
    RequestPohEntries {
        /// First tick (inclusive).
        start_tick: u64,
        /// Last tick (inclusive).
        end_tick: u64,
    },
    /// Reply to [`NetworkMessage::RequestPohEntries`].
    ResponsePohEntries(Vec<PohEntry>),
}

impl NetworkMessage {
    /// Serialize to a compact binary blob (BLAKE3 is applied only by higher layers if needed).
    pub fn encode(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }

    /// Decode from [`NetworkMessage::encode`] output.
    pub fn decode(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_message_roundtrip_postcard() {
        // arrange
        let msg = NetworkMessage::RequestBlock { number: 42 };

        // act
        let bytes = msg.encode().unwrap();
        let got = NetworkMessage::decode(&bytes).unwrap();

        // assert
        assert!(matches!(got, NetworkMessage::RequestBlock { number: 42 }));
    }
}
