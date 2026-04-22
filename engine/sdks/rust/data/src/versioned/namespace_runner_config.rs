use anyhow::{Ok, Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::*;

pub enum NamespaceRunnerConfig {
	V1(pegboard_namespace_runner_config_v1::Data),
	V2(pegboard_namespace_runner_config_v2::RunnerConfig),
	V3(pegboard_namespace_runner_config_v3::RunnerConfig),
	V4(pegboard_namespace_runner_config_v4::RunnerConfig),
	V5(pegboard_namespace_runner_config_v5::RunnerConfig),
}

impl OwnedVersionedData for NamespaceRunnerConfig {
	type Latest = pegboard_namespace_runner_config_v5::RunnerConfig;

	fn wrap_latest(latest: pegboard_namespace_runner_config_v5::RunnerConfig) -> Self {
		NamespaceRunnerConfig::V5(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let NamespaceRunnerConfig::V5(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(NamespaceRunnerConfig::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(NamespaceRunnerConfig::V2(serde_bare::from_slice(payload)?)),
			3 => Ok(NamespaceRunnerConfig::V3(serde_bare::from_slice(payload)?)),
			4 => Ok(NamespaceRunnerConfig::V4(serde_bare::from_slice(payload)?)),
			5 => Ok(NamespaceRunnerConfig::V5(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			NamespaceRunnerConfig::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			NamespaceRunnerConfig::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
			NamespaceRunnerConfig::V3(data) => serde_bare::to_vec(&data).map_err(Into::into),
			NamespaceRunnerConfig::V4(data) => serde_bare::to_vec(&data).map_err(Into::into),
			NamespaceRunnerConfig::V5(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![
			Self::v1_to_v2,
			Self::v2_to_v3,
			Self::v3_to_v4,
			Self::v4_to_v5,
		]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![
			Self::v5_to_v4,
			Self::v4_to_v3,
			Self::v3_to_v2,
			Self::v2_to_v1,
		]
	}
}

impl NamespaceRunnerConfig {
	fn v1_to_v2(self) -> Result<Self> {
		if let NamespaceRunnerConfig::V1(pegboard_namespace_runner_config_v1::Data::Serverless(serverless)) =
			self
		{
			let pegboard_namespace_runner_config_v1::Serverless {
				url,
				headers,
				request_lifespan,
				slots_per_runner,
				min_runners,
				max_runners,
				runners_margin,
			} = serverless;

			Ok(NamespaceRunnerConfig::V2(
				pegboard_namespace_runner_config_v2::RunnerConfig {
					metadata: None,
					kind: pegboard_namespace_runner_config_v2::RunnerConfigKind::Serverless(
						pegboard_namespace_runner_config_v2::Serverless {
							url,
							headers,
							request_lifespan,
							slots_per_runner,
							min_runners,
							max_runners,
							runners_margin,
						},
					),
				},
			))
		} else {
			bail!("unexpected version");
		}
	}

	fn v2_to_v1(self) -> Result<Self> {
		if let NamespaceRunnerConfig::V2(config) = self {
			let pegboard_namespace_runner_config_v2::RunnerConfig { kind, .. } = config;

			match kind {
				pegboard_namespace_runner_config_v2::RunnerConfigKind::Serverless(serverless) => {
					let pegboard_namespace_runner_config_v2::Serverless {
						url,
						headers,
						request_lifespan,
						slots_per_runner,
						min_runners,
						max_runners,
						runners_margin,
					} = serverless;

					Ok(NamespaceRunnerConfig::V1(
						pegboard_namespace_runner_config_v1::Data::Serverless(
							pegboard_namespace_runner_config_v1::Serverless {
								url,
								headers,
								request_lifespan,
								slots_per_runner,
								min_runners,
								max_runners,
								runners_margin,
							},
						),
					))
				}
				pegboard_namespace_runner_config_v2::RunnerConfigKind::Normal => {
					bail!("namespace runner config v1 does not support normal runner config")
				}
			}
		} else {
			bail!("unexpected version");
		}
	}

	fn v2_to_v3(self) -> Result<Self> {
		if let NamespaceRunnerConfig::V2(config) = self {
			let pegboard_namespace_runner_config_v2::RunnerConfig { kind, metadata } = config;

			let kind = match kind {
				pegboard_namespace_runner_config_v2::RunnerConfigKind::Serverless(serverless) => {
					pegboard_namespace_runner_config_v3::RunnerConfigKind::Serverless(
						pegboard_namespace_runner_config_v3::Serverless {
							url: serverless.url,
							headers: serverless.headers,
							request_lifespan: serverless.request_lifespan,
							slots_per_runner: serverless.slots_per_runner,
							min_runners: serverless.min_runners,
							max_runners: serverless.max_runners,
							runners_margin: serverless.runners_margin,
						},
					)
				}
				pegboard_namespace_runner_config_v2::RunnerConfigKind::Normal => {
					pegboard_namespace_runner_config_v3::RunnerConfigKind::Normal
				}
			};

			Ok(NamespaceRunnerConfig::V3(
				pegboard_namespace_runner_config_v3::RunnerConfig {
					kind,
					metadata,
					// Default to false for v2 -> v3 migration
					drain_on_version_upgrade: false,
				},
			))
		} else {
			bail!("unexpected version");
		}
	}

	fn v3_to_v2(self) -> Result<Self> {
		if let NamespaceRunnerConfig::V3(config) = self {
			let pegboard_namespace_runner_config_v3::RunnerConfig { kind, metadata, .. } = config;

			let kind = match kind {
				pegboard_namespace_runner_config_v3::RunnerConfigKind::Serverless(serverless) => {
					pegboard_namespace_runner_config_v2::RunnerConfigKind::Serverless(
						pegboard_namespace_runner_config_v2::Serverless {
							url: serverless.url,
							headers: serverless.headers,
							request_lifespan: serverless.request_lifespan,
							slots_per_runner: serverless.slots_per_runner,
							min_runners: serverless.min_runners,
							max_runners: serverless.max_runners,
							runners_margin: serverless.runners_margin,
						},
					)
				}
				pegboard_namespace_runner_config_v3::RunnerConfigKind::Normal => {
					pegboard_namespace_runner_config_v2::RunnerConfigKind::Normal
				}
			};

			Ok(NamespaceRunnerConfig::V2(
				pegboard_namespace_runner_config_v2::RunnerConfig { kind, metadata },
			))
		} else {
			bail!("unexpected version");
		}
	}

	fn v3_to_v4(self) -> Result<Self> {
		if let NamespaceRunnerConfig::V3(config) = self {
			let pegboard_namespace_runner_config_v3::RunnerConfig {
				kind,
				metadata,
				drain_on_version_upgrade,
			} = config;

			let kind = match kind {
				pegboard_namespace_runner_config_v3::RunnerConfigKind::Serverless(serverless) => {
					pegboard_namespace_runner_config_v4::RunnerConfigKind::Serverless(
						pegboard_namespace_runner_config_v4::Serverless {
							url: serverless.url,
							headers: serverless.headers,
							request_lifespan: serverless.request_lifespan,
							slots_per_runner: serverless.slots_per_runner,
							min_runners: serverless.min_runners,
							max_runners: serverless.max_runners,
							runners_margin: serverless.runners_margin,
							// Default to None for v3 -> v4 migration
							metadata_poll_interval: None,
						},
					)
				}
				pegboard_namespace_runner_config_v3::RunnerConfigKind::Normal => {
					pegboard_namespace_runner_config_v4::RunnerConfigKind::Normal
				}
			};

			Ok(NamespaceRunnerConfig::V4(
				pegboard_namespace_runner_config_v4::RunnerConfig {
					kind,
					metadata,
					drain_on_version_upgrade,
				},
			))
		} else {
			bail!("unexpected version");
		}
	}

	fn v4_to_v3(self) -> Result<Self> {
		if let NamespaceRunnerConfig::V4(config) = self {
			let pegboard_namespace_runner_config_v4::RunnerConfig {
				kind,
				metadata,
				drain_on_version_upgrade,
			} = config;

			let kind = match kind {
				pegboard_namespace_runner_config_v4::RunnerConfigKind::Serverless(serverless) => {
					pegboard_namespace_runner_config_v3::RunnerConfigKind::Serverless(
						pegboard_namespace_runner_config_v3::Serverless {
							url: serverless.url,
							headers: serverless.headers,
							request_lifespan: serverless.request_lifespan,
							slots_per_runner: serverless.slots_per_runner,
							min_runners: serverless.min_runners,
							max_runners: serverless.max_runners,
							runners_margin: serverless.runners_margin,
							// metadata_poll_interval is dropped in downgrade
						},
					)
				}
				pegboard_namespace_runner_config_v4::RunnerConfigKind::Normal => {
					pegboard_namespace_runner_config_v3::RunnerConfigKind::Normal
				}
			};

			Ok(NamespaceRunnerConfig::V3(
				pegboard_namespace_runner_config_v3::RunnerConfig {
					kind,
					metadata,
					drain_on_version_upgrade,
				},
			))
		} else {
			bail!("unexpected version");
		}
	}

	fn v4_to_v5(self) -> Result<Self> {
		if let NamespaceRunnerConfig::V4(config) = self {
			let pegboard_namespace_runner_config_v4::RunnerConfig {
				kind,
				metadata,
				drain_on_version_upgrade,
			} = config;

			let kind = match kind {
				pegboard_namespace_runner_config_v4::RunnerConfigKind::Serverless(serverless) => {
					pegboard_namespace_runner_config_v5::RunnerConfigKind::Serverless(
						pegboard_namespace_runner_config_v5::Serverless {
							url: serverless.url,
							headers: serverless.headers,
							request_lifespan: serverless.request_lifespan,
							// Default to max_runners for v4 -> v5 migration
							max_concurrent_actors: serverless.max_runners as u64,
							// Default to deprecated config value (config.pegboard.serverless_drain_grace_period)
							drain_grace_period: 10_000,
							slots_per_runner: serverless.slots_per_runner,
							min_runners: serverless.min_runners,
							max_runners: serverless.max_runners,
							runners_margin: serverless.runners_margin,
							metadata_poll_interval: serverless.metadata_poll_interval,
						},
					)
				}
				pegboard_namespace_runner_config_v4::RunnerConfigKind::Normal => {
					pegboard_namespace_runner_config_v5::RunnerConfigKind::Normal
				}
			};

			Ok(NamespaceRunnerConfig::V5(
				pegboard_namespace_runner_config_v5::RunnerConfig {
					kind,
					metadata,
					drain_on_version_upgrade,
				},
			))
		} else {
			bail!("unexpected version");
		}
	}

	fn v5_to_v4(self) -> Result<Self> {
		if let NamespaceRunnerConfig::V5(config) = self {
			let pegboard_namespace_runner_config_v5::RunnerConfig {
				kind,
				metadata,
				drain_on_version_upgrade,
			} = config;

			let kind = match kind {
				pegboard_namespace_runner_config_v5::RunnerConfigKind::Serverless(serverless) => {
					pegboard_namespace_runner_config_v4::RunnerConfigKind::Serverless(
						pegboard_namespace_runner_config_v4::Serverless {
							url: serverless.url,
							headers: serverless.headers,
							request_lifespan: serverless.request_lifespan,
							// max_concurrent_actors is dropped in downgrade
							slots_per_runner: serverless.slots_per_runner,
							min_runners: serverless.min_runners,
							max_runners: serverless.max_runners,
							runners_margin: serverless.runners_margin,
							metadata_poll_interval: serverless.metadata_poll_interval,
						},
					)
				}
				pegboard_namespace_runner_config_v5::RunnerConfigKind::Normal => {
					pegboard_namespace_runner_config_v4::RunnerConfigKind::Normal
				}
			};

			Ok(NamespaceRunnerConfig::V4(
				pegboard_namespace_runner_config_v4::RunnerConfig {
					kind,
					metadata,
					drain_on_version_upgrade,
				},
			))
		} else {
			bail!("unexpected version");
		}
	}
}
