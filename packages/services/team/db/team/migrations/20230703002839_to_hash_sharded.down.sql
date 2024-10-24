DROP INDEX teams_create_ts_idx1;
CREATE INDEX ON teams (create_ts DESC);
