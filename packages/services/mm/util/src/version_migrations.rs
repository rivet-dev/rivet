use bit_vec::BitVec;

pub const PORT_RANGE_PROXY_IDX: usize = 0;

/// The bit flags expected for a game version with all migrations applied.
pub fn all() -> BitVec {
	BitVec::from_elem(1, true)
}
