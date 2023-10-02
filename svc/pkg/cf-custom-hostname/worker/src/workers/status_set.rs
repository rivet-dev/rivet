use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "cf-custom-hostname-status-set")]
async fn worker(
	ctx: &OperationContext<cf_custom_hostname::msg::status_set::Message>,
) -> GlobalResult<()> {
	let identifier = internal_unwrap!(ctx.identifier).as_uuid();

	if backend::cf::custom_hostname::Status::from_i32(ctx.status).is_none() {
		internal_panic!("invalid hostname status");
	}

	sqlx::query(indoc!(
		"
		UPDATE db_cf_custom_hostname.custom_hostnames
		SET status = $1
		WHERE identifier = $2
		"
	))
	.bind(ctx.status)
	.bind(identifier)
	.execute(&ctx.crdb().await?)
	.await?;

	Ok(())
}
