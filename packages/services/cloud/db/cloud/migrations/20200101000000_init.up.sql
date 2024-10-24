CREATE TABLE game_configs (
	game_id UUID PRIMARY KEY  -- References db-game.games
);

CREATE TABLE game_namespaces (
	namespace_id UUID PRIMARY KEY  -- References db-game.game_namespaces
);

CREATE TABLE game_versions (
	version_id UUID PRIMARY KEY  -- References db-game.game_versions
);
