//! Random-number-generator adapter trait and deterministic test fake.
//!
//! The OS-backed implementation is intentionally absent from this unit.  It
//! will arrive alongside ULID minting in a later unit, at which point a real
//! `OsRng` wrapper around `getrandom` (or equivalent) will be added here.

/// Abstraction over a random-byte source.
///
/// Callers receive `&impl Rng` so that production code can eventually use a
/// real OS source while tests substitute [`FakeRng`] for full determinism.
///
/// # Note on the OS implementation
///
/// `OsRng` is deferred to the ULID-minting unit.  Do not add a panicking
/// placeholder — the trait exists now so call-sites can be written against it.
pub trait Rng {
    /// Fill `buf` with random (or deterministic, in fakes) bytes.
    fn fill_bytes(&self, buf: &mut [u8]);
}

/// Deterministic test RNG: fills every byte of `buf` with `self.0`.
///
/// Useful for asserting exact byte sequences in unit tests without needing a
/// real entropy source.
pub struct FakeRng(pub u8);

impl Rng for FakeRng {
    fn fill_bytes(&self, buf: &mut [u8]) {
        buf.fill(self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_rng_fills_all_bytes_with_its_value() {
        let rng = FakeRng(0xAB);
        let mut buf = [0u8; 8];
        rng.fill_bytes(&mut buf);
        assert_eq!(buf, [0xAB; 8]);
    }

    #[test]
    fn fake_rng_zero_value() {
        let rng = FakeRng(0x00);
        let mut buf = [0xFFu8; 4];
        rng.fill_bytes(&mut buf);
        assert_eq!(buf, [0x00; 4]);
    }
}
