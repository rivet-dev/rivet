use include_dir::{include_dir, Dir};

#[derive(Clone, Debug)]
pub struct SqlService {
	pub kind: SqlServiceKind,
	pub migrations: Dir<'static>,
	pub db_name: &'static str,
}

#[derive(Clone, Debug)]
pub enum SqlServiceKind {
	CockroachDB,
	ClickHouse,
}

const SQL_SERVICES: &[SqlService] = &[
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/build/db/build"),
		db_name: "db_build",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/captcha/db/captcha"),
		db_name: "db_captcha",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/cdn/db/cdn"),
		db_name: "db_cdn",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/cf-custom-hostname/db/cf-custom-hostname"),
		db_name: "db_cf_custom_hostname",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/cloud/db/cloud"),
		db_name: "db_cloud",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/cluster/db/cluster"),
		db_name: "db_cluster",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/custom-user-avatar/db/custom-avatar"),
		db_name: "db_custom_avatar",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/ds/db/servers"),
		db_name: "db_ds",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/email-verification/db/email-verification"),
		db_name: "db_email_verification",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/game-user/db/game-user"),
		db_name: "db_game_user",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/game/db/game"),
		db_name: "db_game",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/identity-config/db/identity-config"),
		db_name: "db_identity_config",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/ip/db/info"),
		db_name: "db_ip_info",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/job/db/config"),
		db_name: "db_job_config",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/job/db/state"),
		db_name: "db_job_state",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/kv-config/db/kv-config"),
		db_name: "db_kv_config",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/kv/db/kv"),
		db_name: "db_kv",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/linode/db/linode"),
		db_name: "db_linode",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/mm-config/db/mm-config"),
		db_name: "db_mm_config",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/mm/db/state"),
		db_name: "db_mm_state",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/pegboard/db/pegboard"),
		db_name: "db_pegboard",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/team-invite/db/team-invite"),
		db_name: "db_team_invite",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/team/db/team"),
		db_name: "db_team",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/token/db/token"),
		db_name: "db_token",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/upload/db/upload"),
		db_name: "db_upload",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/user-dev/db/user-dev"),
		db_name: "db_user_dev",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/user-follow/db/user-follow"),
		db_name: "db_user_follow",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/user-identity/db/user-identity"),
		db_name: "db_user_identity",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/user-report/db/user-report"),
		db_name: "db_user_report",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/user/db/user"),
		db_name: "db_user",
	},
	SqlService {
		kind: SqlServiceKind::CockroachDB,
		migrations: include_dir!("svc/pkg/workflow/db/workflow"),
		db_name: "db_workflow",
	},
	SqlService {
		kind: SqlServiceKind::ClickHouse,
		migrations: include_dir!("svc/pkg/ds-log/db/log"),
		db_name: "db_ds_log",
	},
	SqlService {
		kind: SqlServiceKind::ClickHouse,
		migrations: include_dir!("svc/pkg/job-log/db/log"),
		db_name: "db_job_log",
	},
];

pub fn get_service(name: &str) -> Option<&'static SqlService> {
	SQL_SERVICES.iter().filter(|x| x.db_name == name).next()
}

pub fn get_services(names: &[&str]) -> Vec<&'static SqlService> {
	SQL_SERVICES
		.iter()
		.filter(|x| names.iter().any(|y| x.db_name == *y))
		.collect::<Vec<_>>()
}

pub fn get_all_services() -> Vec<&'static SqlService> {
	SQL_SERVICES.iter().collect()
}
