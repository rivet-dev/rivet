use gas::prelude::*;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, strum::FromRepr, ToSchema)]
#[schema(as = RunnerPoolVariant)]
#[serde(rename_all = "snake_case")]
pub enum EnvoyPoolVariant {
	Serverless = 0,
	Normal = 1,
}

impl EnvoyPoolVariant {
	pub fn parse(v: &str) -> Option<Self> {
		match v {
			"serverless" => Some(EnvoyPoolVariant::Serverless),
			"normal" => Some(EnvoyPoolVariant::Normal),
			_ => None,
		}
	}
}

impl std::fmt::Display for EnvoyPoolVariant {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			EnvoyPoolVariant::Serverless => write!(f, "serverless"),
			EnvoyPoolVariant::Normal => write!(f, "normal"),
		}
	}
}
