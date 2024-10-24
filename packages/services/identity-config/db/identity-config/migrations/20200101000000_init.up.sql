CREATE TABLE game_versions (
    version_id UUID PRIMARY KEY  -- References db-game.versions
);

CREATE TABLE game_namespaces (
    namespace_id UUID PRIMARY KEY  -- References db-game.versions
);

CREATE TABLE custom_display_names (
	version_id UUID NOT NULL REFERENCES game_versions (version_id),
	display_name STRING NOT NULL,
	UNIQUE (version_id, display_name)
);

CREATE TABLE custom_avatars (
	version_id UUID NOT NULL REFERENCES game_versions (version_id),
	upload_id UUID NOT NULL,
	UNIQUE (version_id, upload_id)
);
