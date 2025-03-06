use anyhow::*;

pub fn var(name: &str) -> Result<String> {
	std::env::var(name).context(name.to_string())
}
