use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "mm-lobby-job-run-cleanup")]
async fn worker(ctx: &OperationContext<job_run::msg::cleanup::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb().await?;

	let run_id = unwrap_ref!(ctx.run_id).as_uuid();

	let lobby_row = sql_fetch_optional!(
		[ctx, (Uuid,)]
		"SELECT lobby_id FROM db_mm_state.lobbies WHERE run_id = $1",
		run_id,
	)
	.await?;

	if let Some((lobby_id,)) = lobby_row {
		msg!([ctx] mm::msg::lobby_cleanup(lobby_id) {
			lobby_id: Some(lobby_id.into()),
		})
		.await?;
	}

	Ok(())
}
