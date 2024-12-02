use anyhow::*;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

/// Parses a string like `foo=bar,hello=world` in to a Serde struct.
///
/// This uses `envy` under the hood. Refer to that for reference on behavior.
pub fn from_str<T: DeserializeOwned>(input: &str) -> Result<T> {
	let vars_iter = input
		.split(',')
		.map(|pair| pair.split_once('=').unwrap_or((&pair, "true")))
		.map(|(k, v)| (k.to_string(), v.to_string()));
	let output = envy::from_iter::<_, T>(vars_iter)?;
	Ok(output)
}

pub fn to_str(input: &HashMap<String, String>) -> Result<String> {
	let mut input = input
		.iter()
		.map(|(k, v)| format!("{k}={v}"))
		.collect::<Vec<_>>();
	input.sort();
	Ok(input.join(" "))
}
