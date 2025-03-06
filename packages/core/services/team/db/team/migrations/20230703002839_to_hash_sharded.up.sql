DROP INDEX teams_create_ts_idx;
CREATE INDEX ON teams (create_ts DESC) USING HASH;
