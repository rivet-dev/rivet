use std::{
	sync::atomic::{AtomicU16, AtomicU64, Ordering},
	time::{SystemTime, UNIX_EPOCH},
};

use crate::tuple::{TuplePack, Versionstamp, pack_with_versionstamp};

static TRANSACTION_COUNTER: AtomicU16 = AtomicU16::new(0);
static LAST_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

pub fn generate_versionstamp(user_version: u16) -> Versionstamp {
	// HACK: Using SystemTime::now() for versionstamp generation is problematic because:
	// (a) System time can go backwards due to NTP adjustments, daylight savings, etc.
	// (b) System time is not synchronized across machines, so versionstamps generated
	//     on different machines may not be correctly ordered
	//
	// This implementation tries to mitigate issue (a) by using max(current_time, last_time)
	// but cannot solve issue (b)
	let current_timestamp = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap()
		.as_micros() as u64;

	// Check if we've moved to a new microsecond to reset the transaction counter
	//
	// NOTE: Time can go backwards, so we use the max of current and last timestamp
	let last_ts = LAST_TIMESTAMP.load(Ordering::Acquire);
	let timestamp = current_timestamp.max(last_ts);

	if timestamp > last_ts {
		// Reset counter for new microsecond
		LAST_TIMESTAMP.store(timestamp, Ordering::Release);
		TRANSACTION_COUNTER.store(0, Ordering::Release);
	}

	let counter = TRANSACTION_COUNTER.fetch_add(1, Ordering::SeqCst);

	// Handle counter overflow: if we've generated 65,536 versionstamps in the same
	// microsecond, increment the timestamp and reset the counter
	let final_timestamp = if counter == u16::MAX {
		// Counter overflowed, increment timestamp
		let new_timestamp = timestamp + 1;
		LAST_TIMESTAMP.store(new_timestamp, Ordering::Release);
		TRANSACTION_COUNTER.store(0, Ordering::Release);
		new_timestamp
	} else {
		timestamp
	};

	let mut bytes = [0u8; 12];

	bytes[0..8].copy_from_slice(&final_timestamp.to_be_bytes());
	bytes[8..10].copy_from_slice(&counter.to_be_bytes());

	bytes[10..12].copy_from_slice(&user_version.to_be_bytes());

	Versionstamp::from(bytes)
}

pub fn substitute_versionstamp(
	packed_data: &mut Vec<u8>,
	versionstamp: Versionstamp,
) -> Result<(), String> {
	const VERSIONSTAMP_MARKER: u8 = 0x33;
	const VERSIONSTAMP_SIZE: usize = 12;

	if packed_data.len() < 4 {
		return Err("Packed data too short to contain versionstamp offset".to_string());
	}

	let offset_bytes = packed_data.split_off(packed_data.len() - 4);
	let offset = u32::from_le_bytes([
		offset_bytes[0],
		offset_bytes[1],
		offset_bytes[2],
		offset_bytes[3],
	]) as usize;

	if offset >= packed_data.len() {
		return Err(format!(
			"Invalid versionstamp offset: {} exceeds data length {}",
			offset,
			packed_data.len()
		));
	}

	// The offset might point to the marker or the first byte of the versionstamp
	let versionstamp_start = if packed_data.get(offset) == Some(&VERSIONSTAMP_MARKER) {
		offset + 1
	} else if offset > 0 && packed_data.get(offset - 1) == Some(&VERSIONSTAMP_MARKER) {
		// The offset points to the first byte of the versionstamp data
		offset
	} else {
		return Err(format!(
			"No versionstamp marker (0x33) found at or before offset {}",
			offset
		));
	};

	let versionstamp_end = versionstamp_start + VERSIONSTAMP_SIZE;

	if versionstamp_end > packed_data.len() {
		return Err("Versionstamp extends beyond data bounds".to_string());
	}

	let existing_bytes = &packed_data[versionstamp_start..versionstamp_end];
	if existing_bytes[0..10] != [0xff; 10] {
		// Versionstamp is already complete, nothing to do
		return Ok(());
	}

	let versionstamp_bytes = versionstamp.as_bytes();
	packed_data[versionstamp_start..versionstamp_end].copy_from_slice(versionstamp_bytes);

	Ok(())
}

pub fn pack_and_substitute_versionstamp<T: TuplePack>(
	value: &T,
	user_version: u16,
) -> Result<Vec<u8>, String> {
	let mut packed_data = pack_with_versionstamp(value);

	let versionstamp = generate_versionstamp(user_version);

	substitute_versionstamp(&mut packed_data, versionstamp)?;

	Ok(packed_data)
}

/// Checks if a value might contain an incomplete versionstamp and attempts to substitute it
///
/// This is a helper function for database drivers that want to support versionstamp substitution.
/// It detects if a value contains an incomplete versionstamp by checking for the offset marker
/// at the end, and substitutes it with a generated versionstamp.
///
/// Returns the potentially modified value. If the value doesn't contain a versionstamp marker
/// or substitution fails, returns the original value.
pub fn substitute_versionstamp_if_incomplete(mut value: Vec<u8>, user_version: u16) -> Vec<u8> {
	// Check if the value contains an incomplete versionstamp
	// An incomplete versionstamp has a 4-byte offset at the end
	if value.len() >= 4 {
		// Try to read the offset from the last 4 bytes
		let offset_bytes = &value[value.len() - 4..];
		let offset = u32::from_le_bytes([
			offset_bytes[0],
			offset_bytes[1],
			offset_bytes[2],
			offset_bytes[3],
		]) as usize;

		// Check if this could be a valid versionstamp offset
		// The offset should point within the value (excluding the 4 offset bytes)
		if offset < value.len() - 4 {
			// Check for versionstamp marker at or near the offset
			const VERSIONSTAMP_MARKER: u8 = 0x33;
			let has_marker = value.get(offset) == Some(&VERSIONSTAMP_MARKER)
				|| (offset > 0 && value.get(offset - 1) == Some(&VERSIONSTAMP_MARKER));

			if has_marker {
				// This looks like it contains an incomplete versionstamp
				// Generate a versionstamp and substitute it
				let versionstamp = generate_versionstamp(user_version);

				// Substitute the versionstamp
				// This will do nothing if the versionstamp is already complete
				let _ = substitute_versionstamp(&mut value, versionstamp);
			}
		}
	}

	value
}
