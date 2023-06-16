local ts = tonumber(ARGV[1])
local query_id = ARGV[2]
local party_member_state_json = ARGV[3]

local key_party_member_config = KEYS[1]

if redis.call('EXISTS', key_party_member_config) == 0 then return {'err', 'PARTY_MEMBER_DOES_NOT_EXIST'} end
if redis.call('JSON.TYPE', key_party_member_config, 'state.matchmaker_finding_lobby_direct') ~= 'object' then return {'err', 'PARTY_MEMBER_IN_DIFFERENT_STATE'} end
if redis.call('JSON.RESP', key_party_member_config, 'state.matchmaker_finding_lobby_direct.direct_query_id') ~= query_id then return {'err', 'PARTY_MEMBER_HAS_DIFFERENT_QUERY'} end

local party_id = redis.call('JSON.RESP', party_member_config, 'party_id')
local player_id = redis.call('JSON.RESP', key_party_member_config, 'state.matchmaker_finding_lobby_direct.player_id')

-- Reset state to pending
redis.call('JSON.SET', key_party_member_config, 'state', party_member_state_json)
redis.call('JSON.SET', key_party_member_config, 'state_change_ts', ts)

return {'ok', party_id}

