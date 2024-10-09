CREATE TABLE game_config (
	game_id UUID PRIMARY KEY,
	host_networking_enabled BOOLEAN NOT NULL DEFAULT FALSE,
	root_user_enabled BOOLEAN NOT NULL DEFAULT FALSE,
	runtime INT NOT NULL, -- ds::types::GameRuntime
);
