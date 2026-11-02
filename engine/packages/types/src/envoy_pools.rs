use std::collections::HashMap;

use gas::prelude::*;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = RunnerPoolConfig)]
pub struct EnvoyPool {
	#[serde(flatten)]
	pub kind: EnvoyPoolKind,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

impl EnvoyPool {
	pub fn drain_on_version_upgrade(&self) -> bool {
		match &self.kind {
			EnvoyPoolKind::Normal {
				drain_on_version_upgrade,
				..
			} => *drain_on_version_upgrade,
			EnvoyPoolKind::Serverless {
				drain_on_version_upgrade,
				..
			} => *drain_on_version_upgrade,
		}
	}

	pub fn actor_eviction_delay(&self) -> u32 {
		match &self.kind {
			EnvoyPoolKind::Normal {
				actor_eviction_delay,
				..
			} => *actor_eviction_delay,
			EnvoyPoolKind::Serverless {
				actor_eviction_delay,
				..
			} => *actor_eviction_delay,
		}
	}

	pub fn actor_eviction_period(&self) -> u32 {
		match &self.kind {
			EnvoyPoolKind::Normal {
				actor_eviction_period,
				..
			} => *actor_eviction_period,
			EnvoyPoolKind::Serverless {
				actor_eviction_period,
				..
			} => *actor_eviction_period,
		}
	}

	pub fn actor_eviction_rate(&self) -> f32 {
		match &self.kind {
			EnvoyPoolKind::Normal {
				actor_eviction_rate,
				..
			} => *actor_eviction_rate,
			EnvoyPoolKind::Serverless {
				actor_eviction_rate,
				..
			} => *actor_eviction_rate,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = RunnerPoolConfigKind)]
#[serde(rename_all = "snake_case")]
pub enum EnvoyPoolKind {
	Normal {
		#[serde(default = "default_drain_on_version_upgrade")]
		drain_on_version_upgrade: bool,
		/// Seconds.
		#[serde(default = "default_actor_eviction_delay")]
		actor_eviction_delay: u32,
		/// Seconds.
		#[serde(default = "default_actor_eviction_period")]
		actor_eviction_period: u32,
		/// Actors per second.
		#[serde(default = "default_actor_eviction_rate")]
		actor_eviction_rate: f32,
	},
	Serverless {
		url: String,
		headers: HashMap<String, String>,
		/// Seconds.
		request_lifespan: u32,
		max_concurrent_actors: u64,
		/// Seconds.
		drain_grace_period: u32,
		/// Deprecated.
		slots_per_runner: u32,
		/// Deprecated.
		min_runners: u32,
		/// Deprecated.
		max_runners: u32,
		/// Deprecated.
		runners_margin: u32,
		/// Milliseconds between metadata polling. If not set, uses the global default.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		metadata_poll_interval: Option<u64>,
		#[serde(default = "default_drain_on_version_upgrade")]
		drain_on_version_upgrade: bool,
		/// Seconds.
		#[serde(default = "default_actor_eviction_delay")]
		actor_eviction_delay: u32,
		/// Seconds.
		#[serde(default = "default_actor_eviction_period")]
		actor_eviction_period: u32,
		/// Actors per second.
		#[serde(default = "default_actor_eviction_rate")]
		actor_eviction_rate: f32,
	},
}

fn default_drain_on_version_upgrade() -> bool {
	false
}

fn default_actor_eviction_delay() -> u32 {
	0
}

fn default_actor_eviction_period() -> u32 {
	0
}

fn default_actor_eviction_rate() -> f32 {
	1.0
}

impl From<EnvoyPool> for rivet_data::generated::pegboard_namespace_envoy_pool_v1::EnvoyPool {
	fn from(value: EnvoyPool) -> Self {
		let EnvoyPool { kind, metadata } = value;
		rivet_data::generated::pegboard_namespace_envoy_pool_v1::EnvoyPool {
			metadata: metadata.and_then(|value| serde_json::to_string(&value).ok()),
			kind: match kind {
				EnvoyPoolKind::Normal { drain_on_version_upgrade, actor_eviction_delay, actor_eviction_period, actor_eviction_rate } => {
					rivet_data::generated::pegboard_namespace_envoy_pool_v1::EnvoyPoolKind::Normal(rivet_data::generated::pegboard_namespace_envoy_pool_v1::Normal {
						drain_on_version_upgrade,
						actor_eviction_delay,
						actor_eviction_period,
						actor_eviction_rate,
					})
				}
				EnvoyPoolKind::Serverless {
					url,
					headers,
					request_lifespan,
					max_concurrent_actors,
					drain_grace_period,
					slots_per_runner,
					min_runners,
					max_runners,
					runners_margin,
					metadata_poll_interval,
					drain_on_version_upgrade,
					actor_eviction_delay,
					actor_eviction_period,
					actor_eviction_rate,
				} => {
					rivet_data::generated::pegboard_namespace_envoy_pool_v1::EnvoyPoolKind::Serverless(
						rivet_data::generated::pegboard_namespace_envoy_pool_v1::Serverless {
							url,
							headers: headers.into(),
							request_lifespan,
							max_concurrent_actors,
							drain_grace_period,
							slots_per_runner: slots_per_runner,
							min_runners: min_runners,
							max_runners: max_runners,
							runners_margin: runners_margin,
							metadata_poll_interval,
							drain_on_version_upgrade,
							actor_eviction_delay,
							actor_eviction_period,
							actor_eviction_rate,
						},
					)
				}
			},
		}
	}
}

impl From<rivet_data::generated::pegboard_namespace_envoy_pool_v1::EnvoyPool> for EnvoyPool {
	fn from(value: rivet_data::generated::pegboard_namespace_envoy_pool_v1::EnvoyPool) -> Self {
		let rivet_data::generated::pegboard_namespace_envoy_pool_v1::EnvoyPool { metadata, kind } =
			value;
		let kind = match kind {
			rivet_data::generated::pegboard_namespace_envoy_pool_v1::EnvoyPoolKind::Normal(o) => {
				EnvoyPoolKind::Normal {
					drain_on_version_upgrade: o.drain_on_version_upgrade,
					actor_eviction_delay: o.actor_eviction_delay,
					actor_eviction_period: o.actor_eviction_period,
					actor_eviction_rate: o.actor_eviction_rate,
				}
			}
			rivet_data::generated::pegboard_namespace_envoy_pool_v1::EnvoyPoolKind::Serverless(
				o,
			) => EnvoyPoolKind::Serverless {
				url: o.url,
				headers: o.headers.into(),
				request_lifespan: o.request_lifespan,
				max_concurrent_actors: o.max_concurrent_actors,
				drain_grace_period: o.drain_grace_period,
				slots_per_runner: o.slots_per_runner,
				min_runners: o.min_runners,
				max_runners: o.max_runners,
				runners_margin: o.runners_margin,
				metadata_poll_interval: o.metadata_poll_interval,
				drain_on_version_upgrade: o.drain_on_version_upgrade,
				actor_eviction_delay: o.actor_eviction_delay,
				actor_eviction_period: o.actor_eviction_period,
				actor_eviction_rate: o.actor_eviction_rate,
			},
		};
		EnvoyPool {
			metadata: metadata.and_then(|raw| serde_json::from_str(&raw).ok()),
			kind,
		}
	}
}

impl EnvoyPool {
	/// If updates to this run config affects the pool.
	pub fn affects_pool(&self) -> bool {
		matches!(self.kind, EnvoyPoolKind::Serverless { .. })
	}
}
