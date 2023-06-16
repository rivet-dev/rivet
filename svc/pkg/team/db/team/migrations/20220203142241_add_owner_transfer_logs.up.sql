CREATE TABLE team_owner_transfer_logs (
    team_id UUID NOT NULL,
    old_owner_user_id UUID NOT NULL,
    new_owner_user_id UUID NOT NULL,
	transfer_ts INT NOT NULL
);