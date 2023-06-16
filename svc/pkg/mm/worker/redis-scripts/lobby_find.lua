local ts = tonumber(ARGV[1])
local query = cjson.decode(ARGV[2])
local query_kind_key_idx = tonumber(ARGV[3])
local available_spots_key_idx = tonumber(ARGV[4])
local available_spots_key_count = tonumber(ARGV[5])
local player_config_key_idx = tonumber(ARGV[6])
local ns_remote_address_player_ids_key_idx = tonumber(ARGV[7])

local key_find_query_state = KEYS[1]
local key_find_query_player_ids = KEYS[2]
local key_ns_player_ids = KEYS[3]
local key_player_unregistered = KEYS[4]

-- MARK: Find
local lobby_id = nil
local lobby_auto_created = false
if query.kind.direct ~= nil then
	-- MARK: Direct

	local key_direct_lobby_config = KEYS[query_kind_key_idx + 1]
	local key_direct_lobby_player_ids = KEYS[query_kind_key_idx + 2]

	-- Check lobby exists
	if redis.call('EXISTS', key_direct_lobby_config) == 0 then
		return {'err', 'LOBBY_NOT_FOUND'}
	end

	-- Check lobby is not closed
	local is_closed = redis.call('HGET', key_direct_lobby_config, 'c') == '1'
	if is_closed then
		return {'err', 'LOBBY_CLOSED'}
	end

	-- Get max player count
	local max_player_count = nil
	if query.join_kind == 'direct' then
		max_player_count = tonumber(redis.call('HGET', key_direct_lobby_config, 'mpd'))
	elseif query.join_kind == 'party' then
		max_player_count = tonumber(redis.call('HGET', key_direct_lobby_config, 'mpp'))
	else
		return redis.error_reply('Invalid join kind')
	end

	-- Validate player count
	local player_count = redis.call('ZCARD', key_direct_lobby_player_ids)
	if player_count + table.getn(query.players) > max_player_count then
		return {'err', 'LOBBY_FULL'}
	end

	lobby_id = query.kind.direct.lobby_id
elseif query.kind.lobby_group ~= nil then
	-- MARK: Lobby group

	-- Iterate over all lobby ranked keys to find the most optimal lobby
	local best_lobby_id = nil
	local best_available_spots = 0
	for i = 1, available_spots_key_count do
		local key_available_spots = KEYS[available_spots_key_idx + i]

		-- Find the lobby with the least available spots (i.e. the most full lobby)
		--
		-- We always use `max_players_normal` when finding lobby groups
		-- instead of `max_players_party` because `max_players_party` is
		-- only relevant when joining a lobby with the exact lobby ID (i.e.
		-- where `max_players_direct` would normally be used).
		local lobby = redis.call('ZRANGEBYSCORE', key_available_spots, table.getn(query.players), '+inf', 'WITHSCORES', 'LIMIT', '0', '1')
		if table.getn(lobby) > 0 then
			local lobby_id = lobby[1]
			local available_spots = tonumber(lobby[2])

			-- Set as best lobby if no lobby selected or lobby is more full
			if best_lobby_id == nil or available_spots < best_available_spots then
				best_lobby_id = lobby_id
				best_available_spots = available_spots
			end
		end
	end

	-- Preemptively create a new lobby. The available spots will be updated
	-- with the preemptive players later in this script, but we call this just
	-- for consistency.
	local auto_create = query.kind.lobby_group.auto_create
	if best_lobby_id == nil and auto_create ~= nil then
		lobby_auto_created = true

		local key_lobby_unready = KEYS[query_kind_key_idx + 1]
		local key_auto_create_lobby_config = KEYS[query_kind_key_idx + 2]
		local key_auto_create_ns_lobby_ids = KEYS[query_kind_key_idx + 3]
		local key_auto_create_lobby_available_spots_normal = KEYS[query_kind_key_idx + 4]
		local key_auto_create_lobby_available_spots_party = KEYS[query_kind_key_idx + 5]

		for k, v in pairs(auto_create.lobby_config) do
			redis.call('HSET', key_auto_create_lobby_config, k, tostring(v))
		end
		redis.call('ZADD', key_auto_create_ns_lobby_ids, ts, auto_create.lobby_id)
		redis.call('ZADD', key_auto_create_lobby_available_spots_normal, auto_create.lobby_config['mpn'], auto_create.lobby_id)
		redis.call('ZADD', key_auto_create_lobby_available_spots_party, auto_create.lobby_config['mpp'], auto_create.lobby_id)
		redis.call('ZADD', key_lobby_unready, tonumber(auto_create.ready_expire_ts), auto_create.lobby_id)
	end

	-- Determine lobby ID to use
	if best_lobby_id ~= nil then
		lobby_id = best_lobby_id
	elseif auto_create ~= nil then
		lobby_id = auto_create.lobby_id
	else
		return {'err', 'NO_AVAILABLE_LOBBIES'}
	end
else
	return redis.error_reply('Invalid query kind')
end

-- MARK: Lobby validation
-- Fetching `lobby:{}:config` and updating the player counts might break the
-- consistency guarantees since the key is not guaranteed to be locked.
--
-- Auto created and direct lobbies will be locked correctly since we pass the
-- key to KEYS. Only lobbies finding through the lobby group will not be locked
-- correctly.
--
-- This is not a concern for now since we're not sharding the Redis servers.

-- Read lobby config
local key_lobby_config = 'mm:lobby:' .. lobby_id .. ':config'
local namespace_id = redis.call('HGET', key_lobby_config, 'ns')
local region_id = redis.call('HGET', key_lobby_config, 'r')
local lobby_group_id = redis.call('HGET', key_lobby_config, 'lg')
local max_players_normal = tonumber(redis.call('HGET', key_lobby_config, 'mpn'))
local max_players_party = tonumber(redis.call('HGET', key_lobby_config, 'mpp'))

-- Build keys for the given lobby ID
local key_lobby_find_queries = 'mm:lobby:' .. lobby_id .. ':find_queries'
local key_lobby_player_ids = 'mm:lobby:' .. lobby_id .. ':player_ids'
local key_lobby_available_spots_normal = 'mm:ns:' .. namespace_id .. ':region:' .. region_id .. ':lg:' .. lobby_group_id .. ':lobby:available_spots:normal'
local key_lobby_available_spots_party = 'mm:ns:' .. namespace_id .. ':region:' .. region_id .. ':lg:' .. lobby_group_id .. ':lobby:available_spots:party'
local key_idle_lobby_ids = 'mm:ns:' .. namespace_id .. ':region:' .. region_id .. ':lg:' .. lobby_group_id .. ':idle_lobby_ids'
local key_idle_lobby_lobby_group_ids = 'mm:ns:' .. namespace_id .. ':region:' .. region_id .. ':lobby:idle:lobby_group_ids'

-- Assert lobby state
if #lobby_id ~= 36 then
	return redis.error_reply('lobby_id is not valid UUID')
end
if #namespace_id ~= 36 then
	return redis.error_reply('namespace_id is not valid UUID')
end
if #region_id ~= 36 then
	return redis.error_reply('region_id is not valid UUID')
end
if #lobby_group_id ~= 36 then
	return redis.error_reply('lobby_group_id is not valid UUID')
end
if max_players_normal == nil then
	return redis.error_reply('max_players_normal is not a valid number')
end
if max_players_party == nil then
	return redis.error_reply('max_players_party is not a valid number')
end
if redis.call('EXISTS', key_lobby_config) == 0 then
	return redis.error_reply('Chosen lobby does not exist')
end

-- MARK: Create players
-- Register the players
for i, player in ipairs(query.players) do
	local player_id = player.player_id
	local key_player_config = KEYS[player_config_key_idx + i]
	local key_ns_remote_address_player_ids = KEYS[ns_remote_address_player_ids_key_idx + i]

	redis.call('HSET', key_player_config, 'l', lobby_id, 'qi', query.query_id)
	if player.remote_address ~= nil then
		redis.call('HSET', key_player_config, 'ra', player.remote_address)
	end
	redis.call('ZADD', key_ns_player_ids, ts, player_id)
	redis.call('ZADD', key_lobby_player_ids, ts, player_id)
	redis.call('ZADD', key_player_unregistered, query.player_register_expire_ts, player_id)
	if player.remote_address ~= nil then
		redis.call('SADD', key_ns_remote_address_player_ids, player_id)
	end
end

-- Update the available spots
local lobby_player_count = redis.call('ZCARD', key_lobby_player_ids)
redis.call('ZADD', key_lobby_available_spots_normal, max_players_normal - lobby_player_count, lobby_id)
redis.call('ZADD', key_lobby_available_spots_party, max_players_party - lobby_player_count, lobby_id)

-- Remove idle lobby if needed
redis.call('ZREM', key_idle_lobby_ids, lobby_id)
redis.call('HDEL', key_idle_lobby_lobby_group_ids, lobby_id)

-- MARK: Create query
for k, v in pairs(query.find_query_state) do
	redis.call('HSET', key_find_query_state, k, tostring(v))
end
redis.call('HSET', key_find_query_state, 'l', lobby_id)
redis.call('HSET', key_find_query_state, 'lac', lobby_auto_created and '1' or '0')

redis.call('ZADD', key_lobby_find_queries, ts, query.query_id)

for _, player in ipairs(query.players) do
	redis.call('SADD', key_find_query_player_ids, player.player_id)
end


return {'ok', {lobby_id, region_id, lobby_group_id}}

