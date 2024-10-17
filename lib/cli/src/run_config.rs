use anyhow::*;
use include_dir::include_dir;
use rivet_migrate::{SqlService, SqlServiceKind};
use rivet_server::{Service, ServiceKind};
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

pub fn default_config() -> Result<RunConfigData> {
	let services = vec![
		// API
		Service::new("api_monolith", ServiceKind::Api, || {
			Box::pin(api_monolith::start())
		}),
		// API internal
		Service::new("api_internal_monolith", ServiceKind::ApiInternal, || {
			Box::pin(api_internal_monolith::start())
		}),
		Service::new("pegboard_ws", ServiceKind::ApiInternal, || {
			Box::pin(pegboard_ws::start())
		}),
		// Standalone
		Service::new("monolith_worker", ServiceKind::Standalone, || {
			Box::pin(monolith_worker::start())
		}),
		Service::new("monolith_workflow_worker", ServiceKind::Standalone, || {
			Box::pin(monolith_workflow_worker::start())
		}),
		// Singleton
		Service::new("pegboard_gc", ServiceKind::Singleton, || {
			Box::pin(pegboard_gc::start())
		}),
		Service::new("nomad_monitor", ServiceKind::Singleton, || {
			Box::pin(nomad_monitor::start())
		}),
		Service::new("cluster_metrics_publish", ServiceKind::Singleton, || {
			Box::pin(cluster_metrics_publish::start())
		}),
		Service::new("cluster_gc", ServiceKind::Singleton, || {
			Box::pin(cluster_gc::start())
		}),
		Service::new(
			"cluster_datacenter_tls_renew",
			ServiceKind::Singleton,
			|| Box::pin(cluster_datacenter_tls_renew::start()),
		),
		Service::new("linode_gc", ServiceKind::Singleton, || {
			Box::pin(linode_gc::start())
		}),
		Service::new("workflow_metrics_publish", ServiceKind::Singleton, || {
			Box::pin(workflow_metrics_publish::start())
		}),
		Service::new("workflow_gc", ServiceKind::Singleton, || {
			Box::pin(workflow_gc::start())
		}),
		Service::new("mm_gc", ServiceKind::Singleton, || Box::pin(mm_gc::start())),
		Service::new("job_gc", ServiceKind::Singleton, || {
			Box::pin(job_gc::start())
		}),
		Service::new("user_delete_pending", ServiceKind::Singleton, || {
			Box::pin(user_delete_pending::start())
		}),
		// Oneshot
		Service::new("build_default_create", ServiceKind::Oneshot, || {
			Box::pin(build_default_create::start())
		}),
		Service::new("pegboard_dc_init", ServiceKind::Oneshot, || {
			Box::pin(pegboard_dc_init::start())
		}),
		Service::new("cluster_default_update", ServiceKind::Oneshot, || {
			Box::pin(cluster_default_update::start(false))
		}),
		Service::new("cluster_workflow_backfill", ServiceKind::Oneshot, || {
			Box::pin(cluster_workflow_backfill::start())
		}),
		// Cron
		Service::new("telemetry_beacon", ServiceKind::Cron, || {
			Box::pin(telemetry_beacon::start())
		}),
		Service::new("user_delete_pending", ServiceKind::Cron, || {
			Box::pin(user_delete_pending::start())
		}),
		// TODO:
		// - load_test_mm_sustain
		// - load_test_mm
		// - load_test_sqlx
		// - load_test_watch_requests
	];

	let sql_services = vec![
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/build/db/build"),
			db_name: "db_build",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/captcha/db/captcha"),
			db_name: "db_captcha",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/cdn/db/cdn"),
			db_name: "db_cdn",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../svc/pkg/cf-custom-hostname/db/cf-custom-hostname"
			),
			db_name: "db_cf_custom_hostname",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/cloud/db/cloud"),
			db_name: "db_cloud",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/cluster/db/cluster"),
			db_name: "db_cluster",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../svc/pkg/custom-user-avatar/db/custom-avatar"
			),
			db_name: "db_custom_avatar",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/ds/db/servers"),
			db_name: "db_ds",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../svc/pkg/email-verification/db/email-verification"
			),
			db_name: "db_email_verification",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/game-user/db/game-user"),
			db_name: "db_game_user",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/game/db/game"),
			db_name: "db_game",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../svc/pkg/identity-config/db/identity-config"
			),
			db_name: "db_identity_config",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/ip/db/info"),
			db_name: "db_ip_info",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/job/db/config"),
			db_name: "db_job_config",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/job/db/state"),
			db_name: "db_job_state",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/kv-config/db/kv-config"),
			db_name: "db_kv_config",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/kv/db/kv"),
			db_name: "db_kv",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/linode/db/linode"),
			db_name: "db_linode",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/mm-config/db/mm-config"),
			db_name: "db_mm_config",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/mm/db/state"),
			db_name: "db_mm_state",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/pegboard/db/pegboard"),
			db_name: "db_pegboard",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../svc/pkg/team-invite/db/team-invite"
			),
			db_name: "db_team_invite",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/team/db/team"),
			db_name: "db_team",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/token/db/token"),
			db_name: "db_token",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/upload/db/upload"),
			db_name: "db_upload",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/user-dev/db/user-dev"),
			db_name: "db_user_dev",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../svc/pkg/user-follow/db/user-follow"
			),
			db_name: "db_user_follow",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../svc/pkg/user-identity/db/user-identity"
			),
			db_name: "db_user_identity",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!(
				"$CARGO_MANIFEST_DIR/../../svc/pkg/user-report/db/user-report"
			),
			db_name: "db_user_report",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/user/db/user"),
			db_name: "db_user",
		},
		SqlService {
			kind: SqlServiceKind::CockroachDB,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/workflow/db/workflow"),
			db_name: "db_workflow",
		},
		SqlService {
			kind: SqlServiceKind::ClickHouse,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/ds-log/db/log"),
			db_name: "db_ds_log",
		},
		SqlService {
			kind: SqlServiceKind::ClickHouse,
			migrations: include_dir!("$CARGO_MANIFEST_DIR/../../svc/pkg/job-log/db/log"),
			db_name: "db_job_log",
		},
	];

	let s3_buckets = vec![
		S3Bucket { name: "build" },
		S3Bucket { name: "cdn" },
		S3Bucket { name: "export" },
		S3Bucket { name: "banner" },
		S3Bucket { name: "logo" },
		S3Bucket { name: "artifacts" },
		S3Bucket { name: "export" },
		S3Bucket { name: "log" },
		S3Bucket {
			name: "imagor-result-storage",
		},
		S3Bucket { name: "svc-build" },
		S3Bucket {
			name: "lobby-history-export",
		},
		S3Bucket { name: "log" },
		S3Bucket { name: "avatar" },
		S3Bucket { name: "billing" },
		S3Bucket { name: "avatar" },
	];

	Ok(RunConfigData {
		services,
		sql_services,
		s3_buckets,
	})
}
