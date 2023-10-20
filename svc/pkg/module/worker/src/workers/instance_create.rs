use chirp_worker::prelude::*;
use futures_util::{StreamExt, TryStreamExt};
use proto::backend::{self, pkg::*};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashSet, time::Duration};

#[derive(Serialize)]
struct Variables<T> {
	input: T,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateAppInput {
	name: String,
	organization_id: String,
	preferred_region: String,
	machines: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateReleaseInput {
	app_id: String,
	image: String,
	platform_version: String,
	strategy: String,
	definition: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryBody<T> {
	query: &'static str,
	variables: Variables<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateAppPayload {
	app: App,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct App {
	id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum GraphQLResponse<T> {
	#[serde(rename = "data")]
	Data(T),
	#[serde(rename = "errors")]
	Errors(Vec<serde_json::Value>),
}

impl<T> GraphQLResponse<T> {
	fn data(self) -> GlobalResult<T> {
		match self {
			GraphQLResponse::Data(data) => Ok(data),
			GraphQLResponse::Errors(errors) => {
				tracing::error!(?errors, "graphql errors");
				bail!("graphql errors")
			}
		}
	}
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateAppResponse {
	create_app: CreateAppPayload,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct FlyMachine {
	id: String,
	instance_id: String,
	name: String,
	checks: Vec<CheckStatus>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct CheckStatus {
	name: String,
	output: Option<String>,
	status: String,
}

#[worker(name = "module-instance-create")]
async fn worker(
	ctx: &OperationContext<module::msg::instance_create::Message>,
) -> Result<(), GlobalError> {
	let crdb = ctx.crdb().await?;

	let (Ok(fly_org), Ok(fly_region)) = (
		std::env::var("FLY_ORGANIZATION_ID"),
		std::env::var("FLY_REGION"),
	) else {
		bail!("fly not enabled")
	};
	let fly_auth_token = util::env::read_secret(&["fly", "auth_token"]).await?;

	let instance_id = unwrap_ref!(ctx.instance_id).as_uuid();
	let version_id = unwrap_ref!(ctx.module_version_id).as_uuid();

	// Read module version
	let versions = op!([ctx] module_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;
	let version = unwrap!(versions.versions.first());
	let module_id = unwrap!(version.module_id).as_uuid();

	// Create transaction
	rivet_pools::utils::crdb::tx(&crdb, |tx| {
		Box::pin(insert_instance(tx, ctx.ts(), (**ctx).clone()))
	})
	.await?;

	// Create Fly app
	if matches!(
		ctx.driver,
		Some(module::msg::instance_create::message::Driver::Fly(_))
	) {
		// Create app
		// TODO: Handle failure
		let image = match unwrap_ref!(version.image) {
			backend::module::version::Image::Docker(docker) => docker.image_tag.clone(),
		};
		let app_id = launch_app(LaunchAppOpts {
			organization_id: fly_org.clone(),
			name: build_app_name(instance_id),
			preferred_region: fly_region.clone(),
			auth_token: fly_auth_token.clone(),
			image: image.clone(),
		})
		.await?;

		// Update app ID
		sqlx::query(indoc!(
			"
			UPDATE db_module.instances_driver_fly
			SET fly_app_id = $2
			WHERE instance_id = $1
			"
		))
		.bind(instance_id)
		.bind(app_id)
		.execute(&crdb)
		.await?;
	}

	// TODO: Find a 2PC system for releasing the app

	msg!([ctx] module::msg::instance_create_complete(instance_id) {
		instance_id: ctx.instance_id,
	})
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "module.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"module_id": module_id,
					"module_instance_id": unwrap_ref!(ctx.instance_id).as_uuid(),
					"module_version_id": unwrap_ref!(ctx.module_version_id).as_uuid(),
				}))?),
				..Default::default()
			},
		],
	})
	.await?;

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn insert_instance(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	now: i64,
	msg: module::msg::instance_create::Message,
) -> GlobalResult<()> {
	let instance_id = unwrap_ref!(msg.instance_id).as_uuid();
	let version_id = unwrap_ref!(msg.module_version_id).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO db_module.instances (instance_id, version_id, create_ts)
		VALUES ($1, $2, $3)
		"
	))
	.bind(instance_id)
	.bind(version_id)
	.bind(now)
	.execute(&mut **tx)
	.await?;

	match unwrap_ref!(msg.driver) {
		module::msg::instance_create::message::Driver::Dummy(_) => {
			sqlx::query(indoc!(
				"
                INSERT INTO db_module.instances_driver_dummy (instance_id)
                VALUES ($1)
                "
			))
			.bind(instance_id)
			.execute(&mut **tx)
			.await?;
		}
		module::msg::instance_create::message::Driver::Fly(_) => {
			sqlx::query(indoc!(
				"
                INSERT INTO db_module.instances_driver_fly (instance_id)
                VALUES ($1)
                "
			))
			.bind(instance_id)
			.execute(&mut **tx)
			.await?;
		}
	}

	Ok(())
}

#[derive(Clone)]
struct LaunchAppOpts {
	organization_id: String,
	name: String,
	preferred_region: String,
	auth_token: String,
	image: String,
}

#[tracing::instrument(skip(opts))]
async fn launch_app(opts: LaunchAppOpts) -> GlobalResult<String> {
	// Create app
	tracing::info!(organization_id = ?opts.organization_id, name = ?opts.name, preferred_region = ?opts.preferred_region, "creating fly app");
	let res = graphql_request::<_, CreateAppResponse>(
		&opts.auth_token,
		indoc!(
			r#"
				mutation CreateApp($input: CreateAppInput!) {
					createApp(input: $input) {
						app {
							id
						}
					}
				}
				"#
		),
		json!({
			"organizationId": opts.organization_id,
			"name": opts.name,
			"preferredRegion": opts.preferred_region,
			"machines": true,
		}),
	)
	.await?;
	let app_id = res.create_app.app.id;

	// Create ipv6
	tracing::info!("creating ipv6");
	graphql_request::<_, serde_json::Value>(
		&opts.auth_token,
		indoc!(
			r#"
			mutation($input: AllocateIPAddressInput!) {
				allocateIpAddress(input: $input) {
					ipAddress {
						id
						address
						type
						region
						createdAt
					}
				}
			}
			"#
		),
		json!({
			"appId": app_id,
			"type": "v6"
		}),
	)
	.await?;

	// Create shared ipv4
	tracing::info!("creating shared ipv4");
	graphql_request::<_, serde_json::Value>(
		&opts.auth_token,
		indoc!(
			r#"
			mutation($input: AllocateIPAddressInput!) {
				allocateIpAddress(input: $input) {
					app {
						sharedIpAddress
					}
				}
			}
			"#
		),
		json!({
			"appId": app_id,
			"type": "shared_v4"
		}),
	)
	.await?;

	// Create machines in parallel
	let machines = futures_util::stream::iter(0..2)
		.map({
			let opts = opts.clone();
			move |_| Ok(create_machine(opts.clone()))
		})
		.try_buffer_unordered(8)
		.try_collect::<Vec<_>>()
		.await?;
	let machine_ids = machines
		.iter()
		.map(|x| x.id.clone())
		.collect::<HashSet<_>>();

	// Wait for machines to show up in list
	//
	// We need to check this in order to prevent a race condition with deploying a new version when
	// the machines are not registered yet. Deploying a new version relies on the machine list to
	// know what machines to update.
	let mut attempts = 0;
	loop {
		tokio::time::sleep(Duration::from_secs(1)).await;
		attempts += 1;

		tracing::info!(?attempts, "listing machines");
		let res = reqwest::Client::new()
			.get(format!(
				"https://api.machines.dev/v1/apps/{}/machines",
				opts.name
			))
			.bearer_auth(&opts.auth_token)
			.send()
			.await?
			.error_for_status()?
			.json::<Vec<FlyMachine>>()
			.await?;

		// Check if machine exists
		let listed_machine_ids = res.iter().map(|x| x.id.clone()).collect::<HashSet<_>>();
		if listed_machine_ids.is_superset(&machine_ids) {
			tracing::info!(?attempts, "machine lists match");
			break;
		} else {
			tracing::info!(
				?listed_machine_ids,
				?machine_ids,
				"machines not in list yet"
			)
		}

		// Tick
		if attempts > 15 {
			tracing::warn!("machine did not show up in list endpoint soon enough");
			break;
		}
	}

	// Wait for proxy to respond to health checks
	//
	// We don't use native health checks since that doesn't reflect if the proxy acknowledges the
	// machines yet.
	let mut attempts = 0;
	loop {
		tokio::time::sleep(Duration::from_millis(250)).await;
		attempts += 1;

		tracing::info!(?attempts, "checking health");
		let res = reqwest::Client::new()
			.get(format!("https://{}.fly.dev/healthz", opts.name))
			.bearer_auth(&opts.auth_token)
			.send()
			.await;
		match res {
			Ok(x) => match x.error_for_status() {
				Ok(_) => {
					tracing::info!(?attempts, "health check passed");
					break;
				}
				Err(err) => {
					tracing::info!("health check request failed: {}", err);
				}
			},
			Err(err) => {
				tracing::info!("health check request failed: {}", err);
			}
		}

		// TODO: Fail worker & roll back if checks do not pass
		// Tick
		if attempts > 15_000 / 250 {
			tracing::warn!("health check did not pass");
			break;
		}
	}

	Ok(app_id)
}

#[tracing::instrument(skip(opts))]
async fn create_machine(opts: LaunchAppOpts) -> GlobalResult<FlyMachine> {
	tracing::info!("creating machine");

	let config = util_module::fly::MachineConfig { image: &opts.image }.build_machine_config();

	// Create machine
	let machine = reqwest::Client::new()
		.post(format!(
			"https://api.machines.dev/v1/apps/{}/machines",
			opts.name
		))
		.bearer_auth(&opts.auth_token)
		.json(&json!({
			"region": opts.preferred_region,
			"config": config,
		}))
		.send()
		.await?
		.error_for_status()?
		.json::<FlyMachine>()
		.await?;
	tracing::info!(?machine, "machine created");

	// Wait for machine to start
	tracing::info!("waiting for machine to start");
	reqwest::Client::new()
		.get(format!(
			"https://api.machines.dev/v1/apps/{}/machines/{}/wait?instance_id={}&timeout=45&state=started",
			opts.name, machine.id, machine.instance_id
		))
		.bearer_auth(&opts.auth_token)
		.send()
		.await?
		.error_for_status()?;
	tracing::info!("machine started");

	// We run our own manual health checks. Even if these health checks show as passing, the proxy
	// may not register the machine yet.
	// // Wait for health checks to pass
	// // https://github.com/superfly/flyctl/blob/903ee7d4e3bb5b85c535d05fccae9db94ddcd7b5/api/machine_types.go#L380
	// let mut attempts = 0;
	// loop {
	// 	tokio::time::sleep(Duration::from_secs(1)).await;
	// 	attempts += 1;

	// 	tracing::info!(?attempts, id = ?machine.id, "checking health");
	// 	let res = reqwest::Client::new()
	// 		.get(format!(
	// 			"https://api.machines.dev/v1/apps/{}/machines/{}",
	// 			opts.name, machine.id
	// 		))
	// 		.bearer_auth(&opts.auth_token)
	// 		.send()
	// 		.await?
	// 		.error_for_status()?
	// 		.json::<FlyMachine>()
	// 		.await?;
	// 	tracing::info!(checks = ?res.checks, "health checks");

	// 	// Check health checks
	// 	if res.checks.iter().all(|x| x.status == "pass") {
	// 		tracing::info!("machine health checks passed");
	// 		break;
	// 	} else {
	// 		tracing::info!("machine health checks not passed yet")
	// 	}

	// 	// Tick
	// 	if attempts > 15 {
	// 		tracing::warn!("machine health checks did not pass soon enough");
	// 		break;
	// 	}
	// }

	Ok(machine)
}

fn build_app_name(instance_id: Uuid) -> String {
	format!("{}-mod-{}", util::env::namespace(), instance_id)
}

async fn graphql_request<Req: Serialize, Res: DeserializeOwned>(
	auth_token: &str,
	query: &'static str,
	req: Req,
) -> GlobalResult<Res> {
	let res = reqwest::Client::new()
		.post("https://api.fly.io/graphql")
		.bearer_auth(auth_token)
		.json(&QueryBody {
			query,
			variables: Variables { input: req },
		})
		.send()
		.await?
		.error_for_status()?
		.json::<GraphQLResponse<Res>>()
		.await?
		.data()?;

	Ok(res)
}
