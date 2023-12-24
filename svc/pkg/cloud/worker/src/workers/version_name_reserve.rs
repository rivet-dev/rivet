use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "cloud-version_name_reserve")]
async fn worker(
	ctx: &OperationContext<cloud::msg::version_name_reserve::Message>,
) -> GlobalResult<()> {
	let game_id = unwrap!(ctx.game_id).as_uuid();
	let request_id = unwrap!(ctx.request_id).as_uuid();

	// Generate date
	//
	// Use UTC in order to ensure that the month is consistent if a team is collaborating from
	// multiple locations around the world with different time zones
	let now = chrono::Utc::now();
	let date_prefix = now.format("%Y.%m").to_string();

	let (display_name,) = sql_fetch_one!(
		[ctx, (String,)]
		r#"
		WITH next_version AS (
			-- Increment the highest existing number by 1, or start with 1 if none exist
			SELECT $2 || ' (' || (COALESCE(MAX(SUBSTRING(version_display_name FROM $2 || ' \((\d+)\)')::INT), 0) + 1) || ')' AS new_version
			FROM db_cloud.game_version_name_reservations
			-- Filter for rows with the matching game ID and version format
			WHERE version_display_name ~ $2 || ' \(\d+\)'
			AND game_id = $1
		)
		-- Insert the new version name into the table
		INSERT INTO db_cloud.game_version_name_reservations (game_id, version_display_name, create_ts)
		SELECT $1, new_version, $3
		FROM next_version
		RETURNING version_display_name;
		"#,
		game_id,
		date_prefix,
		ctx.ts(),
	)
	.await?;

	msg!([ctx] cloud::msg::version_name_reserve_complete(game_id, request_id) {
		game_id: ctx.game_id,
		request_id: ctx.request_id,
		version_display_name: display_name,
	})
	.await?;

	Ok(())
}
