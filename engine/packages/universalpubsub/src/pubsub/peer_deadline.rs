//! A peer receives a request budget in milliseconds and starts its own monotonic deadline.

use anyhow::Result;
use rivet_env::Clocks;

use crate::errors;

pub(super) fn remaining_ms(deadline_boot_ms: u64, sender: Clocks) -> u64 {
	deadline_boot_ms.saturating_sub(sender.boot_ms)
}

pub(super) fn local_deadline_ms(remaining_ms: u64, receiver: Clocks) -> Result<u64> {
	if remaining_ms == 0 {
		return Err(errors::Timeout.build());
	}
	Ok(receiver
		.boot_ms
		.checked_add(remaining_ms)
		.ok_or(rivet_env::ClockError::Overflow)?)
}


