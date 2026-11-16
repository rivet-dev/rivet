//! Inode numbers packed from the claim log that created a file and that log's create counter.
//!
//! A created inode is numbered by its create entry: the claim log id in the high 13 bits and
//! the entry's sequence in that claim log as the low 40 bits, 53 bits in all. Numbers stay
//! below 2^53 so that runtimes reading them as doubles, such as JavaScript's `fs.stat`, never
//! see two files with the same number. The root has a fixed number, and 0 is never issued,
//! because tools read inode 0 as unknown. A file keeps its number when a cede moves its
//! directory into another claim: the high bits name the claim log that created it, not the
//! one that owns it now.

use crate::{Error, Result};

/// Bits of an inode number that name the claim log that created it.
pub const CLAIM_LOG_BITS: u32 = 13;

/// Bits of an inode number that hold the creating claim log's counter.
pub const COUNTER_BITS: u32 = 40;

/// The highest claim log id an inode number can carry.
pub const MAX_CLAIM_LOG: u16 = (1 << CLAIM_LOG_BITS) - 1;

/// The highest create counter of one claim log.
pub const MAX_COUNTER: u64 = (1 << COUNTER_BITS) - 1;

/// The claim log of a volume's default claim on `/`. Its files keep small numbers, equal to
/// their create entry's sequence.
pub use rivet_data::ROOT_CLAIM_LOG;

/// The inode number of a file created by the entry at `counter` in `claim_log`. A claim log
/// whose counter ran out refuses the create with `NoInodes`.
pub fn pack(claim_log: u16, counter: u64) -> Result<u64> {
	if claim_log > MAX_CLAIM_LOG {
		return Err(Error::Invalid);
	}
	if counter == 0 || counter > MAX_COUNTER {
		return Err(Error::NoInodes);
	}
	Ok((u64::from(claim_log) << COUNTER_BITS) | counter)
}

/// The claim log that created `ino`.
pub fn claim_log(ino: u64) -> u16 {
	((ino >> COUNTER_BITS) & u64::from(MAX_CLAIM_LOG)) as u16
}

/// The create counter of `ino` within its claim log.
pub fn counter(ino: u64) -> u64 {
	ino & MAX_COUNTER
}

/// The inode number a create entry at `volume_seq` of the default claim log issues.
pub fn created(volume_seq: u64) -> Result<u64> {
	pack(ROOT_CLAIM_LOG, volume_seq)
}
