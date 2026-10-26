//! Relative file addresses within a worker's configured log directory.
//! Each worker has one shard, numbered zero. Segment names carry the first
//! batch sequence, and a corrupt segment retains that sequence when quarantined.

use anyhow::{Result, bail, ensure};
use rivet_keys::keys::codec::{TextComponent, write_display};

rivet_keys::fs_key! {
	pub LogShardKey(pub file: LogShardFile) => ["shards", "0", {file}];
	value: bytes;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogShardFile {
	Format,
	FormatTemp,
	Checkpoint,
	CheckpointTemp,
	/// A delta checkpoint over the chain ending at its predecessor, named by the batch
	/// boundary it covers.
	CheckpointDelta {
		last_included_log_seq: u64,
	},
	CheckpointDeltaTemp,
	Segment {
		first_log_seq: u64,
	},
	CorruptSegment {
		first_log_seq: u64,
	},
	/// A durably zero-filled file that the next segment roll renames into place. It holds
	/// no batches until that rename, so recovery deletes it.
	SpareSegment,
	/// A spare segment while it is being zero-filled, before its contents are synced.
	SpareSegmentTemp,
}

impl TextComponent for LogShardFile {
	fn to_text(&self) -> String {
		let mut out = String::new();
		self.write_text(&mut out);
		out
	}

	fn write_text(&self, out: &mut String) {
		match self {
			Self::Format => out.push_str("FORMAT"),
			Self::FormatTemp => out.push_str("FORMAT.tmp"),
			Self::Checkpoint => out.push_str("checkpoint"),
			Self::CheckpointTemp => out.push_str("checkpoint.tmp"),
			Self::CheckpointDelta {
				last_included_log_seq,
			} => {
				out.push_str("checkpoint-delta-");
				write_display(out, last_included_log_seq);
			}
			Self::CheckpointDeltaTemp => out.push_str("checkpoint-delta.tmp"),
			Self::Segment { first_log_seq } => {
				out.push_str("seg-");
				write_display(out, first_log_seq);
			}
			Self::CorruptSegment { first_log_seq } => {
				out.push_str("seg-");
				write_display(out, first_log_seq);
				out.push_str(".corrupt");
			}
			Self::SpareSegment => out.push_str("seg-spare"),
			Self::SpareSegmentTemp => out.push_str("seg-spare.tmp"),
		}
	}

	fn from_text(name: &str) -> Result<Self> {
		let key = match name {
			"FORMAT" => Self::Format,
			"FORMAT.tmp" => Self::FormatTemp,
			"checkpoint" => Self::Checkpoint,
			"checkpoint.tmp" => Self::CheckpointTemp,
			"checkpoint-delta.tmp" => Self::CheckpointDeltaTemp,
			"seg-spare" => Self::SpareSegment,
			"seg-spare.tmp" => Self::SpareSegmentTemp,
			_ if name.starts_with("checkpoint-delta-") => Self::CheckpointDelta {
				last_included_log_seq: name["checkpoint-delta-".len()..].parse()?,
			},
			_ => {
				let Some(sequence) = name.strip_prefix("seg-") else {
					bail!("unknown log shard file");
				};
				match sequence.strip_suffix(".corrupt") {
					Some(sequence) => Self::CorruptSegment {
						first_log_seq: sequence.parse()?,
					},
					None => Self::Segment {
						first_log_seq: sequence.parse()?,
					},
				}
			}
		};
		ensure!(key.to_text() == name, "noncanonical log shard filename");
		Ok(key)
	}
}
