use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Epoxy runs in a single datacenter with fixed membership, so it has no settings yet. The
/// section stays so multi-region settings have a home.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Epoxy {}

impl Epoxy {
	pub fn validate(&self) -> Result<()> {
		Ok(())
	}
}
