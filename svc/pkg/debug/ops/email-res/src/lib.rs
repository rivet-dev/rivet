use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "debug-email-res")]
async fn handle(
	ctx: OperationContext<debug::email_res::Request>,
) -> GlobalResult<debug::email_res::Response> {
	let crdb = ctx.crdb().await?;
	let verification_id = unwrap_ref!(ctx.verification_id).as_uuid();

	let (code,) = sql_fetch_one!(
		[ctx, (String,)]
		"
		SELECT code from db_email_verification.verifications
		WHERE verification_id = $1
		",
		verification_id,
	)
	.await?;

	// Send code and verification id
	Ok(debug::email_res::Response { code: code.clone() })
}
