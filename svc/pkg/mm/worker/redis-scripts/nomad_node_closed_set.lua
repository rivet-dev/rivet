local lobby_count = ARGV[1]

for i=1,lobby_count do
	local real_i = i - 1
	local lobby_id = ARGV[real_i * 3 + 2]
	local max_players_normal = tonumber(ARGV[real_i * 3 + 3])
	local max_players_party = tonumber(ARGV[real_i * 3 + 4])
	
	local key_lobby_config = KEYS[real_i * 4 + 2]
	
	redis.call('HSET', key_lobby_config, 'nc', 0)
	
	local is_closed = redis.call('HGET', key_lobby_config, 'c')
	
	-- Don't modify closed lobbies
	if is_closed ~= '1' then
		local key_lobby_player_ids = KEYS[real_i * 4 + 3]
		local key_lobby_available_spots_normal = KEYS[real_i * 4 + 4]
		local key_lobby_available_spots_party = KEYS[real_i * 4 + 5]

		local player_count = redis.call('ZCARD', key_lobby_player_ids)
		redis.call('ZADD', key_lobby_available_spots_normal, max_players_normal - player_count, lobby_id)
		redis.call('ZADD', key_lobby_available_spots_party, max_players_party - player_count, lobby_id)	
	end
end
