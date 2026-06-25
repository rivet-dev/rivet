use anyhow::Result;
use rivet_universaldb_commit::{self as proto, versioned};
use vbare::OwnedVersionedData;

use crate::{
	options::{ConflictRangeType, MutationType},
	tx_ops::Operation,
};

/// Decoded form of a `udb_commit_requests.payload` blob.
///
/// `read_version` is intentionally omitted: it is also denormalized into the `read_version` column,
/// which is what the leader's drain reads, so decoding it here would be dead.
pub struct DecodedCommit {
	pub conflict_ranges: Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>,
	pub operations: Vec<Operation>,
}

/// Encode a follower's commit request to the versioned BARE wire format with an embedded version
/// header so a leader running older or newer code can still decode it during a rolling deploy.
pub fn encode_commit_request(
	read_version: u64,
	conflict_ranges: &[(Vec<u8>, Vec<u8>, ConflictRangeType)],
	operations: &[Operation],
) -> Result<Vec<u8>> {
	let request = proto::CommitRequest {
		read_version,
		conflict_ranges: conflict_ranges
			.iter()
			.map(|(begin, end, kind)| proto::ConflictRange {
				begin: begin.clone(),
				end: end.clone(),
				kind: conflict_range_type_to_proto(*kind),
			})
			.collect(),
		operations: operations.iter().map(operation_to_proto).collect(),
	};

	versioned::CommitRequest::wrap_latest(request)
		.serialize_with_embedded_version(proto::PROTOCOL_VERSION)
}

/// Decode a `udb_commit_requests.payload` blob produced by [`encode_commit_request`].
pub fn decode_commit_request(payload: &[u8]) -> Result<DecodedCommit> {
	let request = versioned::CommitRequest::deserialize_with_embedded_version(payload)?;

	let conflict_ranges = request
		.conflict_ranges
		.into_iter()
		.map(|range| {
			(
				range.begin,
				range.end,
				conflict_range_type_from_proto(range.kind),
			)
		})
		.collect();

	let operations = request
		.operations
		.into_iter()
		.map(operation_from_proto)
		.collect();

	Ok(DecodedCommit {
		conflict_ranges,
		operations,
	})
}

fn conflict_range_type_to_proto(kind: ConflictRangeType) -> proto::ConflictRangeType {
	match kind {
		ConflictRangeType::Read => proto::ConflictRangeType::Read,
		ConflictRangeType::Write => proto::ConflictRangeType::Write,
	}
}

fn conflict_range_type_from_proto(kind: proto::ConflictRangeType) -> ConflictRangeType {
	match kind {
		proto::ConflictRangeType::Read => ConflictRangeType::Read,
		proto::ConflictRangeType::Write => ConflictRangeType::Write,
	}
}

fn operation_to_proto(op: &Operation) -> proto::Operation {
	match op {
		Operation::SetValue { key, value } => proto::Operation::SetValue(proto::SetValue {
			key: key.clone(),
			value: value.clone(),
		}),
		Operation::Clear { key } => proto::Operation::Clear(proto::Clear { key: key.clone() }),
		Operation::ClearRange { begin, end } => proto::Operation::ClearRange(proto::ClearRange {
			begin: begin.clone(),
			end: end.clone(),
		}),
		Operation::AtomicOp {
			key,
			param,
			op_type,
		} => proto::Operation::AtomicOp(proto::AtomicOp {
			key: key.clone(),
			param: param.clone(),
			op_type: mutation_type_to_proto(*op_type),
		}),
	}
}

fn operation_from_proto(op: proto::Operation) -> Operation {
	match op {
		proto::Operation::SetValue(proto::SetValue { key, value }) => {
			Operation::SetValue { key, value }
		}
		proto::Operation::Clear(proto::Clear { key }) => Operation::Clear { key },
		proto::Operation::ClearRange(proto::ClearRange { begin, end }) => {
			Operation::ClearRange { begin, end }
		}
		proto::Operation::AtomicOp(proto::AtomicOp {
			key,
			param,
			op_type,
		}) => Operation::AtomicOp {
			key,
			param,
			op_type: mutation_type_from_proto(op_type),
		},
	}
}

fn mutation_type_to_proto(op_type: MutationType) -> proto::MutationType {
	match op_type {
		MutationType::Add => proto::MutationType::Add,
		MutationType::And => proto::MutationType::And,
		MutationType::BitAnd => proto::MutationType::BitAnd,
		MutationType::Or => proto::MutationType::Or,
		MutationType::BitOr => proto::MutationType::BitOr,
		MutationType::Xor => proto::MutationType::Xor,
		MutationType::BitXor => proto::MutationType::BitXor,
		MutationType::AppendIfFits => proto::MutationType::AppendIfFits,
		MutationType::Max => proto::MutationType::Max,
		MutationType::Min => proto::MutationType::Min,
		MutationType::SetVersionstampedKey => proto::MutationType::SetVersionstampedKey,
		MutationType::SetVersionstampedValue => proto::MutationType::SetVersionstampedValue,
		MutationType::ByteMin => proto::MutationType::ByteMin,
		MutationType::ByteMax => proto::MutationType::ByteMax,
		MutationType::CompareAndClear => proto::MutationType::CompareAndClear,
	}
}

fn mutation_type_from_proto(op_type: proto::MutationType) -> MutationType {
	match op_type {
		proto::MutationType::Add => MutationType::Add,
		proto::MutationType::And => MutationType::And,
		proto::MutationType::BitAnd => MutationType::BitAnd,
		proto::MutationType::Or => MutationType::Or,
		proto::MutationType::BitOr => MutationType::BitOr,
		proto::MutationType::Xor => MutationType::Xor,
		proto::MutationType::BitXor => MutationType::BitXor,
		proto::MutationType::AppendIfFits => MutationType::AppendIfFits,
		proto::MutationType::Max => MutationType::Max,
		proto::MutationType::Min => MutationType::Min,
		proto::MutationType::SetVersionstampedKey => MutationType::SetVersionstampedKey,
		proto::MutationType::SetVersionstampedValue => MutationType::SetVersionstampedValue,
		proto::MutationType::ByteMin => MutationType::ByteMin,
		proto::MutationType::ByteMax => MutationType::ByteMax,
		proto::MutationType::CompareAndClear => MutationType::CompareAndClear,
	}
}
