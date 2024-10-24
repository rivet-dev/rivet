CREATE TABLE dev_users (
    user_id UUID PRIMARY KEY
);

CREATE TABLE beta_join_requests (
    user_id UUID PRIMARY KEY,
    ts INT NOT NULL,
    name TEXT NOT NULL,
    company_name TEXT,
    company_size TEXT NOT NULL,
    preferred_tools TEXT NOT NULL,
    goals TEXT NOT NULL
);
