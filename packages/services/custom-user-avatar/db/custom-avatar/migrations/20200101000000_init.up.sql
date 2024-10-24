CREATE TABLE custom_avatars (
    upload_id UUID PRIMARY KEY,  -- References db-upload.uploads
    game_id UUID NOT NULL,  -- References db-game.games
    create_ts INT NOT NULL,
    UNIQUE (game_id, upload_id)
);
