use anyhow::*;
use include_dir::include_dir;
use rivet_migrate::{SqlService, SqlServiceKind};
use rivet_service_manager::{CronConfig, Service, ServiceKind};
use s3_util::S3Bucket;
use std::sync::Arc;

pub type RunConfig = Arc<RunConfigData>;

pub struct RunConfigData {
	pub services: Vec<Service>,
	pub sql_services: Vec<SqlService>,
	pub s3_buckets: Vec<S3Bucket>,
}

impl RunConfigData {
	/// Replaces an existing service. Throws an error if cannot find service.
	pub fn replace_service(&mut self, service: Service) -> Result<()> {
		let old_len = self.services.len();
		self.services.retain(|x| x.name != service.name);
		ensure!(
			self.services.len() < old_len,
			"could not find instance of service {} to replace",
			service.name
		);
		self.services.push(service);
		Ok(())
	}
}

pub fn config(rivet_config: rivet_config::Config) -> Result<RunConfigData> {
	let server_config = rivet_config.server().map_err(|err| anyhow!("{err:?}"))?;

	let mut services = vec![
		// API
		Service::new(
			"api_monolith_public",
			ServiceKind::ApiPublic,
			|config, pools| Box::pin(api_monolith_public::start(config, pools)),
		),
		Service::new(
			"api_monolith_edge",
			ServiceKind::ApiEdge,
			|config, pools| Box::pin(api_monolith_edge::start(config, pools)),
		),
		Service::new("pegboard_ws", ServiceKind::ApiEdge, |config, pools| {
			Box::pin(pegboard_ws::start(config, pools))
		}),
		Service::new(
			"monolith_worker",
			ServiceKind::Standalone,
			|config, pools| Box::pin(monolith_worker::start(config, pools)),
		),
		Service::new(
			"monolith_workflow_worker",
			ServiceKind::Standalone,
			|config, pools| Box::pin(monolith_workflow_worker::start(config, pools)),
		),
		Service::new("pegboard_gc", ServiceKind::Singleton, |config, pools| {
			Box::pin(pegboard_gc::start(config, pools))
		}),
		Service::new("pegboard_dc_init", ServiceKind::Oneshot, |config, pools| {
			Box::pin(pegboard_dc_init::start(config, pools))
		}),
		Service::new(
			"workflow_metrics_publish",
			ServiceKind::Singleton,
			|config, pools| Box::pin(workflow_metrics_publish::start(config, pools)),
		),
		Service::new("workflow_gc", ServiceKind::Singleton, |config, pools| {
			Box::pin(workflow_gc::start(config, pools))
		}),
		Service::new("mm_gc", ServiceKind::Singleton, |config, pools| {
			Box::pin(mm_gc::start(config, pools))
		}),
		Service::new(
			"build_default_create",
			ServiceKind::Oneshot,
			|config, pools| Box::pin(build_default_create::start(config, pools)),
		),
		Service::new(
			"user_delete_pending",
			ServiceKind::Cron(CronConfig {
				run_immediately: false,
				schedule: "0 0 0 * * *".into(),
			}),
			|config, pools| Box::pin(user_delete_pending::start(config, pools)),
		),
		Service::new(
			"cluster_metrics_publish",
			ServiceKind::Singleton,
			|config, pools| Box::pin(cluster_metrics_publish::start(config, pools)),
		),
		Service::new("cluster_gc", ServiceKind::Singleton, |config, pools| {
			Box::pin(cluster_gc::start(config, pools))
		}),
		Service::new(
			"cluster_default_update",
			ServiceKind::Oneshot,
			|config, pools| Box::pin(cluster_default_update::start(config, pools)),
		),
		Service::new(
			"pegboard_metrics_publish",
			ServiceKind::Singleton,
			|config, pools| Box::pin(pegboard_metrics_publish::start(config, pools)),
		),
	];

	if server_config.is_tls_enabled() {
		services.push(Service::new(
			"cluster_datacenter_tls_renew",
			ServiceKind::Singleton,
			|config, pools| Box::pin(cluster_datacenter_tls_renew::start(config, pools)),
		));
	}

	if server_config.rivet.auth.access_kind == rivet_config::config::rivet::AccessKind::Development
	{
		services.push(Service::new(
			"cloud_default_create",
			ServiceKind::Oneshot,
			|config, pools| Box::pin(cloud_default_create::start(config, pools)),
		));
	}

	if server_config.linode.is_some() {
		services.push(Service::new(
			"linode_gc",
			ServiceKind::Singleton,
			|config, pools| Box::pin(linode_gc::start(config, pools)),
		));
	}

	if server_config.nomad.is_some() {
		services.push(Service::new(
			"nomad_monitor",
			ServiceKind::Singleton,
			|config, pools| Box::pin(nomad_monitor::start(config, pools)),
		));
	}

	if server_config.nomad.is_some() && server_config.rivet.job_run.is_some() {
		services.push(Service::new(
			"job_gc",
			ServiceKind::Singleton,
			|config, pools| Box::pin(job_gc::start(config, pools)),
		));
	}

	if server_config.rivet.telemetry.enable {
		services.push(Service::new(
			"telemetry_beacon",
			ServiceKind::Cron(CronConfig {
				run_immediately: true,
				schedule: "0 0 * * * *".into(),
			}),
			|config, pools| Box::pin(telemetry_beacon::start(config, pools)),
		));
	}

	let sql_services = vec![
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/build/db/build"),
			db_name: "db_build",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/captcha/db/captcha"),
			db_name: "db_captcha",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/cdn/db/cdn"),
			db_name: "db_cdn",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../services/cf-custom-hostname/db/cf-custom-hostname"
			),
			db_name: "db_cf_custom_hostname",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/cloud/db/cloud"),
			db_name: "db_cloud",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/cluster/db/cluster"),
			db_name: "db_cluster",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../services/custom-user-avatar/db/custom-avatar"
			),
			db_name: "db_custom_avatar",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/ds/db/servers"),
			db_name: "db_ds",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../services/dynamic-config/db/dynamic-config"
			),
			db_name: "db_dynamic_config",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../services/email-verification/db/email-verification"
			),
			db_name: "db_email_verification",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/game/db/game"),
			db_name: "db_game",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/ip/db/info"),
			db_name: "db_ip_info",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/job/db/config"),
			db_name: "db_job_config",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/job/db/state"),
			db_name: "db_job_state",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/linode/db/linode"),
			db_name: "db_linode",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/mm-config/db/mm-config"),
			db_name: "db_mm_config",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/mm/db/state"),
			db_name: "db_mm_state",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/pegboard/db/pegboard"),
			db_name: "db_pegboard",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../services/team-invite/db/team-invite"
			),
			db_name: "db_team_invite",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/team/db/team"),
			db_name: "db_team",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/token/db/token"),
			db_name: "db_token",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/upload/db/upload"),
			db_name: "db_upload",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../services/user-identity/db/user-identity"
			),
			db_name: "db_user_identity",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/user/db/user"),
			db_name: "db_user",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/workflow/db/workflow"),
			db_name: "db_workflow",
		},
		SqlService {
			kind: SqlServiceKind::ClickHouse,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/ds-log/db/log"),
			db_name: "db_ds_log",
		},
		SqlService {
			kind: SqlServiceKind::ClickHouse,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../services/job-log/db/log"),
			db_name: "db_job_log",
		},
	];

	let s3_buckets = vec![
		S3Bucket {
			name: "bucket-build",
		},
		S3Bucket { name: "bucket-cdn" },
		S3Bucket {
			name: "bucket-export",
		},
		S3Bucket {
			name: "bucket-banner",
		},
		S3Bucket {
			name: "bucket-logo",
		},
		S3Bucket {
			name: "bucket-artifacts",
		},
		S3Bucket {
			name: "bucket-export",
		},
		S3Bucket { name: "bucket-log" },
		S3Bucket {
			name: "bucket-svc-build",
		},
		S3Bucket {
			name: "bucket-lobby-history-export",
		},
		S3Bucket { name: "bucket-log" },
		S3Bucket {
			name: "bucket-avatar",
		},
		S3Bucket {
			name: "bucket-billing",
		},
		S3Bucket {
			name: "bucket-avatar",
		},
	];

	Ok(RunConfigData {
		services,
		sql_services,
		s3_buckets,
	})
}
