use chirp_workflow::prelude::*;
use url::Url;

use super::{
	super::{
		fdb::FDB_VERSION,
		traefik::{
			TUNNEL_CLICKHOUSE_NATIVE_PORT, TUNNEL_CLICKHOUSE_PORT, TUNNEL_CRDB_PORT,
			TUNNEL_NATS_PORT, TUNNEL_PROMETHEUS_PORT, TUNNEL_REDIS_EPHEMERAL_PORT,
			TUNNEL_REDIS_PERSISTENT_PORT, TUNNEL_S3_PORT,
		},
	},
	TUNNEL_API_EDGE_PORT,
};

pub async fn install(config: &rivet_config::Config) -> GlobalResult<String> {
	let provision_config = &config.server()?.rivet.provision()?;

	Ok(include_str!("../../files/rivet_worker_install.sh")
		.replace(
			"__EDGE_SERVER_BINARY_URL__",
			provision_config.edge_server_binary_url.as_ref(),
		)
		.replace("__FDB_VERSION__", FDB_VERSION))
}

pub fn configure(config: &rivet_config::Config) -> GlobalResult<String> {
	let server_config = config.server()?;

	use rivet_config::config::*;
	let edge_config = Root {
		server: Some(Server {
			// TODO: Is this safe?
			jwt: server_config.jwt.clone(),
			tls: server_config.tls.clone(),
			rivet: Rivet {
				namespace: server_config.rivet.namespace.clone(),
				auth: server_config.rivet.auth.clone(),
				api_public: ApiPublic {
					public_origin: Some(server_config.rivet.edge_api_url("___DATACENTER_NAME_ID___")?),
					..server_config.rivet.api_public.clone()
				},
				ui: Ui {
					enable: Some(false),
					..Default::default()
				},
				edge: Some(Edge {
					// Gets replaced by a template later
					cluster_id: Uuid::nil(),
					datacenter_id: Uuid::nil(),
					intercom_endpoint: Url::parse(&format!("http://127.0.0.1:{TUNNEL_API_EDGE_PORT}"))?,
				}),
				..Default::default()
			},
			cockroachdb: CockroachDb {
				url: Url::parse(&format!(
					"postgres://127.0.0.1:{TUNNEL_CRDB_PORT}/postgres?sslmode=require"
				))?,
				..server_config.cockroachdb.clone()
			},
			redis: RedisTypes {
				ephemeral: Redis {
					url: Url::parse(&format!(
						"rediss://127.0.0.1:{TUNNEL_REDIS_EPHEMERAL_PORT}/#insecure",
					))?,
					..server_config.redis.ephemeral.clone()
				},
				persistent: Redis {
					url: Url::parse(&format!(
						"rediss://127.0.0.1:{TUNNEL_REDIS_PERSISTENT_PORT}/#insecure",
					))?,
					..server_config.redis.persistent.clone()
				},
			},
			clickhouse: server_config.clickhouse.as_ref().map(|clickhouse| GlobalResult::Ok(ClickHouse {
				http_url: Url::parse(&format!(
					"https://127.0.0.1:{TUNNEL_CLICKHOUSE_PORT}",
				))?,
				native_url: Url::parse(&format!(
					"clickhouse://127.0.0.1:{TUNNEL_CLICKHOUSE_NATIVE_PORT}",
				))?,
				..clickhouse.clone()
			})).transpose()?,
			prometheus: Some(Prometheus {
				url: Url::parse(&format!(
					"http://127.0.0.1:{TUNNEL_PROMETHEUS_PORT}",
				))?,
			}),

			foundationdb: Some(FoundationDb {
				addresses: Addresses::Dynamic {
					fetch_endpoint: Url::parse(&format!("http://127.0.0.1:{TUNNEL_API_EDGE_PORT}/provision/datacenters/___DATACENTER_ID___/servers?pools=fdb"))?,
				},
				..Default::default()
			}),
			nats: Nats {
				urls: vec![Url::parse(&format!("nats://127.0.0.1:{TUNNEL_NATS_PORT}"))?],
				..server_config.nats.clone()
			},
			s3: S3 {
				endpoint_internal: Url::parse(&format!("http://127.0.0.1:{TUNNEL_S3_PORT}"))?,
				..server_config.s3.clone()
			},
			ip_info: server_config.ip_info.clone(),
			turnstile: server_config.turnstile.clone(),
			linode: server_config.linode.clone(),
			..Default::default()
		}),
	};
	let mut edge_config_json = serde_json::to_value(&edge_config)?;

	// Add placeholders for templating
	edge_config_json["server"]["rivet"]["default_cluster_id"] = "___CLUSTER_ID___".into();
	edge_config_json["server"]["rivet"]["edge"]["cluster_id"] = "___CLUSTER_ID___".into();
	edge_config_json["server"]["rivet"]["edge"]["datacenter_id"] = "___DATACENTER_ID___".into();

	Ok(
		include_str!("../../files/rivet_worker_configure.sh").replace(
			"__RIVET_EDGE_CONFIG__",
			&serde_json::to_string(&edge_config_json)?,
		),
	)
}
