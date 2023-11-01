use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

// TODO: Check in the case of an unfollow, check if the user was actually following before the unfollow so we
// do not send a `user_follow::msg::delete` message
#[operation(name = "user-follow-toggle")]
async fn handle(
	ctx: OperationContext<user_follow::toggle::Request>,
) -> GlobalResult<user_follow::toggle::Response> {
	let follower_user_id = unwrap_ref!(ctx.follower_user_id).as_uuid();
	let following_user_id = unwrap_ref!(ctx.following_user_id).as_uuid();

	ensure!(follower_user_id != following_user_id, "cannot follow self");

	let crdb = ctx.crdb().await?;
	let mutual = if ctx.active {
		tokio::try_join!(
			sql_query!(
				[ctx, &crdb]
				"
				INSERT INTO db_user_follow.user_follows
				(follower_user_id, following_user_id, create_ts, ignored)
				VALUES ($1, $2, $3, false)
				",
				follower_user_id,
				following_user_id,
				util::timestamp::now(),
			),
			// Along with creating a new follow, ignore the following user's follow (if it exists). This
			// ensures that:
			// - if the following user has followed the follower user first,
			// - and the follower user follows then unfollows,
			// the original following user's follow won't show up in the follower user's "recent follows"
			// list again.
			sql_query!(
				[ctx, &crdb]
				"
				UPDATE db_user_follow.user_follows
				SET ignored = TRUE
				WHERE
					follower_user_id = $1 AND
					following_user_id = $2
				",
				following_user_id,
				follower_user_id,
				util::timestamp::now(),
			),
		)?;

		// Check for mutuality after creating record
		check_mutual(&crdb, follower_user_id, following_user_id).await?
	} else {
		// Check for mutuality before deleting record
		let mutual = check_mutual(&crdb, follower_user_id, following_user_id).await?;

		sql_query!(
			[ctx]
			"DELETE FROM db_user_follow.user_follows WHERE follower_user_id = $1 AND following_user_id = $2",
			follower_user_id,
			following_user_id,
		)
		.await?;

		mutual
	};

	if ctx.active {
		msg!([ctx] user_follow::msg::create(follower_user_id, following_user_id) {
			follower_user_id: Some(follower_user_id.into()),
			following_user_id: Some(following_user_id.into()),
			is_mutual: mutual,
		})
		.await?;

		// Users have become mutuals
		if mutual {
			msg!([ctx] user::msg::mutual_follow_create(follower_user_id) {
				user_a_id: Some(follower_user_id.into()),
				user_b_id: Some(following_user_id.into()),
			})
			.await?;
			msg!([ctx] user::msg::mutual_follow_create(following_user_id) {
				user_a_id: Some(following_user_id.into()),
				user_b_id: Some(follower_user_id.into()),
			})
			.await?;
		}

		msg!([ctx] analytics::msg::event_create() {
			events: vec![ analytics::msg::event_create::Event {
				name: "user_follow.create".into(),
				user_id: Some(follower_user_id.into()),
				properties_json: Some(serde_json::to_string(&json!({
					"follower": follower_user_id,
					"following": following_user_id,
					"became_mutual": mutual,
				}))?),
				..Default::default()
			} ],
		})
		.await?;
	} else {
		msg!([ctx] user_follow::msg::delete(follower_user_id, following_user_id) {
			follower_user_id: Some(follower_user_id.into()),
			following_user_id: Some(following_user_id.into()),
		})
		.await?;

		// Users stop being mutuals after follow deletion
		if mutual {
			msg!([ctx] user::msg::mutual_follow_delete(follower_user_id) {
				user_a_id: Some(follower_user_id.into()),
				user_b_id: Some(following_user_id.into()),
			})
			.await?;
			msg!([ctx] user::msg::mutual_follow_delete(following_user_id) {
				user_a_id: Some(following_user_id.into()),
				user_b_id: Some(follower_user_id.into()),
			})
			.await?;
		}

		msg!([ctx] analytics::msg::event_create() {
			events: vec![analytics::msg::event_create::Event {
				name: "user_follow.delete".into(),
				user_id: Some(follower_user_id.into()),
				properties_json: Some(serde_json::to_string(&json!({
					"follower": follower_user_id,
					"following": following_user_id,
					"was_mutual": mutual,
				}))?),
				..Default::default()
			}],
		})
		.await?;
	}

	Ok(user_follow::toggle::Response {})
}

async fn check_mutual(
	crdb: &CrdbPool,
	follower_user_id: Uuid,
	following_user_id: Uuid,
) -> GlobalResult<bool> {
	let res = sqlx::query_as::<_, (i64,)>(indoc!(
		"
		SELECT 1
			FROM db_user_follow.user_follows
			WHERE
				(follower_user_id = $1 AND following_user_id = $2) OR
				(follower_user_id = $2 AND following_user_id = $1)
		LIMIT 2
		"
	))
	.bind(follower_user_id)
	.bind(following_user_id)
	.fetch_all(crdb)
	.await?;

	Ok(res.len() == 2)
}
