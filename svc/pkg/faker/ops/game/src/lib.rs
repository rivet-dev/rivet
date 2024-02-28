use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "faker-game")]
async fn handle(
	ctx: OperationContext<faker::game::Request>,
) -> GlobalResult<faker::game::Response> {
	let dev_team_id = if let Some(dev_team_id) = &ctx.dev_team_id {
		*dev_team_id
	} else {
		let team_create_res = op!([ctx] faker_team {

			..Default::default()
		})
		.await?;
		*unwrap_ref!(team_create_res.team_id)
	};

	let game_create_res = op!([ctx] game_create {
		name_id: util::faker::ident(),
		display_name: util::faker::display_name(),
		developer_team_id: Some(dev_team_id),
	})
	.await?;

	op!([ctx] cloud_game_config_create {
		game_id: game_create_res.game_id,
	})
	.await?;

	op!([ctx] mm_config_game_upsert {
		game_id: game_create_res.game_id,
		config: Some(backend::matchmaker::GameConfig {
			// Required for testing
			host_networking_enabled: true,
			// Will never be tested
			root_user_enabled: false,
		})
	})
	.await?;

	let mut namespace_ids = Vec::<common::Uuid>::new();
	let mut version_ids = Vec::<common::Uuid>::new();
	if !ctx.skip_namespaces_and_versions {
		let version_create_res = op!([ctx] faker_game_version {
			game_id: game_create_res.game_id,
			..Default::default()
		})
		.await?;
		let version_id = unwrap_ref!(version_create_res.version_id).as_uuid();
		version_ids.push(version_id.into());

		let namespace_name_ids = vec!["prod".to_owned(), "staging".to_owned()];
		for name_id in &namespace_name_ids {
			let ns_create_res = op!([ctx] faker_game_namespace {
				game_id: game_create_res.game_id,
				version_id: version_create_res.version_id,
				override_name_id: name_id.clone(),
				..Default::default()
			})
			.await?;
			namespace_ids.push(*unwrap_ref!(ns_create_res.namespace_id));
		}
	}

	Ok(faker::game::Response {
		game_id: game_create_res.game_id,
		namespace_ids,
		version_ids,
	})
}
