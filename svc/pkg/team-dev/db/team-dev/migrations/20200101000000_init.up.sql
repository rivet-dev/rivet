CREATE TABLE dev_teams (
    team_id UUID PRIMARY KEY,
    customer_id STRING NOT NULL,
    UNIQUE (team_id, customer_id)
);
