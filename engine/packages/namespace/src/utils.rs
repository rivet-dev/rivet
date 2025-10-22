use rivet_types::{
	keys::namespace::runner_config::RunnerConfigVariant,
	runner_configs::{RunnerConfig, RunnerConfigKind},
};

pub fn runner_config_variant(runner_config: &RunnerConfig) -> RunnerConfigVariant {
	match runner_config.kind {
		RunnerConfigKind::Normal { .. } => RunnerConfigVariant::Normal,
		RunnerConfigKind::Serverless { .. } => RunnerConfigVariant::Serverless,
	}
}
