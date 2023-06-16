ALTER TABLE game_namespaces ADD COLUMN name_id STRING NOT NULL DEFAULT gen_random_uuid()::STRING;
CREATE UNIQUE INDEX ix_game_namespaces_game_id_name_id ON game_namespaces (game_id, name_id);
