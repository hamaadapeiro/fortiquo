use crate::error::CryptoError;
use fortiquo_types::{Address, PublicKeyBytes};

/// Trait for deriving addresses from public keys.
pub trait AddressDeriver: Send + Sync {
    /// Derive an address from a public key.
    fn derive_address(&self, public_key: &PublicKeyBytes) -> Result<Address, CryptoError>;
}

/// Default address deriver: first 20 bytes of BLAKE3(public_key).
#[derive(Clone, Copy, Debug)]
pub struct Blake3AddressDeriver;

impl AddressDeriver for Blake3AddressDeriver {
    fn derive_address(&self, public_key: &PublicKeyBytes) -> Result<Address, CryptoError> {
        let hash = blake3::hash(public_key.as_slice());
        let bytes = hash.as_bytes();
        let mut addr_bytes = [0u8; 20];
        addr_bytes.copy_from_slice(&bytes[0..20]);
        Ok(Address::new(addr_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_address_deriver() {
        let deriver = Blake3AddressDeriver;
        let pubkey = PublicKeyBytes::new(vec![42u8; 1184]);
        let addr = deriver.derive_address(&pubkey).unwrap();

        // Same pubkey should produce same address (deterministic)
        let addr2 = deriver.derive_address(&pubkey).unwrap();
        assert_eq!(addr, addr2);
    }

    #[test]
    fn test_different_pubkeys_different_addresses() {
        let deriver = Blake3AddressDeriver;
        let pubkey1 = PublicKeyBytes::new(vec![1u8; 1184]);
        let pubkey2 = PublicKeyBytes::new(vec![2u8; 1184]);

        let addr1 = deriver.derive_address(&pubkey1).unwrap();
        let addr2 = deriver.derive_address(&pubkey2).unwrap();

        assert_ne!(addr1, addr2);
    }

    #[test]
    fn test_address_is_20_bytes() {
        let deriver = Blake3AddressDeriver;
        let pubkey = PublicKeyBytes::new(vec![99u8; 1184]);
        let addr = deriver.derive_address(&pubkey).unwrap();

        assert_eq!(addr.as_bytes().len(), 20);
    }
}
