use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "user-report-create")]
async fn worker(ctx: &OperationContext<user_report::msg::create::Message>) -> GlobalResult<()> {
	let reporter_user_id = unwrap_ref!(ctx.reporter_user_id).as_uuid();
	let subject_user_id = unwrap_ref!(ctx.subject_user_id).as_uuid();
	let namespace_id = ctx.subject_user_id.as_ref().map(common::Uuid::as_uuid);

	sql_query!(
		[ctx]
		"
		INSERT INTO db_user_report.user_reports (
			reporter_user_id,
			subject_user_id,
			namespace_id,
			create_ts,
			reason
		)
		VALUES ($1, $2, $3, $4, $5)
		",
		reporter_user_id,
		subject_user_id,
		namespace_id,
		ctx.ts(),
		ctx.reason.as_ref(),
	)
	.await?;

	Ok(())
}
