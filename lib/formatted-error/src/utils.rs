use std::{collections::HashMap, string::ToString};

pub(crate) fn render_template(template: &'static str, context: &HashMap<String, String>) -> String {
	let mut potential_replace = false;
	let mut start_index = 0;
	let mut is_escaped = false;

	template
		.chars()
		.enumerate()
		.map(|(i, c)| {
			if c == '{' {
				// Double opening bracket (escaped)
				if potential_replace {
					potential_replace = false;
					is_escaped = true;

					Some(c.to_string())
				}
				// Single opening bracket
				else {
					potential_replace = true;
					start_index = i;

					None
				}
			} else if potential_replace {
				// Unescaped match found
				if c == '}' {
					potential_replace = false;

					// Get key of context
					let key = template[start_index + 1..i].to_string();

					// Return insert
					Some(context.get(&key).cloned().unwrap_or("?".to_string()))
				} else {
					None
				}
			}
			// Single closing bracket (when escaped)
			else if is_escaped && c == '}' {
				is_escaped = false;
				None
			}
			// Anything else
			else {
				Some(c.to_string())
			}
		})
		.flatten()
		.collect::<String>()
}
