CREATE TABLE access_tokens (
    name TEXT PRIMARY KEY,
    user_id UUID NOT NULL,  -- References db-users.users
    create_ts INT NOT NULL
);
