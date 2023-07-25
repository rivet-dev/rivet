use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;

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
				internal_panic!("graphql errors")
			}
		}
	}
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateAppResponse {
	create_app: CreateAppPayload,
}

#[worker(name = "module-instance-create")]
async fn worker(
	ctx: OperationContext<module::msg::instance_create::Message>,
) -> Result<(), GlobalError> {
	let crdb = ctx.crdb("db-module").await?;

	let (Ok(fly_org), Ok(fly_region)) = (std::env::var("FLY_ORGANIZATION_ID"), std::env::var("FLY_REGION"))  else {
		internal_panic!("fly not enabled")
	};
	let fly_auth_token = util::env::read_secret(&["fly", "auth_token"]).await?;

	let instance_id = internal_unwrap!(ctx.instance_id).as_uuid();
	let version_id = internal_unwrap!(ctx.module_version_id).as_uuid();

	// Read module version
	let versions = op!([ctx] module_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;
	let version = internal_unwrap_owned!(versions.versions.first());
	let module_id = internal_unwrap_owned!(version.module_id).as_uuid();

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
		let image = match internal_unwrap!(version.image) {
			backend::module::version::Image::Docker(docker) => docker.image_tag.clone(),
		};
		let app_id = launch_app(LaunchAppOpts {
			organization_id: &fly_org,
			name: &build_app_name(instance_id),
			preferred_region: &fly_region,
			auth_token: &fly_auth_token,
			image: &image,
		})
		.await?;

		// Update app ID
		sqlx::query(indoc!(
			"
			UPDATE instances_driver_fly
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
					"module_instance_id": internal_unwrap!(ctx.instance_id).as_uuid(),
					"module_version_id": internal_unwrap!(ctx.module_version_id).as_uuid(),
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
	let instance_id = internal_unwrap!(msg.instance_id).as_uuid();
	let version_id = internal_unwrap!(msg.module_version_id).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO instances (instance_id, version_id, create_ts)
		VALUES ($1, $2, $3)
		"
	))
	.bind(instance_id)
	.bind(version_id)
	.bind(now)
	.execute(&mut *tx)
	.await?;

	match internal_unwrap!(msg.driver) {
		module::msg::instance_create::message::Driver::Dummy(_) => {
			sqlx::query(indoc!(
				"
                INSERT INTO instances_driver_dummy (instance_id)
                VALUES ($1)
                "
			))
			.bind(instance_id)
			.execute(&mut *tx)
			.await?;
		}
		module::msg::instance_create::message::Driver::Fly(_) => {
			sqlx::query(indoc!(
				"
                INSERT INTO instances_driver_fly (instance_id)
                VALUES ($1)
                "
			))
			.bind(instance_id)
			.execute(&mut *tx)
			.await?;
		}
	}

	Ok(())
}

#[derive(Copy, Clone)]
struct LaunchAppOpts<'a> {
	organization_id: &'a str,
	name: &'a str,
	preferred_region: &'a str,
	auth_token: &'a str,
	image: &'a str,
}

#[tracing::instrument(skip(opts))]
async fn launch_app(opts: LaunchAppOpts<'_>) -> GlobalResult<String> {
	// Create app
	tracing::info!(organization_id = ?opts.organization_id, name = ?opts.name, preferred_region = ?opts.preferred_region, "creating fly app");
	let res = graphql_request::<_, CreateAppResponse>(
		opts.auth_token,
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
		opts.auth_token,
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
		opts.auth_token,
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

	// Create machines
	for _ in 0..2 {
		create_machine(opts).await?;
	}

	Ok(app_id)
}

#[tracing::instrument(skip(opts))]
async fn create_machine(opts: LaunchAppOpts<'_>) -> GlobalResult<()> {
	tracing::info!("creating machine");

	let config = util_module::fly::MachineConfig { image: opts.image }.build_machine_config();

	#[derive(Deserialize, Debug)]
	#[allow(dead_code)]
	struct FlyMachine {
		id: String,
		instance_id: String,
		name: String,
	}

	// Create machine
	let machine = reqwest::Client::new()
		.post(format!(
			"https://api.machines.dev/v1/apps/{}/machines",
			opts.name
		))
		.bearer_auth(opts.auth_token)
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
		.bearer_auth(opts.auth_token)
		.send()
		.await?
		.error_for_status()?;
	tracing::info!("machine started");

	// Wait for machine to show up in list
	let mut backoff = util::Backoff::new(4, Some(5), 200, 100);
	loop {
		tracing::info!("listing machines");
		let res = reqwest::Client::new()
			.get(format!(
				"https://api.machines.dev/v1/apps/{}/machines",
				opts.name
			))
			.bearer_auth(opts.auth_token)
			.send()
			.await?
			.error_for_status()?
			.json::<Vec<FlyMachine>>()
			.await?;

		// Check if machine exists
		if res.iter().any(|x| x.id == machine.id) {
			tracing::info!("found machine in list");
			break;
		} else {
			tracing::info!("machine not in list yet")
		}

		// Tick
		if backoff.tick().await {
			tracing::warn!("machine did not show up in list endpoint soon enough");
			break;
		}
	}

	Ok(())
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
