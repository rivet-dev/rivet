CREATE TABLE game_version_custom_headers (
    version_id UUID REFERENCES game_versions (version_id),
	glob BYTES NOT NULL,
	priority INT NOT NULL,
    header_name STRING NOT NULL,
	header_value STRING NOT NULL,
    PRIMARY KEY (version_id, glob)
);
