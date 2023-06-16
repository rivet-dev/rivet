use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "mm-lobby-job-run-cleanup")]
async fn worker(ctx: OperationContext<job_run::msg::cleanup::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb("db-mm-state").await?;

	let run_id = internal_unwrap!(ctx.run_id).as_uuid();

	let lobby_row = sqlx::query_as::<_, (Uuid,)>("SELECT lobby_id FROM lobbies WHERE run_id = $1")
		.bind(run_id)
		.fetch_optional(&crdb)
		.await?;

	if let Some((lobby_id,)) = lobby_row {
		msg!([ctx] mm::msg::lobby_cleanup(lobby_id) {
			lobby_id: Some(lobby_id.into()),
		})
		.await?;
	}

	Ok(())
}
