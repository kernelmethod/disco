//! This module serves as a thin layer over rand_chacha, which hides
//! some of the details of how random number generation is implemented
//! and provides some additional functionality (e.g. fast key erasure).

use rand::prelude::*;
use rand_chacha::ChaCha20Rng;

type Buffer = [u8; 1024];

type CRNGKey = [u8; 32];
type CRNGOutput = [u8; core::mem::size_of::<Buffer>() - core::mem::size_of::<CRNGKey>()];

const KEY_LEN: usize = core::mem::size_of::<CRNGKey>();
const OUTPUT_LEN: usize = core::mem::size_of::<CRNGOutput>();

pub struct CRNG {
    buffer: Buffer,
    rng: ChaCha20Rng,
}

unsafe fn slice_to_array<const N: usize>(slice: &[u8]) -> &[u8; N] {
    &*(slice.as_ptr() as *const [u8; N])
}

impl CRNG {
    pub fn new() -> Self {
        CRNG {
            buffer: [0u8; core::mem::size_of::<Buffer>()],
            rng: ChaCha20Rng::from_entropy(),
        }
    }

    fn key_slice(&self) -> &[u8] {
        &self.buffer[..core::mem::size_of::<CRNGKey>()]
    }

    fn output_slice(&self) -> &[u8] {
        &self.buffer[core::mem::size_of::<CRNGKey>()..]
    }

    pub fn output(&self) -> &CRNGOutput {
        unsafe { slice_to_array::<OUTPUT_LEN>(self.output_slice()) }
    }

    pub fn regenerate(&mut self) -> &CRNGOutput {
        self.rng.fill(&mut self.buffer);

        // Replace the current key of the internal RNG
        let key = unsafe { slice_to_array::<KEY_LEN>(self.key_slice()) };
        self.rng = ChaCha20Rng::from_seed(*key);

        self.output()
    }
}

#[cfg(test)]
mod test {
    use super::CRNG;

    /// Check that we are erasing the key every time we regenerate the
    /// output of the CRNG.
    #[test]
    fn test_fke() {
        let mut crng = CRNG::new();
        let key1 = crng.rng.get_seed();

        crng.regenerate();
        let key2 = crng.rng.get_seed();

        assert!(key1 != key2);
    }
}
