use gas::prelude::*;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Namespace {
	pub namespace_id: Id,
	pub name: String,
	pub display_name: String,
	pub create_ts: i64,
	pub runner_kind: RunnerKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = NamespacesRunnerKind)]
pub enum RunnerKind {
	Outbound {
		url: String,
		/// Seconds.
		request_lifespan: u32,
		slots_per_runner: u32,
		min_runners: u32,
		max_runners: u32,
		runners_margin: u32,
	},
	Custom,
}

impl From<RunnerKind> for rivet_data::generated::namespace_runner_kind_v1::Data {
	fn from(value: RunnerKind) -> Self {
		match value {
			RunnerKind::Outbound {
				url,
				request_lifespan,
				slots_per_runner,
				min_runners,
				max_runners,
				runners_margin,
			} => rivet_data::generated::namespace_runner_kind_v1::Data::Outbound(
				rivet_data::generated::namespace_runner_kind_v1::Outbound {
					url,
					request_lifespan,
					slots_per_runner,
					min_runners,
					max_runners,
					runners_margin,
				},
			),
			RunnerKind::Custom => rivet_data::generated::namespace_runner_kind_v1::Data::Custom,
		}
	}
}

impl From<rivet_data::generated::namespace_runner_kind_v1::Data> for RunnerKind {
	fn from(value: rivet_data::generated::namespace_runner_kind_v1::Data) -> Self {
		match value {
			rivet_data::generated::namespace_runner_kind_v1::Data::Outbound(o) => {
				RunnerKind::Outbound {
					url: o.url,
					request_lifespan: o.request_lifespan,
					slots_per_runner: o.slots_per_runner,
					min_runners: o.min_runners,
					max_runners: o.max_runners,
					runners_margin: o.runners_margin,
				}
			}
			rivet_data::generated::namespace_runner_kind_v1::Data::Custom => RunnerKind::Custom,
		}
	}
}
