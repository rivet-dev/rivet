CREATE TABLE sites (
    site_id UUID PRIMARY KEY,
    game_id UUID NOT NULL,  -- References db-game.games
    upload_id UUID NOT NULL,  -- References db-upload.uploads
    display_name TEXT NOT NULL,
    create_ts INT NOT NULL,
    UNIQUE (game_id, upload_id)
);

CREATE TABLE game_versions (
    version_id UUID PRIMARY KEY,  -- References db-game.versions
    site_id UUID NOT NULL REFERENCES sites (site_id)
);
