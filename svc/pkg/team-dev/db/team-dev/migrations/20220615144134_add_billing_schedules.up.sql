CREATE TABLE dev_team_billing_schedules (
    team_id UUID PRIMARY KEY,
    billing_day INT NOT NULL,
    UNIQUE (team_id, billing_day)
);
