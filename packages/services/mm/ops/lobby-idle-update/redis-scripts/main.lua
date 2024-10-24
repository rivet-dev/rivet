local namespace_id = ARGV[1]
local lobby_group_count = tonumber(ARGV[2])

local key_idle_lobby_lobby_group_ids = KEYS[1]

-- Parse lobby groups
local all_lobby_group_ids = {}
local create_lobby_group_ids = {}  -- Lobby group IDs to create idle lobbies in, there may be repeats
local overflow_lobby_ids = {}  -- Idle lobbies to be removed
for i = 1, lobby_group_count do
	local base_idx = 3 + (i - 1) * 3
	local lobby_group_id = ARGV[base_idx]
	local min_idle_lobbies = tonumber(ARGV[base_idx + 1])
	local max_idle_lobbies = tonumber(ARGV[base_idx + 2])
	local key_idle_lobby_ids = KEYS[1 + i]

	table.insert(all_lobby_group_ids, lobby_group_id)

	-- Find extra lobbies to remove if needed
	local current_idle_lobbies = tonumber(redis.call('ZCARD', key_idle_lobby_ids))
	if current_idle_lobbies > max_idle_lobbies then
		-- Remove idle lobbies
		local lobbies_to_remove = current_idle_lobbies - max_idle_lobbies
		for _, lobby_id in ipairs(redis.call('ZRANGE', key_idle_lobby_ids, 0, lobbies_to_remove - 1)) do
			table.insert(overflow_lobby_ids, lobby_id)
		end
	elseif current_idle_lobbies < min_idle_lobbies then
		-- Register idle lobbies
		for _ = 1, (min_idle_lobbies - current_idle_lobbies) do
			table.insert(create_lobby_group_ids, lobby_group_id)
		end
	end
end

-- Find outdated idle lobbies
local scan_cursor = 0
local outdated_lobby_ids = {}
repeat
	-- Scan lobby groups
	local res = redis.call('HSCAN', key_idle_lobby_lobby_group_ids, scan_cursor)
	scan_cursor = tonumber(res[1])
	local kv = res[2]
	for i = 1, table.getn(kv), 2 do
		local lobby_id = kv[i]
		local lobby_group_id = kv[i + 1]

		-- Check if the lobby group id exists in the most recent version
		local is_valid = false
		for _, valid_lobby_group_id in ipairs(all_lobby_group_ids) do
			if lobby_group_id == valid_lobby_group_id then
				is_valid = true
				break
			end
		end
		if not is_valid then
			table.insert(outdated_lobby_ids, lobby_id)
		end
	end
until scan_cursor == 0

return {create_lobby_group_ids, overflow_lobby_ids, outdated_lobby_ids}

