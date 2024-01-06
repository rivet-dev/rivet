ALTER TABLE teams
	ADD deactivate_reasons INT[] NOT NULL DEFAULT '{}',
	ADD deactivate_update_ts INT;
