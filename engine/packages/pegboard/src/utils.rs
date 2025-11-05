use rivet_runner_protocol as protocol;
use rivet_types::{
	keys::namespace::runner_config::RunnerConfigVariant,
	runner_configs::{RunnerConfig, RunnerConfigKind},
};

pub fn event_actor_id(event: &protocol::Event) -> &str {
	match event {
		protocol::Event::EventActorIntent(protocol::EventActorIntent { actor_id, .. }) => actor_id,
		protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
			actor_id,
			..
		}) => actor_id,
		protocol::Event::EventActorSetAlarm(protocol::EventActorSetAlarm { actor_id, .. }) => {
			actor_id
		}
	}
}

pub fn event_generation(event: &protocol::Event) -> u32 {
	match event {
		protocol::Event::EventActorIntent(protocol::EventActorIntent { generation, .. }) => {
			*generation
		}
		protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
			generation,
			..
		}) => *generation,
		protocol::Event::EventActorSetAlarm(protocol::EventActorSetAlarm {
			generation, ..
		}) => *generation,
	}
}

pub fn runner_config_variant(runner_config: &RunnerConfig) -> RunnerConfigVariant {
	match runner_config.kind {
		RunnerConfigKind::Normal { .. } => RunnerConfigVariant::Normal,
		RunnerConfigKind::Serverless { .. } => RunnerConfigVariant::Serverless,
	}
}
