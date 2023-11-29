use rivet_util as util;

pub const LOBBY_READY_TIMEOUT: i64 = util::duration::minutes(5);
pub const PLAYER_READY_TIMEOUT: i64 = util::duration::minutes(2);
pub const PLAYER_AUTO_REMOVE_TIMEOUT: i64 = util::duration::hours(8);

pub const MIN_HOST_PORT: u16 = 26000;
pub const MAX_HOST_PORT: u16 = 31999;

/// Constants used for mocking responses when using dev tokens.
pub const DEV_REGION_ID: &str = "dev-lcl";
pub const DEV_PROVIDER_NAME: &str = "Development";
pub const DEV_REGION_NAME: &str = "Local";

// Also see svc/mm-lobby-create/src/nomad_job.rs
pub const DEFAULT_ENV_KEYS: &[&str] = &[
	"RIVET_API_ENDPOINT",
	"RIVET_CHAT_API_URL",
	"RIVET_GROUP_API_URL",
	"RIVET_IDENTITY_API_URL",
	"RIVET_KV_API_URL",
	"RIVET_MATCHMAKER_API_URL",
	"RIVET_NAMESPACE_NAME",
	"RIVET_NAMESPACE_ID",
	"RIVET_VERSION_NAME",
	"RIVET_VERSION_ID",
	"RIVET_GAME_MODE_ID",
	"RIVET_GAME_MODE_NAME",
	"RIVET_LOBBY_ID",
	"RIVET_TOKEN",
	"RIVET_REGION_ID",
	"RIVET_REGION_NAME",
	"RIVET_MAX_PLAYERS_NORMAL",
	"RIVET_MAX_PLAYERS_DIRECT",
	"RIVET_MAX_PLAYERS_PARTY",
	"RIVET_LOBBY_TOKEN",
	"RIVET_LOBBY_GROUP_ID",
	"RIVET_LOBBY_GROUP_NAME",
];
