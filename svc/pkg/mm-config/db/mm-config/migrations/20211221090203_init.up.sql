CREATE TABLE game_namespaces (
	namespace_id UUID PRIMARY KEY,
	lobby_count_max INT NOT NULL DEFAULT 128,
	max_players_per_client INT NOT NULL DEFAULT 8,
	max_players_per_client_vpn INT NOT NULL DEFAULT 4,
	max_players_per_client_proxy INT NOT NULL DEFAULT 2,
	max_players_per_client_tor INT NOT NULL DEFAULT 2,
	max_players_per_client_hosting INT NOT NULL DEFAULT 2
);

CREATE TABLE game_versions (
	version_id UUID PRIMARY KEY,  -- References db-game.versions
	captcha_config BYTES
);

CREATE TABLE lobby_groups (
	lobby_group_id UUID PRIMARY KEY,
	version_id uuid NOT NULL REFERENCES game_versions (version_id),

	name_id STRING NOT NULL,
	
	max_players_normal INT NOT NULL,
	max_players_direct INT NOT NULL,
	max_players_party INT NOT NULL,

	runtime BYTES NOT NULL,  -- LobbyRuntime bytes
	runtime_meta BYTES NOT NULL  -- LobbyRuntimeMeta bytes
);

CREATE TABLE lobby_group_regions (
	lobby_group_id UUID NOT NULL REFERENCES lobby_groups (lobby_group_id),
	region_id UUID NOT NULL,
	tier_name_id STRING NOT NULL,
	PRIMARY KEY (lobby_group_id, region_id)
);

CREATE TABLE lobby_group_idle_lobbies (
	lobby_group_id UUID NOT NULL REFERENCES lobby_groups (lobby_group_id),
	region_id UUID NOT NULL,
	min_idle_lobbies INT NOT NULL,
	max_idle_lobbies INT NOT NULL,
	PRIMARY KEY (lobby_group_id, region_id)
);

