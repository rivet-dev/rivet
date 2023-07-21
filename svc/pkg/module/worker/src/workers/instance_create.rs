use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize)]
struct Variables<T> {
	input: T,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateAppInput {
	organization_id: String,
	name: String,
	preferred_region: String,
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

	// Create app
	// TODO: Handle failure
	let image = match internal_unwrap!(version.image) {
		backend::module::version::Image::Docker(docker) => docker.image_tag.clone(),
	};
	let app_id = launch_app(LaunchAppOpts {
		organization_id: &fly_org,
		name: &format!("module-instance-{}", instance_id),
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

struct LaunchAppOpts<'a> {
	organization_id: &'a str,
	name: &'a str,
	preferred_region: &'a str,
	auth_token: &'a str,
	image: &'a str,
}

async fn launch_app(opts: LaunchAppOpts<'_>) -> GlobalResult<String> {
	// Create app
	tracing::info!(organization_id = ?opts.organization_id, name = ?opts.name, preferred_region = ?opts.preferred_region, "creating fly app");
	let res = reqwest::Client::new()
		.post("https://api.fly.io/graphql")
		.bearer_auth(opts.auth_token)
		.json(&QueryBody {
			query: indoc!(
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
			variables: Variables {
				input: CreateAppInput {
					organization_id: opts.organization_id.into(),
					name: opts.name.into(),
					preferred_region: opts.preferred_region.into(),
				},
			},
		})
		.send()
		.await?
		.error_for_status()?
		.json::<GraphQLResponse<CreateAppResponse>>()
		.await?
		.data()?;
	let app_id = res.create_app.app.id;

	// Deploy version
	tracing::info!(?app_id, image = ?opts.image, "creating fly release");
	let res = reqwest::Client::new()
		.post("https://api.fly.io/graphql")
		.bearer_auth(opts.auth_token)
		.json(&QueryBody {
			query: indoc!(
				r#"
				mutation CreateRelease($input: CreateReleaseInput!) {
					createRelease(input: $input) {
						app {
							id
						}
					}
				}
				"#
			),
			variables: Variables {
				input: CreateReleaseInput {
					app_id: app_id.clone(),
					image: opts.image.into(),
					platform_version: "machines".into(),
					strategy: "CANARY".into(),
					definition: json!({}),
				},
			},
		})
		.send()
		.await?
		.error_for_status()?
		.json::<GraphQLResponse<serde_json::Value>>()
		.await?
		.data()?;

	Ok(app_id)
}
