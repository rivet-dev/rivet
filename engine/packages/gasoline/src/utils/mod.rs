pub mod tags;
pub mod time;

/// Returns true if `subset` is a subset of `superset`.
pub fn is_value_subset(subset: &serde_json::Value, superset: &serde_json::Value) -> bool {
	match (subset, superset) {
		(serde_json::Value::Object(sub_obj), serde_json::Value::Object(super_obj)) => {
			sub_obj.iter().all(|(k, sub_val)| {
				super_obj
					.get(k)
					.map_or(false, |super_val| is_value_subset(sub_val, super_val))
			})
		}
		(serde_json::Value::Array(sub_arr), serde_json::Value::Array(super_arr)) => sub_arr
			.iter()
			.zip(super_arr)
			.all(|(sub_val, super_val)| is_value_subset(sub_val, super_val)),
		_ => subset == superset,
	}
}
