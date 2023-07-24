use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;
use std::collections::HashMap;
use std::convert::TryInto;

// A note on gradual deploys:
//
// We reuse the same Fly app for each unique namespace ID & key combination
// in order ot ensure version changes are made safely.
//
// We don't use namespace ID & module IDs because there might be multiple of the same module per namespace.
//
// We don't use namespace ID & version IDs because we want a gradual deploy when changing the version.

#[derive(Debug, sqlx::FromRow)]
struct NamespaceInstances {
	key: String,
	instance_id: Uuid,
}

#[worker(name = "module-ns-version-set")]
async fn worker(
	ctx: OperationContext<game::msg::ns_version_set_complete::Message>,
) -> Result<(), GlobalError> {
	let crdb = ctx.crdb("db-module").await?;

	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();
	let version_id = internal_unwrap!(ctx.version_id).as_uuid();

	// TODO: Transaction

	// Get version config
	let game_versions = op!([ctx] module_game_version_get {
		version_ids: vec![version_id.into()],
	})
	.await?;
	let game_version = internal_unwrap_owned!(game_versions.versions.first());
	let new_version_keys = game_version
		.module_dependencies
		.iter()
		.map(|x| x.key.as_str())
		.collect::<HashSet<&str>>();

	// Find all existing instances for ns
	let existing_instances = sqlx::query_as::<_, NamespaceInstances>(indoc!(
		"
		SELECT key, instance_id
		FROM namespace_instances
		WHERE namespace_id = $1
		"
	))
	.bind(namespace_id)
	.fetch_all(&crdb)
	.await?;
	let current_version_keys = existing_instances
		.iter()
		.map(|x| x.key.as_str())
		.collect::<HashSet<&str>>();

	// New instances
	let new_dep_keys = new_version_keys
		.difference(&current_version_keys)
		.collect::<Vec<_>>();
	for dep_key in new_dep_keys {
		let version_id = game_version
			.module_dependencies
			.iter()
			.find(|x| x.key == *key)
			.map(|x| x.version_id)
			.as_uuid();
		create_instances(ctx.chirp(), &crdb, namespace_id, dep_key, version_id).await?;
	}

	// Update instances
	let update_dep_keys = new_version_keys
		.intersection(&current_version_keys)
		.collect::<Vec<_>>();

	// Delete instances
	let delete_dep_keys = current_version_keys
		.difference(&new_version_keys)
		.collect::<Vec<_>>();

	Ok(())
}

async fn create_instances(
	client: &chirp_client::Client,
	crdb: &CrdbPool,
	namespace_id: Uuid,
	dep_key: &str,
	version_id: Uuid,
) -> Result<(), GlobalError> {
	// Create instance
	let instance_id = Uuid::new_v4();
	msg!([client] module::msg::instance_create(instance_id) -> module::msg::instance_create_complete {
		instance_id: Some(instance_id.into()),
		module_version_id: Some(version_id.into()),
		driver: Some(module::msg::instance_create::message::Driver::Fly(module::msg::instance_create::message::Fly {})),
	})
	.await
	.unwrap();

	// Insert instance
	sqlx::query(indoc!(
		"
		INSERT INTO namespace_instances (namespace_id, key, instance_id)
		VALUES ($1, $2, $3)
		"
	))
	.bind(namespace_id)
	.bind(key)
	.bind(version_id)
	.execute(crdb)
	.await?;

	Ok(())
}
