use rivet_types::namespaces::RunnerConfig;

use crate::keys;

pub fn runner_config_variant(runner_config: &RunnerConfig) -> keys::RunnerConfigVariant {
	match runner_config {
		RunnerConfig::Serverless { .. } => keys::RunnerConfigVariant::Serverless,
	}
}
