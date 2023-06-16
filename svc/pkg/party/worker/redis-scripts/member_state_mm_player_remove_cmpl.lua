local ts = tonumber(ARGV[1])
local lobby_id = ARGV[2]
local player_id = ARGV[3]
local party_member_state_json = ARGV[4]

local key_party_member_config = KEYS[1]

-- Update party member that this player belonged to. We do this again to make
-- sure there's not a race condition.
local update_party_id = nil
if
	redis.call('EXISTS', key_party_member_config) == 1 and
	(
		redis.call('JSON.TYPE', key_party_member_config, 'state.*.player_id') == 'string' and
		redis.call('JSON.RESP', key_party_member_config, 'state.*.player_id') == player_id
	)
then
	update_party_id = redis.call('JSON.RESP', key_party_member_config, 'party_id')
	redis.call('JSON.SET', key_party_member_config, 'state', party_member_state_json)
	redis.call('JSON.SET', key_party_member_config, 'state_change_ts', ts)
end

-- Find all parties in the lobby
local party_search = redis.call(
	'FT.SEARCH', 'party-idx',
	'@mm_lobby_id:{' .. lobby_id:gsub('-', '\\-') .. '}',
	'RETURN', 1, '$.party_id'
)
local lobby_party_ids = {}
for i=1,tonumber(party_search[1]) do
	local entry = party_member_search[2 + ((i - 1) * 2)]
	local party_id = entry[2]
	table.insert(lobby_party_ids, party_id:gsub('-', '\\-'))
end

-- Find the party member that has been pending the longest
local party_member_search = redis.call(
	'FT.SEARCH', 'party-member-idx',
	'@party_id:{' .. table.concat(lobby_party_ids, '|') .. '}',
	'RETURN', 3, '$.user_id', '$.party_id', '$.state',
	'SORTBY', 'state_change_ts', 'ASC'
)

-- Find the pending member that has been pending the longest.
--
-- These are sorted by state change ts already, so we stop once we've find the
-- first user ID.
local pending_user_id = nil
local pending_user_party_id = nil
local oldest_state_change_ts = nil
local i = 0
while pending_user_id == nil do
	local entry = party_member_search[2 + (i * 2)]
	
	if entry ~= nil then
		local user_id = entry[2]
		local party_id = entry[4]
		local state = entry[6]

		if state['matchmaker_ready'] ~= nil then
			pending_user_id = user_id
			pending_user_party_id = party_id
			oldest_state_change_ts = state_change_ts
		else
			i = i + 1
		end
	else
		return {'err', 'party_member_search exhausted'}
	end
end

return {'ok', {update_party_id, pending_user_id, pending_user_party_id}}

