use crate::options::MutationType;

/// Apply an atomic operation to a value
pub fn apply_atomic_op(
	current: Option<&[u8]>,
	param: &[u8],
	op_type: MutationType,
) -> Option<Vec<u8>> {
	match op_type {
		MutationType::Add => Some(apply_add(current, param)),
		MutationType::BitAnd => Some(apply_bit_and(current, param)),
		MutationType::BitOr => Some(apply_bit_or(current, param)),
		MutationType::BitXor => Some(apply_bit_xor(current, param)),
		MutationType::AppendIfFits => Some(apply_append_if_fits(current, param)),
		MutationType::Max => Some(apply_max(current, param)),
		MutationType::Min => Some(apply_min(current, param)),
		MutationType::ByteMin => Some(apply_byte_min(current, param)),
		MutationType::ByteMax => Some(apply_byte_max(current, param)),
		MutationType::CompareAndClear => apply_compare_and_clear(current, param),
		MutationType::SetVersionstampedKey | MutationType::SetVersionstampedValue => {
			// TODO: impl versionstamps
			Some(param.to_vec())
		}
		// Deprecated operations (fallback to bitwise operations)
		MutationType::And => Some(apply_bit_and(current, param)),
		MutationType::Or => Some(apply_bit_or(current, param)),
		MutationType::Xor => Some(apply_bit_xor(current, param)),
	}
}

fn apply_add(current: Option<&[u8]>, param: &[u8]) -> Vec<u8> {
	let current = extend_current(&current, param).collect::<Vec<_>>();

	// Convert to little-endian integers
	let current_int = bytes_to_i64_le(&current);
	let param_int = bytes_to_i64_le(param);

	let result = current_int.wrapping_add(param_int);
	i64_to_bytes_le(result, param.len().max(current.len()).max(8))
}

fn apply_bit_and(current: Option<&[u8]>, param: &[u8]) -> Vec<u8> {
	// If no current value, return param
	if current.is_none() {
		return param.to_vec();
	};

	bitwise_op(current, param, |a, b| a & b)
}

fn apply_bit_or(current: Option<&[u8]>, param: &[u8]) -> Vec<u8> {
	bitwise_op(current, param, |a, b| a | b)
}

fn apply_bit_xor(current: Option<&[u8]>, param: &[u8]) -> Vec<u8> {
	bitwise_op(current, param, |a, b| a ^ b)
}

fn apply_append_if_fits(current: Option<&[u8]>, param: &[u8]) -> Vec<u8> {
	// If no current value, return param
	let Some(current) = current else {
		return param.to_vec();
	};

	// FoundationDB has a 100KB value limit
	const MAX_VALUE_SIZE: usize = 100_000;

	let mut result = current.to_vec();
	if result.len() + param.len() <= MAX_VALUE_SIZE {
		result.extend_from_slice(param);
	}
	result
}

fn apply_max(current: Option<&[u8]>, param: &[u8]) -> Vec<u8> {
	let current = extend_current(&current, param).collect::<Vec<_>>();

	// Compare as little-endian integers
	let current_int = bytes_to_i64_le(&current);
	let param_int = bytes_to_i64_le(param);

	if param_int > current_int {
		param.to_vec()
	} else {
		current.to_vec()
	}
}

fn apply_min(current: Option<&[u8]>, param: &[u8]) -> Vec<u8> {
	// If no current value, return param
	if current.is_none() {
		return param.to_vec();
	};

	let current = extend_current(&current, param).collect::<Vec<_>>();

	// Compare as little-endian integers
	let current_int = bytes_to_i64_le(&current);
	let param_int = bytes_to_i64_le(param);

	if param_int < current_int {
		param.to_vec()
	} else {
		current.to_vec()
	}
}

fn apply_byte_min(current: Option<&[u8]>, param: &[u8]) -> Vec<u8> {
	// If no current value, return param
	let Some(current) = current else {
		return param.to_vec();
	};

	if param < current {
		param.to_vec()
	} else {
		current.to_vec()
	}
}

fn apply_byte_max(current: Option<&[u8]>, param: &[u8]) -> Vec<u8> {
	// If no current value, return param
	let Some(current) = current else {
		return param.to_vec();
	};

	if param > current {
		param.to_vec()
	} else {
		current.to_vec()
	}
}

fn apply_compare_and_clear(current: Option<&[u8]>, param: &[u8]) -> Option<Vec<u8>> {
	if current == Some(param) {
		None // Clear the key
	} else {
		current.map(|x| x.to_vec()) // Keep current value
	}
}

fn bitwise_op<F>(current: Option<&[u8]>, param: &[u8], op: F) -> Vec<u8>
where
	F: Fn(u8, u8) -> u8,
{
	extend_current(&current, param)
		.zip(param.iter().copied())
		.map(|(a, b)| op(a, b))
		.collect()
}

fn extend_current<'a>(
	current: &'a Option<&'a [u8]>,
	param: &'a [u8],
) -> impl Iterator<Item = u8> + 'a {
	current
		.iter()
		.map(|x| *x)
		.flatten()
		.copied()
		.take(param.len())
		.chain(
			std::iter::repeat(0).take(
				param
					.len()
					.saturating_sub(current.map(|x| x.len()).unwrap_or_default()),
			),
		)
}

fn bytes_to_i64_le(bytes: &[u8]) -> i64 {
	if bytes.is_empty() {
		return 0;
	}

	let mut padded = [0u8; 8];
	let len = bytes.len().min(8);
	padded[..len].copy_from_slice(&bytes[..len]);

	i64::from_le_bytes(padded)
}

fn i64_to_bytes_le(value: i64, min_len: usize) -> Vec<u8> {
	let bytes = value.to_le_bytes();
	let len = min_len.max(8);

	let mut result = vec![0u8; len];
	result[..8].copy_from_slice(&bytes);

	result
}
