ALTER TABLE lobby_groups
	ADD COLUMN find_config BYTES,
	ADD COLUMN join_config BYTES,
	ADD COLUMN create_config BYTES;
