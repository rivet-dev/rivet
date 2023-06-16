use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "chat-last-read-ts-set")]
async fn worker(ctx: OperationContext<chat::msg::last_read_ts_set::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb("db-chat").await?;

	let user_id = internal_unwrap!(ctx.user_id).as_uuid();
	let thread_id = internal_unwrap!(ctx.thread_id).as_uuid();

	let res = sqlx::query(indoc!(
		"
		INSERT INTO thread_user_settings (user_id, thread_id, last_read_ts)
		VALUES ($1, $2, $3)
		ON CONFLICT (user_id, thread_id) DO UPDATE
		SET last_read_ts = $3
		WHERE EXCLUDED.last_read_ts < $3
		"
	))
	.bind(user_id)
	.bind(thread_id)
	.bind(ctx.last_read_ts)
	.execute(&crdb)
	.await?;

	// Send chat read event if the new timestamp is after the stored timestamp
	if res.rows_affected() > 0 {
		tracing::info!("updated last read ts");

		msg!([ctx] chat::msg::last_read_ts_update(user_id, thread_id) {
			user_id: ctx.user_id,
			thread_id: ctx.thread_id,
			read_ts: ctx.last_read_ts,
		})
		.await?;
	} else {
		tracing::info!("last read ts outdated");
	}

	Ok(())
}
