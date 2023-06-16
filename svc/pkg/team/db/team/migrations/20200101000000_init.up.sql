CREATE TABLE teams (
    team_id UUID PRIMARY KEY,
    display_name STRING NOT NULL,
    create_ts INT NOT NULL
);

CREATE TABLE team_members (
    team_id UUID NOT NULL,
    user_id UUID NOT NULL,
    join_ts INT NOT NULL,
    PRIMARY KEY (team_id, user_id)
);

