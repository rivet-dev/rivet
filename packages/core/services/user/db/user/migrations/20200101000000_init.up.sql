CREATE TABLE users (
    user_id UUID PRIMARY KEY,
    display_name STRING NOT NULL,
    display_name_len INT AS (char_length(display_name)) STORED,
    account_number INT NOT NULL,
    avatar_id STRING NOT NULL,
    join_ts INT NOT NULL,
    UNIQUE (display_name, account_number),
    INDEX (display_name, display_name_len ASC, account_number ASC)  -- Used for user search  -- TODO: Use trigrams when available: https://github.com/cockroachdb/cockroach/issues/41285
);

CREATE TABLE user_presences (
    user_id UUID PRIMARY KEY,
    status INT NOT NULL DEFAULT 0,
    update_ts INT NOT NULL DEFAULT 0,
    game_activity BYTES,  -- Nullable
    custom_activity BYTES  -- Nullable
);
