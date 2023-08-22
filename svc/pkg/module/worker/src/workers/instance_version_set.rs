use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde::Deserialize;
use serde_json::json;

#[worker(name = "module-instance-version-set")]
async fn worker(
	ctx: OperationContext<module::msg::instance_version_set::Message>,
) -> Result<(), GlobalError> {
	let crdb = ctx.crdb("db-module").await?;

	let instance_id = internal_unwrap!(ctx.instance_id).as_uuid();
	let version_id = internal_unwrap!(ctx.version_id).as_uuid();

	let (instances, versions) = tokio::try_join!(
		op!([ctx] module_instance_get {
			instance_ids: vec![instance_id.into()],

		}),
		op!([ctx] module_version_get {
			version_ids: vec![version_id.into()],
		}),
	)?;
	let instance = internal_unwrap_owned!(instances.instances.first());
	let version = internal_unwrap_owned!(versions.versions.first());

	// Get Docker image
	let image = match internal_unwrap!(version.image) {
		backend::module::version::Image::Docker(docker) => docker.image_tag.as_str(),
	};

	// Update instance
	match internal_unwrap!(instance.driver) {
		backend::module::instance::Driver::Fly(fly) => {
			let app_id = internal_unwrap!(fly.fly_app_id, "fly machine not started yet");

			update_fly_machines(app_id, image).await?;
		}
		backend::module::instance::Driver::Dummy(_) => {
			tracing::info!("nothing to do for dummy driver");
		}
	}

	// Update database
	sqlx::query(indoc!(
		"
		UPDATE instances
		SET version_id = $2
		WHERE instance_id = $1
		"
	))
	.bind(instance_id)
	.bind(version_id)
	.execute(&crdb)
	.await?;

	msg!([ctx] module::msg::instance_version_set_complete(instance_id) {
		instance_id: Some(instance_id.into()),
		version_id: Some(version_id.into()),
	})
	.await?;

	Ok(())
}

#[tracing::instrument]
async fn update_fly_machines(app_id: &str, image: &str) -> GlobalResult<()> {
	let fly_auth_token = util::env::read_secret(&["fly", "auth_token"]).await?;

	let client = reqwest::Client::new();

	tracing::info!("listing machines");

	#[derive(Deserialize, Debug)]
	struct FlyMachine {
		id: String,
		config: FlyMachineConfig,
	}

	#[derive(Deserialize, Debug)]
	struct FlyMachineConfig {
		// TODO: We should be using image_ref instead for more thorough comparisons
		image: String,
	}

	let machines = client
		.get(format!(
			"https://api.machines.dev/v1/apps/{app_id}/machines",
		))
		.bearer_auth(&fly_auth_token)
		.send()
		.await?
		.error_for_status()?
		.json::<Vec<FlyMachine>>()
		.await?;
	tracing::info!(len = machines.len(), ?machines, "fetched machines");

	for machine in &machines {
		if machine.config.image == image {
			tracing::info!(id = ?machine.id, "machine already up to date");
			continue;
		}

		tracing::info!(id = ?machine.id, "updating machine");

		let config = util_module::fly::MachineConfig { image }.build_machine_config();

		reqwest::Client::new()
			.post(format!(
				"https://api.machines.dev/v1/apps/{app_id}/machines/{}",
				machine.id,
			))
			.bearer_auth(&fly_auth_token)
			.json(&json!({
				"config": config,
			}))
			.send()
			.await?
			.error_for_status()?;
	}

	Ok(())
}
