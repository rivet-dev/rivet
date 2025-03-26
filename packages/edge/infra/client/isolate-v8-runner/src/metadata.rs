use std::collections::HashMap;

use anyhow::*;
use deno_core::{serde_v8::GlobalValue, v8};
use pegboard::{protocol, types};
use pegboard_config::isolate_runner as config;
use rivet_api::models;
use rivet_convert::ApiFrom;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsMetadata {
	pub actor: JsMetadataActor,
	pub network: Option<JsMetadataNetwork>,
	pub project: JsMetadataProject,
	pub environment: JsMetadataEnvironment,
	pub region: JsMetadataRegion,
	pub cluster: JsMetadataCluster,
	pub build: JsMetadataBuild,
}

impl JsMetadata {
	pub fn from_actor(
		actor_config: config::actor::Config,
		scope: &mut v8::HandleScope<'_>,
	) -> Result<Self> {
		let metadata = actor_config.metadata.deserialize()?;

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
			network: metadata
				.network
				.map(|network| {
					Ok(JsMetadataNetwork {
						ports: network
							.ports
							.into_iter()
							.map(|(name, mut port)| {
								// Because the actor's original metadata was created before its ports were allocated,
								// we have to modify it to set the ports here. This only applies to host ports.
								if let types::Routing::Host { .. } = port.routing {
									let transformed_port_name =
										pegboard::util::pegboard_normalize_port_name(&name);

									port.public_port = Some(
										actor_config
											.ports
											.get(&transformed_port_name)
											.context("no proxied port found for host port")?
											.target
											.try_into()?,
									);
								}

								Ok((name, models::ActorsPort::api_from(port).into()))
							})
							.collect::<Result<_>>()?,
					})
				})
				.transpose()?,
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
pub struct JsMetadataNetwork {
	pub ports: HashMap<String, CamelCaseActorsPort>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CamelCaseActorsPort {
	protocol: models::ActorsPortProtocol,
	internal_port: Option<i32>,
	hostname: Option<String>,
	port: Option<i32>,
	path: Option<String>,
	url: Option<String>,
	routing: Box<models::ActorsPortRouting>,
}

// Identity conversion from the api model to the camel case struct
impl From<models::ActorsPort> for CamelCaseActorsPort {
	fn from(value: models::ActorsPort) -> CamelCaseActorsPort {
		CamelCaseActorsPort {
			protocol: value.protocol,
			internal_port: value.internal_port,
			hostname: value.hostname,
			port: value.port,
			path: value.path,
			url: value.url,
			routing: value.routing,
		}
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CamelCaseActorsPortRouting {
	pub guard: Option<serde_json::Value>,
	pub host: Option<serde_json::Value>,
}

// Identity conversion from the api model to the camel case struct
impl From<models::ActorsPortRouting> for CamelCaseActorsPortRouting {
	fn from(value: models::ActorsPortRouting) -> CamelCaseActorsPortRouting {
		CamelCaseActorsPortRouting {
			guard: value.guard,
			host: value.host,
		}
	}
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
