use std::time::Duration;

/// Timeout for HTTP request to a peer datacenter.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Response is `list<Instance>
///
/// ## Based on size
///
/// Instance = 250 bytes for an avg key size (bare minimum 115 bytes)
///
/// Reasonable total download size = 64 MiB
///
/// That's ~268k instances
///
/// ## Based on FDB limitation
///
/// FDB starts having performance issues between 10k-100k keys in a single range read based on
/// rudimentary tests.
///
/// Can likely parallelize FDB transactions to return larger chunks.
///
/// ## Main limitation
///
/// FDB is the main limitation here, so we opt to use an FDB-friendly chunk size.
///
/// ## Estimated download time
///
/// Our current impl is incredily slow since we do not parallelize downloads.
///
/// Assume we have 10M keys with a 50k chunk size. This results in 200 round trips.
///
/// If we have a latency of 200 ms (worst case), this takes 40 seconds total to propagate.
pub const DOWNLOAD_INSTANCE_COUNT: u64 = 50_000;

/// Number of keys to recover in a single chunk during the recovery process.
///
/// Keys are recovered in chunks to avoid memory issues and provide progress updates.
/// The recovery process scans all key-instance markers and rebuilds the committed values.
///
/// This number is relatively small since we have to do a lot of operations per key.
pub const RECOVER_KEY_CHUNK_SIZE: u64 = 500;
