local ts = ARGV[1]
local party_id = ARGV[2]
local query_id = ARGV[3]
local party_state_json = ARGV[4]

local key_party_config = KEYS[1]

if redis.call('EXISTS', key_party_config) == 0 then return {'err', 'PARTY_DOES_NOT_EXIST'} end
if redis.call('JSON.TYPE', key_party_config, 'state.matchmaker_finding_lobby') ~= 'object' then return {'err', 'PARTY_IN_DIFFERENT_STATE'} end
if redis.call('JSON.RESP', key_party_config, 'state.matchmaker_finding_lobby.query_id') ~= query_id then return {'err', 'PARTY_HAS_DIFFERENT_QUERY'} end

-- Update party
redis.call('JSON.SET', key_party_config, 'state', party_state_json)
redis.call('JSON.SET', key_party_config, 'state_change_ts', ts)

-- Update members waiting for lobby
local party_member_search = redis.call('FT.SEARCH', 'party-member-idx', '@party_id:{' .. party_id:gsub('-', '\\-') .. '}', 'NOCONTENT')
local joining_party_members = {}
for i=1,tonumber(party_member_search[1]) do
	local key_party_member_config = party_member_search[i + 1]

	if redis.call('JSON.TYPE', key_party_member_config, 'state.matchmaker_finding_lobby') == 'object' then
		local user_id = redis.call('JSON.RESP', key_party_member_config, 'user_id')
		local player_id = redis.call('JSON.RESP', key_party_member_config, 'state.matchmaker_finding_lobby.player_id')
		local player_token = redis.call('JSON.RESP', key_party_member_config, 'state.matchmaker_finding_lobby.player_token')

		-- Set the member's state to `MatchmakerLobby` 
		redis.call('JSON.SET', key_party_member_config, 'state', cjson.encode({
			matchmaker_lobby = {
				player_id = player_id,
				player_token = player_token
			}
		}))
		redis.call('JSON.SET', key_party_member_config, 'state_change_ts', ts)

		table.insert(joining_party_members, {user_id, player_id, player_token})
	end
end

return {'ok', joining_party_members}

