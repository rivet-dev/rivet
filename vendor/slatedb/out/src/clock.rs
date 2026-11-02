//! This module contains utility methods and structs for handling time.
//!
//! SlateDB has two concepts of time:
//!
//! 1. The [SystemClock], which is used to measure wall-clock time for things
//!    like garbage collection schedule ticks, compaction schedule ticks, and so
//!    on.
//! 2. The [LogicalClock], which is a monotonically increasing number used to order
//!    writes in the database. This could represent a logical sequence number
//!    (LSN) from a database, a Kafka offset, a `created_at` timestamp
//!    associated with the write, and so on.
//!
//! The [SystemClock] struct is provided in `slatedb-common`. The `slatedb` package
//! builds on top of that and provides the [LogicalClock] abstraction.
//! [DefaultLogicalClock] implements a logical clock that wraps the
//! [DefaultSystemClock] and returns the number of milliseconds since the Unix epoch.

#![allow(clippy::disallowed_methods)]

use crate::error::SlateDBError;
use log::debug;
use slatedb_common::clock::SystemClock;
use std::{
    cmp,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
    time::Duration,
};

/// SlateDB uses MonotonicClock internally so that it can enforce that clock ticks
/// from the underlying implementation are monotonically increasing.
#[allow(dead_code)] // unused during DST
pub(crate) struct MonotonicClock {
    pub(crate) last_tick: AtomicI64,
    delegate: Arc<dyn SystemClock>,
}

impl MonotonicClock {
    pub(crate) fn new(delegate: Arc<dyn SystemClock>, init_tick: i64) -> Self {
        Self {
            delegate,
            last_tick: AtomicI64::new(init_tick),
        }
    }

    pub(crate) fn set_last_tick(&self, tick: i64) -> Result<i64, SlateDBError> {
        self.enforce_monotonic(tick)
    }

    #[cfg_attr(dst, allow(dead_code))]
    pub(crate) async fn now(&self) -> Result<i64, SlateDBError> {
        let tick = self.delegate.now().timestamp_millis();
        match self.enforce_monotonic(tick) {
            Err(SlateDBError::InvalidClockTick {
                last_tick,
                next_tick: _,
            }) => {
                let sync_millis = cmp::min(10_000, 2 * (last_tick - tick).unsigned_abs());
                debug!(
                    "Clock tick {} is lagging behind the last known tick {}. \
                    Sleeping {}ms to potentially resolve skew before returning InvalidClockTick.",
                    tick, last_tick, sync_millis
                );
                tokio::time::sleep(Duration::from_millis(sync_millis)).await;
                self.enforce_monotonic(self.delegate.now().timestamp_millis())
            }
            result => result,
        }
    }

    fn enforce_monotonic(&self, tick: i64) -> Result<i64, SlateDBError> {
        let updated_last_tick = self.last_tick.fetch_max(tick, Ordering::SeqCst);
        if tick < updated_last_tick {
            return Err(SlateDBError::InvalidClockTick {
                last_tick: updated_last_tick,
                next_tick: tick,
            });
        }

        Ok(tick)
    }
}
