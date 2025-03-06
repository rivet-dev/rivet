use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::Instant;
use std::sync::OnceLock;

/// An atomic wrapper around Instant that stores the time as a duration since a reference point
#[derive(Debug)]
pub struct AtomicInstant {
    value: AtomicU64,
}

impl AtomicInstant {
    /// Creates a new AtomicInstant set to the current time
    pub fn new() -> Self {
        Self {
            value: AtomicU64::new(Self::now_as_secs()),
        }
    }

    /// Updates the stored time to the current instant
    pub fn store_now(&self) {
        self.value.store(Self::now_as_secs(), Ordering::SeqCst);
    }

    /// Loads the stored instant
    pub fn load(&self) -> Instant {
        let secs = self.value.load(Ordering::SeqCst);
        // We use a fixed reference point (program start) to ensure consistent durations
        static START: OnceLock<Instant> = OnceLock::new();
        let start = START.get_or_init(Instant::now);
        *start + Duration::from_secs(secs)
    }

    /// Helper to get current time as seconds since program start
    fn now_as_secs() -> u64 {
        let now = Instant::now();
        static START: OnceLock<Instant> = OnceLock::new();
        let start = START.get_or_init(Instant::now);
        now.duration_since(*start).as_secs()
    }
}
