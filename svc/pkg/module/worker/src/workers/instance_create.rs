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
struct LaunchAppInput {
	organization_id: String,
	name: String,
	regions: Vec<String>,
	image: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryBody {
	query: &'static str,
	variables: Variables<LaunchAppInput>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchAppPayload {
	app: App,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct App {
	id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphQLResponse {
	data: GraphQLResponseInner,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphQLResponseInner {
	launch_app: LaunchAppPayload,
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
	let app_id = launch_app(
		LaunchAppInput {
			organization_id: fly_org,
			name: format!("module-instance-{}", instance_id),
			regions: vec![fly_region],
			image,
		},
		&fly_auth_token,
	)
	.await?;

	// Update app ID
	sqlx::query(indoc!(
		"
		UPDATE instances
		SET app_id = $2
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

async fn launch_app(input: LaunchAppInput, auth_token: &str) -> GlobalResult<String> {
	let query_body = QueryBody {
		query: indoc!(
			"
			mutation LaunchApp($input: LaunchAppInput!) {
				launchApp(input: $input) {
					app {
						id
					}
				}
			}
			"
		),
		variables: Variables { input },
	};

	let res = reqwest::Client::new()
		.post("https://api.fly.io/graphql")
		.bearer_auth(auth_token)
		.json(&query_body)
		.send()
		.await?
		.json::<GraphQLResponse>()
		.await?;

	Ok(res.data.launch_app.app.id)
}
