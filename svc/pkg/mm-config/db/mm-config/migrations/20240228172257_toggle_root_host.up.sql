CREATE TABLE games (
	game_id UUID PRIMARY KEY,
	host_networking_enabled BOOLEAN NOT NULL DEFAULT FALSE,
	root_user_enabled BOOLEAN NOT NULL DEFAULT FALSE
);

