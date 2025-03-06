local ts = tonumber(ARGV[1])
local lobby_id = ARGV[2]
local new_player_expire_ts = ARGV[3]

local key_lobby_config = KEYS[1]
local key_lobby_unready = KEYS[2]
local key_player_unregistered = KEYS[3]
local key_lobby_player_ids = KEYS[4]

-- Set lobby ready ts
redis.call('HSET', key_lobby_config, 'rt', ts)

-- Remove unready timestamp
redis.call('ZREM', key_lobby_unready, lobby_id)

-- Update player expiration timestamps since the player needs to have
-- enough time to register *after* the lobby has readied.
local player_ids = redis.call('ZRANGE', key_lobby_player_ids, 0, -1)
for _, player_id in ipairs(player_ids) do
	redis.call('ZADD', key_player_unregistered, new_player_expire_ts, player_id)
end

