CREATE TABLE join_requests (
    team_id UUID NOT NULL,
    user_id UUID NOT NULL,
    ts INT NOT NULL,
    PRIMARY KEY (team_id, user_id)
);
