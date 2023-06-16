CREATE TABLE games (
    game_id UUID PRIMARY KEY,
    create_ts INT NOT NULL,
    name_id STRING NOT NULL UNIQUE,
    display_name STRING NOT NULL,
    url STRING NOT NULL,
    developer_team_id UUID NOT NULL,
    description STRING NOT NULL
);

CREATE TABLE game_tags (
    game_id UUID NOT NULL,
    tag STRING NOT NULL,
    PRIMARY KEY (game_id, tag),
    INDEX (tag)
);

CREATE TABLE game_versions (
    version_id UUID PRIMARY KEY,
    game_id UUID NOT NULL REFERENCES games (game_id),
    create_ts INT NOT NULL,
    display_name STRING NOT NULL,
    INDEX (version_id)
);

CREATE TABLE game_namespaces (
    namespace_id UUID PRIMARY KEY,
    game_id UUID NOT NULL REFERENCES games (game_id),
    create_ts INT NOT NULL,
    display_name STRING NOT NULL,
    version_id UUID NOT NULL REFERENCES game_versions (version_id),
    INDEX (game_id),
    INDEX (version_id)
);

