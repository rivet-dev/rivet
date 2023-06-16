use serde::Deserialize;

pub fn decode(s: &str) -> Result<Project, toml::de::Error> {
	toml::from_str(s)
}

/// Configuration for the Bolt.toml at the root of the project.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct Project {
	#[serde(default)]
	pub billing_enabled: bool,
}
