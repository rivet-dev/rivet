use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[derive(Debug, sqlx::FromRow)]
struct LobbyRow {
	stop_ts: Option<i64>,
	run_id: Option<Uuid>,
}

#[worker(name = "mm-lobby-stop")]
async fn worker(ctx: &OperationContext<mm::msg::lobby_stop::Message>) -> GlobalResult<()> {
	let lobby_id = internal_unwrap!(ctx.lobby_id).as_uuid();

	let crdb = ctx.crdb().await?;

	// Fetch the lobby.
	//
	// This also ensures that mm-lobby-find or mm-lobby-create
	// has already inserted the row and prevents race conditions.
	let lobby_row = sqlx::query_as::<_, LobbyRow>(indoc!(
		"
		WITH
			select_lobby AS (
				SELECT stop_ts, run_id
				FROM db_mm_state.lobbies
				WHERE lobby_id = $1
			),
			_update AS (
				UPDATE db_mm_state.lobbies
				SET stop_ts = $2
				WHERE lobby_id = $1 AND stop_ts IS NULL
				RETURNING 1
			)
		SELECT * FROM select_lobby
		"
	))
	.bind(lobby_id)
	.bind(ctx.ts())
	.fetch_optional(&crdb)
	.await?;
	tracing::info!(?lobby_row, "lobby row");

	let Some(lobby_row) = lobby_row else {
		if ctx.req_dt() > util::duration::minutes(5) {
			tracing::error!("discarding stale message");
			return Ok(());
		} else {
			retry_panic!("lobby not found, may be race condition with insertion");
		}
	};

	// conflicting locks on the lobby row
	// Cleanup the lobby ASAP.
	//
	// This will also be called in `job-run-cleanup`, but this is idempotent.
	msg!([ctx] mm::msg::lobby_cleanup(lobby_id) {
		lobby_id: Some(lobby_id.into()),
	})
	.await?;

	// Stop the job. This will call cleanup and delete the lobby row.
	if let Some(run_id) = lobby_row.run_id {
		msg!([ctx] job_run::msg::stop(run_id) {
			run_id: Some(run_id.into()),
			..Default::default()
		})
		.await?;
	}

	Ok(())
}
