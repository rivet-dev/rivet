use anyhow::*;
use deno_core::{serde_v8::GlobalValue, v8};
use pegboard::protocol;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsMetadata {
	pub actor: JsMetadataActor,
	pub project: JsMetadataProject,
	pub environment: JsMetadataEnvironment,
	pub region: JsMetadataRegion,
	pub cluster: JsMetadataCluster,
	pub build: JsMetadataBuild,
}

impl JsMetadata {
	pub fn from_actor(
		metadata: protocol::ActorMetadata,
		scope: &mut v8::HandleScope<'_>,
	) -> Result<Self> {
		Ok(JsMetadata {
			actor: JsMetadataActor {
				id: metadata.actor.actor_id,
				tags: metadata.actor.tags,
				created_at: {
					let date = v8::Local::from(
						v8::Date::new(scope, metadata.actor.create_ts as f64)
							.context("bad date")?,
					);

					v8::Global::new(scope, date).into()
				},
			},
			project: JsMetadataProject {
				id: metadata.project.project_id,
				slug: metadata.project.slug,
			},
			environment: JsMetadataEnvironment {
				id: metadata.environment.env_id,
				slug: metadata.environment.slug,
			},
			region: JsMetadataRegion {
				id: metadata.datacenter.name_id,
				name: metadata.datacenter.display_name,
			},
			cluster: JsMetadataCluster {
				id: metadata.cluster.cluster_id,
			},
			build: JsMetadataBuild {
				id: metadata.build.build_id,
			},
		})
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsMetadataActor {
	pub id: Uuid,
	pub tags: protocol::HashableMap<String, String>,
	pub created_at: GlobalValue, // v8::Date
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsMetadataProject {
	pub id: Uuid,
	pub slug: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsMetadataEnvironment {
	pub id: Uuid,
	pub slug: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsMetadataRegion {
	pub id: String,
	pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsMetadataCluster {
	pub id: Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsMetadataBuild {
	pub id: Uuid,
}
