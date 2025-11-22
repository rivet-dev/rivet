use rivet_runner_protocol as protocol;

pub fn event_actor_id(event: &protocol::mk2::Event) -> &str {
	match event {
		protocol::mk2::Event::EventActorIntent(protocol::mk2::EventActorIntent {
			actor_id,
			..
		}) => actor_id,
		protocol::mk2::Event::EventActorStateUpdate(protocol::mk2::EventActorStateUpdate {
			actor_id,
			..
		}) => actor_id,
		protocol::mk2::Event::EventActorSetAlarm(protocol::mk2::EventActorSetAlarm {
			actor_id,
			..
		}) => actor_id,
	}
}

pub fn event_actor_id_mk1(event: &protocol::Event) -> &str {
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
