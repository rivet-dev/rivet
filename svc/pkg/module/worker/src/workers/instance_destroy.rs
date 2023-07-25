use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;
use std::collections::HashMap;
use std::convert::TryInto;

#[worker(name = "module-instance-destroy")]
async fn worker(
	ctx: OperationContext<game::msg::instance_destroy::Message>,
) -> Result<(), GlobalError> {
	// Delete app
	match internal_unwrap!(instance.driver) {
		backend::module::instance::Driver::Fly(fly) => {
			let app_id = internal_unwrap!(fly.fly_app_id, "fly machine not started yet");

			delete_fly_app(app_id).await?;
		}
		backend::module::instance::Driver::Dummy(_) => {
			tracing::info!("nothing to do for dummy driver");
		}
	}

	// Update database
	sqlx::query(indoc!(
		"
		UPDATE instances
		SET destroy_ts = $2
		WHERE instance_id = $1
		"
	))
	.bind(instance_id)
	.bind(req.ts())
	.execute(&crdb)
	.await?;

	msg!([ctx] module::msg::instance_destroy_complete(instance_id) {
		instance_id: Some(instance_id.into()),
	})
	.await?;

	Ok(())
}

#[tracing::instrument]
async fn delete_fly_app(app_id: &str) -> GlobalResult<()> {
	let fly_auth_token = util::env::read_secret(&["fly", "auth_token"]).await?;

	tracing::info!("deleting app");

	let machines = reqwest::Client::new()
		.delete(format!("https://api.machines.dev/v1/apps/{app_id}",))
		.bearer_auth(&fly_auth_token)
		.send()
		.await?
		.error_for_status()?;

	Ok(())
}
