CREATE TABLE emails (
    email TEXT PRIMARY KEY,
    user_id UUID NOT NULL,  -- References db-users.users
    create_ts INT NOT NULL
);
