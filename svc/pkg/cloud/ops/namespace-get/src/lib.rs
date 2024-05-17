use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct GameNamespace {
	namespace_id: Uuid,
	// No important data here, this is a placeholder for things to come
}

#[operation(name = "cloud-namespace-get")]
async fn handle(
	ctx: OperationContext<cloud::namespace_get::Request>,
) -> GlobalResult<cloud::namespace_get::Response> {
	let namespace_ids = ctx
		.namespace_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let cloud_namespaces = sql_fetch_all!(
		[ctx, GameNamespace]
		"
		SELECT namespace_id
		FROM db_cloud.game_namespaces
		WHERE namespace_id = ANY($1)
		",
		namespace_ids,
	)
	.await?;

	let all_namespace_ids_proto = cloud_namespaces
		.iter()
		.map(|c| c.namespace_id)
		.map(common::Uuid::from)
		.collect::<Vec<_>>();

	// Fetch all dependent configs
	let (cdn_configs_res, mm_configs_res, kv_configs_res, identity_configs_res) = tokio::try_join!(
		op!([ctx] cdn_namespace_get {
			namespace_ids: all_namespace_ids_proto.clone(),
		}),
		op!([ctx] mm_config_namespace_get {
			namespace_ids: all_namespace_ids_proto.clone(),
		}),
		op!([ctx] kv_config_namespace_get {
			namespace_ids: all_namespace_ids_proto.clone(),
		}),
		op!([ctx] identity_config_namespace_get {
			namespace_ids: all_namespace_ids_proto.clone(),
		}),
	)?;

	let namespaces = cloud_namespaces
		.iter()
		.filter_map(|cloud_namespace| {
			let namespace_id_proto = common::Uuid::from(cloud_namespace.namespace_id);
			Some(backend::cloud::GameNamespace {
				namespace_id: Some(namespace_id_proto),
				config: Some(backend::cloud::NamespaceConfig {
					cdn: if let Some(x) = cdn_configs_res
						.namespaces
						.iter()
						.find(|cdn_namespace| {
							cdn_namespace.namespace_id.as_ref() == Some(&namespace_id_proto)
						})
						.map(|v| v.config.clone().unwrap())
					{
						Some(x)
					} else {
						tracing::warn!(namespace_id = %cloud_namespace.namespace_id, "missing cdn for ns");
						return None;
					},
					matchmaker: if let Some(x) = mm_configs_res
						.namespaces
						.iter()
						.find(|mm_namespace| {
							mm_namespace.namespace_id.as_ref() == Some(&namespace_id_proto)
						})
						.map(|v| v.config.clone().unwrap())
					{
						Some(x)
					} else {
						tracing::warn!(namespace_id = %cloud_namespace.namespace_id, "missing mm for ns");
						return None;
					},
					kv: if let Some(x) = kv_configs_res
						.namespaces
						.iter()
						.find(|kv_namespace| {
							kv_namespace.namespace_id.as_ref() == Some(&namespace_id_proto)
						})
						.map(|v| v.config.clone().unwrap())
					{
						Some(x)
					} else {
						tracing::warn!(namespace_id = %cloud_namespace.namespace_id, "missing kv for ns");
						return None;
					},
					identity: if let Some(x) = identity_configs_res
						.namespaces
						.iter()
						.find(|identity_namespace| {
							identity_namespace.namespace_id.as_ref() == Some(&namespace_id_proto)
						})
						.map(|v| v.config.clone().unwrap())
					{
						Some(x)
					} else {
						tracing::warn!(namespace_id = %cloud_namespace.namespace_id, "missing identity for ns");
						return None;
					},
				}),
			})
		})
		.collect::<Vec<_>>();

	Ok(cloud::namespace_get::Response { namespaces })
}
