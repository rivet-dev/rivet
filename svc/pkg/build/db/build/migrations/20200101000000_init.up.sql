CREATE TABLE builds (
    build_id UUID PRIMARY KEY,
    game_id UUID NOT NULL,  -- References db-game.games
    upload_id UUID NOT NULL,  -- References db-upload.uploads
    display_name TEXT NOT NULL,
    create_ts INT NOT NULL,
    UNIQUE (game_id, upload_id)
);
