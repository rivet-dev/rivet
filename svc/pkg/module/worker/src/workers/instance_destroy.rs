use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "module-instance-destroy")]
async fn worker(
	ctx: &OperationContext<module::msg::instance_destroy::Message>,
) -> Result<(), GlobalError> {
	let crdb = ctx.crdb("db-module").await?;

	let instance_id = internal_unwrap!(ctx.instance_id).as_uuid();

	let instances = op!([ctx] module_instance_get {
		instance_ids: vec![instance_id.into()],

	})
	.await?;
	let instance = internal_unwrap_owned!(instances.instances.first());

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
	.bind(ctx.ts())
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

	reqwest::Client::new()
		.delete(format!("https://api.machines.dev/v1/apps/{app_id}",))
		.bearer_auth(&fly_auth_token)
		.send()
		.await?
		.error_for_status()?;

	Ok(())
}
