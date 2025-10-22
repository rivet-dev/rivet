use uuid::Uuid;

/// Sorts two UUIDs such that the first returned ID is before the second.
///
/// This matches the functionality of comparing the two UUIDs alphabetically,
/// like we do in JS.
pub fn id_pair(id_a: Uuid, id_b: Uuid) -> (Uuid, Uuid) {
	if id_a < id_b {
		(id_a, id_b)
	} else {
		(id_b, id_a)
	}
}
