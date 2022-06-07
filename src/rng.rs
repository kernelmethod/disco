//! This module serves as a thin layer over rand_chacha, which hides
//! some of the details of how random number generation is implemented
//! and provides some additional functionality (e.g. fast key erasure).

use crate::error::{ErrorKind, Result};
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20::ChaCha20;

const BUFFER_LEN: usize = 1024;
const KEY_LEN: usize = 32;
const OUTPUT_LEN: usize = BUFFER_LEN - KEY_LEN;

type Buffer = [u8; BUFFER_LEN];
type Key = [u8; KEY_LEN];
type Output = [u8; OUTPUT_LEN];

/// Convert a slice to a fixed-length array.
unsafe fn slice_to_array<const N: usize>(slice: &[u8]) -> &[u8; N] {
    &*(slice.as_ptr() as *const [u8; N])
}

pub struct CryptoRng {
    buffer: Buffer,
    nonce: [u8; 12],
}

impl CryptoRng {
    pub fn from_entropy() -> Result<Self> {
        let buffer = [0u8; BUFFER_LEN];
        let nonce = [0u8; 12];
        let mut crng = CryptoRng { buffer, nonce };

        // Initialize the key for the CryptoRng using the operating system's
        // random stream.
        let key_slice = crng.key_slice();

        match getrandom::getrandom(&mut key_slice[..]) {
            Err(e) => Err(ErrorKind::GetRandomError(e)),
            Ok(_) => Ok(crng),
        }
    }

    fn key_slice(&mut self) -> &mut [u8] {
        &mut self.buffer[..KEY_LEN]
    }

    fn output_slice(&self) -> &[u8] {
        &self.buffer[KEY_LEN..]
    }

    pub fn key(&mut self) -> &Key {
        unsafe { slice_to_array::<KEY_LEN>(self.key_slice()) }
    }

    pub fn output(&self) -> &Output {
        unsafe { slice_to_array::<OUTPUT_LEN>(self.output_slice()) }
    }

    pub fn regenerate(&mut self) -> &Output {
        let nonce = self.nonce;
        let mut cipher = ChaCha20::new(self.key().into(), &nonce.into());
        cipher.apply_keystream(&mut self.buffer);
        self.output()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Check that CryptoRng::from_entropy() creates a key that is filled
    /// with random data.
    #[test]
    fn test_from_entropy() {
        let mut crng = CryptoRng::from_entropy().unwrap();
        let key = crng.key().clone();
        let zeros = [0u8; KEY_LEN];

        assert_eq!(zeros.len(), key.len());
        assert!(key != zeros);
    }

    /// Check that we are erasing the key every time we regenerate the
    /// output of the CryptoRng.
    #[test]
    fn test_fke() {
        let mut crng = CryptoRng::from_entropy().unwrap();
        let key1 = crng.key().clone();

        crng.regenerate();
        let key2 = crng.key().clone();

        assert!(key1 != key2);
    }
}
