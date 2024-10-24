use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let mut team_ids = Vec::<(common::Uuid, String)>::new();

	for _ in 0..4usize {
		let create_res = op!([ctx] faker_team {
			..Default::default()
		})
		.await
		.unwrap();

		let get_res = op!([ctx] team_get {
			team_ids: vec![create_res.team_id.unwrap()],
		})
		.await
		.unwrap();
		let team_data = get_res.teams.first().unwrap();

		team_ids.push((team_data.team_id.unwrap(), team_data.display_name.clone()));
	}

	let mut req_display_names = team_ids
		.iter()
		.map(|(_, display_name)| display_name.clone())
		.collect::<Vec<_>>();
	req_display_names.push(util::faker::display_name()); // Non-existent name

	let res = op!([ctx] team_resolve_display_name {
		display_names: req_display_names,
	})
	.await
	.unwrap();
	assert_eq!(team_ids.len(), res.teams.len());
}
