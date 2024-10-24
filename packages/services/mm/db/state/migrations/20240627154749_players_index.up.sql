-- For mm-lobby-cleanup
CREATE INDEX ON players (lobby_id, remove_ts);

-- For mm-lobby-runtime-aggregate
CREATE INDEX ON lobbies (create_ts) STORING (namespace_id, region_id, stop_ts);