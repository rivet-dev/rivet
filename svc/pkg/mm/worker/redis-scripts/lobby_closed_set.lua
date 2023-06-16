local lobby_id = ARGV[1]
local max_players_normal = tonumber(ARGV[2])
local max_players_party = tonumber(ARGV[3])

local key_lobby_config = KEYS[1]
local key_lobby_player_ids = KEYS[2]
local key_lobby_available_spots_normal = KEYS[3]
local key_lobby_available_spots_party = KEYS[4]

redis.call('HSET', key_lobby_config, 'c', 0)

local player_count = redis.call('ZCARD', key_lobby_player_ids)
redis.call('ZADD', key_lobby_available_spots_normal, max_players_normal - player_count, lobby_id)
redis.call('ZADD', key_lobby_available_spots_party, max_players_party - player_count, lobby_id)

