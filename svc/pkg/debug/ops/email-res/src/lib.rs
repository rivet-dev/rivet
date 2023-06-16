use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "debug-email-res")]
async fn handle(
	ctx: OperationContext<debug::email_res::Request>,
) -> GlobalResult<debug::email_res::Response> {
	let verification_id = internal_unwrap!(ctx.verification_id).as_uuid();

	unimplemented!("needs to be migrated to cockroach")

	// let (code,) = ctx
	// 	.scylla("db-email-verification")
	// 	.await?
	// 	.query(
	// 		indoc!(
	// 			"
	// 			SELECT code from verifications
	// 			WHERE verification_id = ?
	// 			"
	// 		),
	// 		(verification_id,),
	// 	)
	// 	.await?
	// 	.first_row_typed::<(String,)>()?;

	// Send code and verification id
	// Ok(debug::email_res::Response { code: code.clone() })
}
