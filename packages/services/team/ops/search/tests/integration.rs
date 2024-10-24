use chirp_worker::prelude::*;
use proto::backend::pkg::*;

/// Generates a short random string used to make the search results unique.
fn gen_part() -> String {
	let mut rng = rand::thread_rng();
	std::iter::repeat_with(|| {
		let idx = rng.gen_range(0..util::faker::IDENT_CHARSET_ALPHANUM.len());
		util::faker::IDENT_CHARSET_ALPHANUM[idx] as char
	})
	.take(8)
	.collect::<String>()
}

async fn create_team(ctx: &TestCtx, display_name: Option<String>) -> (Uuid, String, String) {
	// to ensure name-uniqueness
	let part1 = match display_name {
		Some(x) => x,
		None => gen_part(),
	};

	let part2 = gen_part();

	let team_id = Uuid::new_v4();
	msg!([ctx] team::msg::create(team_id) -> team::msg::create_complete {
		team_id: Some(team_id.into()),
		owner_user_id: Some(Uuid::new_v4().into()),
		display_name: format!("{} {}", part1, part2),
	})
	.await
	.unwrap();

	(team_id, part1, part2)
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	let rand_str = gen_part();

	let (_, _, part2) = create_team(&ctx, Some(rand_str.clone())).await;
	create_team(&ctx, Some(rand_str.clone())).await;

	let res = op!([ctx] team_search {
		query: rand_str.clone(),
		limit: 1,
		anchor: None,
	})
	.await
	.unwrap();

	assert_eq!(1, res.team_ids.len());

	let res = op!([ctx] team_search {
		query: rand_str.clone(),
		limit: 5,
		anchor: None,
	})
	.await
	.unwrap();

	assert_eq!(2, res.team_ids.len());

	let res = op!([ctx] team_search {
		query: part2,
		limit: 10,
		anchor: None,
	})
	.await
	.unwrap();

	assert_eq!(1, res.team_ids.len());
}
