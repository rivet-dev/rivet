use rivet_types::{
	keys::namespace::runner_config::RunnerConfigVariant, runner_configs::RunnerConfig,
};

pub fn runner_config_variant(runner_config: &RunnerConfig) -> RunnerConfigVariant {
	match runner_config {
		RunnerConfig::Serverless { .. } => RunnerConfigVariant::Serverless,
	}
}
