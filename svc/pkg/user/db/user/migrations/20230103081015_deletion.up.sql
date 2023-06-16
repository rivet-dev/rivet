ALTER TABLE users ADD COLUMN delete_request_ts INT;
ALTER TABLE users ADD COLUMN delete_complete_ts INT;
CREATE INDEX delete_request_ts_index ON users (delete_request_ts DESC);
