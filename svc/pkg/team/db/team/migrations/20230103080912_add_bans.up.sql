CREATE TABLE banned_users (
    team_id UUID NOT NULL,
    user_id UUID NOT NULL,
    ban_ts INT NOT NULL,
    PRIMARY KEY (team_id, user_id)
);
