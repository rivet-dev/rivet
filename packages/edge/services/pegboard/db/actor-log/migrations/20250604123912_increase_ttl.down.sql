ALTER TABLE actor_logs
  MODIFY TTL toDate(ts + toIntervalDay(3));
