//! Clock adapter trait and implementations.

use std::time::SystemTime;

/// Abstraction over wall-clock time.
///
/// Injected wherever zenith-session needs the current time, so tests can
/// substitute a fixed value without relying on the real system clock.
pub trait Clock {
    fn now(&self) -> SystemTime;
}

/// Real clock: delegates to [`SystemTime::now`].
pub struct OsClock;

impl Clock for OsClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

/// Deterministic test clock: always returns the fixed [`SystemTime`] it was
/// constructed with.
pub struct FakeClock(pub SystemTime);

impl Clock for FakeClock {
    fn now(&self) -> SystemTime {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_clock_returns_fixed_value() {
        let fixed = SystemTime::UNIX_EPOCH
            .checked_add(std::time::Duration::from_secs(1_000_000))
            .unwrap();
        let clock = FakeClock(fixed);
        assert_eq!(clock.now(), fixed);
        // Calling twice returns the same value.
        assert_eq!(clock.now(), fixed);
    }
}
