CREATE TABLE lobbies (
	lobby_id UUID PRIMARY KEY,
	namespace_id UUID NOT NULL,
	region_id UUID NOT NULL,
	lobby_group_id UUID NOT NULL,
	run_id UUID,
	max_players_normal INT NOT NULL,
	max_players_direct INT NOT NULL,
	max_players_party INT NOT NULL,
	token_session_id UUID,
	create_ts INT NOT NULL,
	preemptive_create_ts INT,
	ready_ts INT,
	stop_ts INT,
	is_closed BOOL NOT NULL,
	create_ray_id UUID NOT NULL,

	-- Store the static key data in one family
	FAMILY f1 (lobby_id, namespace_id, region_id, lobby_group_id),

	INDEX (namespace_id, create_ts DESC),

	-- Optimized for mm-lobby-runtime-aggregate
	INDEX (namespace_id, stop_ts DESC) STORING (region_id, lobby_group_id, create_ts),

	INDEX (run_id)
);

CREATE TABLE find_queries (
	query_id UUID PRIMARY KEY,
	namespace_id UUID NOT NULL,
	join_kind INT NOT NULL,  -- backend::matchmaker::query::JoinKind
	lobby_id UUID NOT NULL REFERENCES lobbies,
	lobby_auto_created BOOL NOT NULL,
	status INT NOT NULL,
	error_code INT  -- Nullable
);

CREATE TABLE players (
	player_id UUID PRIMARY KEY,
	lobby_id UUID NOT NULL REFERENCES lobbies,
	create_ts INT NOT NULL,
	register_ts INT,
	remove_ts INT,
	token_session_id UUID NOT NULL,
	create_ray_id UUID NOT NULL,
	find_query_id UUID REFERENCES find_queries,
	remote_address TEXT,
	INDEX (lobby_id)
);

