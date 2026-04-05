use std::time::Duration;

/// Timeout for HTTP request to a peer datacenter.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Number of changelog entries to fetch in a single catch-up page.
///
/// This keeps learner range reads bounded while still making steady progress through the
/// immutable per-key commit history.
pub const CHANGELOG_READ_COUNT: u64 = 5_000;

