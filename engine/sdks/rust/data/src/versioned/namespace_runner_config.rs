use anyhow::{Ok, Result, bail};
use vbare::OwnedVersionedData;

use crate::generated::*;

pub enum NamespaceRunnerConfig {
	V1(namespace_runner_config_v1::Data),
	V2(namespace_runner_config_v2::RunnerConfig),
}

impl OwnedVersionedData for NamespaceRunnerConfig {
	type Latest = namespace_runner_config_v2::RunnerConfig;

	fn wrap_latest(latest: namespace_runner_config_v2::RunnerConfig) -> Self {
		NamespaceRunnerConfig::V2(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		#[allow(irrefutable_let_patterns)]
		if let NamespaceRunnerConfig::V2(data) = self {
			Ok(data)
		} else {
			bail!("version not latest");
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(NamespaceRunnerConfig::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(NamespaceRunnerConfig::V2(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			NamespaceRunnerConfig::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
			NamespaceRunnerConfig::V2(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v2_to_v1]
	}
}

impl NamespaceRunnerConfig {
	fn v1_to_v2(self) -> Result<Self> {
		if let NamespaceRunnerConfig::V1(namespace_runner_config_v1::Data::Serverless(serverless)) =
			self
		{
			let namespace_runner_config_v1::Serverless {
				url,
				headers,
				request_lifespan,
				slots_per_runner,
				min_runners,
				max_runners,
				runners_margin,
			} = serverless;

			Ok(NamespaceRunnerConfig::V2(
				namespace_runner_config_v2::RunnerConfig {
					metadata: None,
					kind: namespace_runner_config_v2::RunnerConfigKind::Serverless(
						namespace_runner_config_v2::Serverless {
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
			let namespace_runner_config_v2::RunnerConfig { kind, .. } = config;

			match kind {
				namespace_runner_config_v2::RunnerConfigKind::Serverless(serverless) => {
					let namespace_runner_config_v2::Serverless {
						url,
						headers,
						request_lifespan,
						slots_per_runner,
						min_runners,
						max_runners,
						runners_margin,
					} = serverless;

					Ok(NamespaceRunnerConfig::V1(
						namespace_runner_config_v1::Data::Serverless(
							namespace_runner_config_v1::Serverless {
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
				namespace_runner_config_v2::RunnerConfigKind::Normal => {
					bail!("namespace runner config v1 does not support normal runner config")
				}
			}
		} else {
			bail!("unexpected version");
		}
	}
}
