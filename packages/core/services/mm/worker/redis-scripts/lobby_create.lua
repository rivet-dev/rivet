local ts = tonumber(ARGV[1])
local lobby_id = ARGV[2]
local lobby_config = cjson.decode(ARGV[3])
local ready_expire_ts = tonumber(ARGV[4])

local key_lobby_config = KEYS[1]
local key_ns_lobby_ids = KEYS[2]
local key_lobby_available_spots_normal = KEYS[3]
local key_lobby_available_spots_party = KEYS[4]
local key_lobby_unready = KEYS[5]
local key_idle_lobby_ids = KEYS[6]
local key_idle_lobby_lobby_group_ids = KEYS[7]
local key_lobby_player_ids = KEYS[8]

local player_count = redis.call('ZCARD', key_lobby_player_ids);
for k, v in pairs(lobby_config) do
	redis.call('HSET', key_lobby_config, k, tostring(v))
end
redis.call('ZADD', key_ns_lobby_ids, ts, lobby_id)
redis.call('ZADD', key_lobby_available_spots_normal, lobby_config['mpn'] - player_count, lobby_id)
redis.call('ZADD', key_lobby_available_spots_party, lobby_config['mpp'] - player_count, lobby_id)
redis.call('ZADD', key_lobby_unready, tonumber(ready_expire_ts), lobby_id)

-- Add to idle lobbies if needed
local is_custom = lobby_config['cu']
if player_count == 0 and not is_custom then
	redis.call('ZADD', key_idle_lobby_ids, ts, lobby_id)
	redis.call('HSET', key_idle_lobby_lobby_group_ids, lobby_id, lobby_config['lg'])
end

