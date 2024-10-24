use std::collections::HashMap;

use global_error::prelude::*;

/// Converts the topic map to a deterministic string.
///
/// This is used for efficient and simple database keys.
pub fn serialize_topic_str(topic: &HashMap<String, String>) -> GlobalResult<String> {
	// TODO: Improve performance of this

	// Build deterministic topic string
	let mut topic_pairs = topic.clone().into_iter().collect::<Vec<(String, String)>>();
	topic_pairs.sort_by_key(|x| x.0.clone());
	let topic_str = serde_json::to_string(&topic_pairs)?;

	Ok(topic_str)
}
