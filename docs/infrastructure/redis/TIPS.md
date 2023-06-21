# Redis tips

## Key boundaries

Redis operations are parallelized in a cluster, so design keys around minimizing the frequency of their use/how often they get locked.

Consider how many keys will be locked for each operation and design them like you would design a `RwLock`. Similarly, if running a Lua script on the database, you should only use the keys passed in to the script.

Using a hash to store multiple properties about one entity instead of multiple keys is more efficient (at the tradeoff of being limited in your data structure). i.e. using `lobby:{lobby_id}:config` instead of multiple keys: `lobby:{lobby_id}:max_players_normal` and `lobby:{lobby_id}:max_players_party`.

## Prefer sets over counters

Counters are incredibly error prone if one Redis request fails and often has negative cascading effects. Sets are more reliable and enable idempotent operations.

For example: instead of using `INCR lobby:{lobby_id}:player_count`, prefer `SADD lobby:{lobby_id}:player_ids {player_id}` paired with `SCARD lobby:{lobby_id}:player_count`.

## Use sorted sets for acknowledged expirations

Sorted sets are efficient for finding expired objects.

For example: since we can't ackhowledge the expiration of a key with `EXPIRE players:{player_id}`, we can use `ZADD players:unregisered {expirt_ts} {player_id}` then find all expired players with `ZRANGE players:unregisered 0 {ts} BYSCORE`.
