CREATE TABLE game_versions (
    version_id UUID PRIMARY KEY  -- References db-game.versions
);

CREATE TABLE game_namespaces (
    namespace_id UUID PRIMARY KEY  -- References db-game.versions
);
