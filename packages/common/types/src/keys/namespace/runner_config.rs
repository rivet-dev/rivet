use gas::prelude::*;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, strum::FromRepr, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunnerConfigVariant {
	Serverless = 0,
	Normal = 1,
}

impl RunnerConfigVariant {
	pub fn parse(v: &str) -> Option<Self> {
		match v {
			"serverless" => Some(RunnerConfigVariant::Serverless),
			"normal" => Some(RunnerConfigVariant::Normal),
			_ => None,
		}
	}
}

impl std::fmt::Display for RunnerConfigVariant {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			RunnerConfigVariant::Serverless => write!(f, "serverless"),
			RunnerConfigVariant::Normal => write!(f, "normal"),
		}
	}
}
