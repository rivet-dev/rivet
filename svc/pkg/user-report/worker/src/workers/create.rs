use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "user-report-create")]
async fn worker(ctx: &OperationContext<user_report::msg::create::Message>) -> GlobalResult<()> {
	let reporter_user_id = internal_unwrap!(ctx.reporter_user_id).as_uuid();
	let subject_user_id = internal_unwrap!(ctx.subject_user_id).as_uuid();
	let namespace_id = ctx.subject_user_id.as_ref().map(common::Uuid::as_uuid);

	sqlx::query(indoc!(
		"
		INSERT INTO db_user_report.user_reports (
			reporter_user_id,
			subject_user_id,
			namespace_id,
			create_ts,
			reason
		)
		VALUES ($1, $2, $3, $4, $5)
		"
	))
	.bind(reporter_user_id)
	.bind(subject_user_id)
	.bind(namespace_id)
	.bind(ctx.ts())
	.bind(ctx.reason.as_ref())
	.execute(&ctx.crdb().await?)
	.await?;

	Ok(())
}
