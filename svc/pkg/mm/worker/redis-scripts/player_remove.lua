local key_player_config = KEYS[1]
local key_ns_player_ids = KEYS[2]
local key_lobby_player_ids = KEYS[3]
local key_lobby_registered_player_ids = KEYS[4]
local key_player_unregistered = KEYS[5]
local key_player_auto_remove = KEYS[6]
local key_lobby_available_spots_normal = KEYS[7]
local key_lobby_available_spots_party = KEYS[8]
local key_ns_remote_address_player_ids = KEYS[9]
local key_idle_lobby_ids = KEYS[10]
local key_idle_lobby_lobby_group_ids = KEYS[11]
local key_lobby_config = KEYS[12]

local ts = tonumber(ARGV[1])
local lobby_id = ARGV[2]
local player_id = ARGV[3]
local lobby_group_id = ARGV[4]
local max_players_normal = tonumber(ARGV[5])
local max_players_party = tonumber(ARGV[6])
local auto_remove_lobby = ARGV[7] == '1'

-- Remove the player
redis.call('DEL', key_player_config)
redis.call('ZREM', key_ns_player_ids, player_id)
redis.call('ZREM', key_lobby_player_ids, player_id)
redis.call('ZREM', key_lobby_registered_player_ids, player_id)
redis.call('ZREM', key_player_unregistered, player_id)
redis.call('ZREM', key_player_auto_remove, player_id)
if key_ns_remote_address_player_ids ~= "" then
	redis.call('SREM', key_ns_remote_address_player_ids, player_id)
end

if redis.call('EXISTS', key_lobby_config) == 1 then
	local player_count = redis.call('ZCARD', key_lobby_player_ids)

	-- Update available spots
	if redis.call('HGET', key_lobby_config, 'c') ~= '1' then
		redis.call('ZADD', key_lobby_available_spots_normal, max_players_normal - player_count, lobby_id)
		redis.call('ZADD', key_lobby_available_spots_party, max_players_party - player_count, lobby_id)
	end

	-- Remove the lobby if empty
	if player_count == 0 then
		if auto_remove_lobby then
			-- Remove lobby and prevent it from being seeked again
			redis.call('ZREM', key_lobby_available_spots_normal, lobby_id)
			redis.call('ZREM', key_lobby_available_spots_party, lobby_id)

			return { true, false }
		else
			-- Mark lobby as idle
			local did_set_idle = redis.call('ZADD', key_idle_lobby_ids, ts, lobby_id) == 1
			redis.call('HSET', key_idle_lobby_lobby_group_ids, lobby_id, lobby_group_id)

			return { false, did_set_idle }
		end
	else
		-- Normal player removing, nothing to do

		return { false, false }
	end
else
	-- Lobby does not exist
	return { false, false }
end
