local ts = tonumber(ARGV[1])
local party_id = ARGV[2]
local query_id = ARGV[3]
local lobby_id = ARGV[4]
local party_member_state_json = ARGV[5]

local key_party_config = KEYS[1]
local key_party_member_config = KEYS[2]

if redis.call('EXISTS', key_party_member_config) == 0 then
	return {'err', 'PARTY_MEMBER_DOES_NOT_EXIST'}
end
if redis.call('JSON.RESP', key_party_member_config, 'party_id') ~= party_id then
	return {'err', 'PARTY_MEMBER_NOT_IN_PARTY'}
end
if redis.call('JSON.TYPE', key_party_member_config, 'state.matchmaker_finding_lobby_direct') ~= 'object' then
	return {'err', 'PARTY_MEMBER_IN_DIFFERENT_STATE'}
end
if redis.call('JSON.RESP', key_party_member_config, 'state.matchmaker_finding_lobby_direct.direct_query_id') ~= query_id then
	return {'err', 'PARTY_MEMBER_HAS_DIFFERENT_QUERY'}
end
if redis.call('JSON.TYPE', key_party_config, 'state.matchmaker_lobby') ~= 'object' then
	return {'err', 'PARTY_NOT_IN_LOBBY'}
end
if redis.call('JSON.RESP', key_party_config, 'state.matchmaker_lobby.lobby_id') ~= lobby_id then
	return {'err', 'PARTY_IN_DIFFERENT_LOBBY'}
end

local player_id = redis.call('JSON.RESP', key_party_member_config, 'state.matchmaker_finding_lobby_direct.player_id')
local player_id = redis.call('JSON.RESP', key_party_member_config, 'state.matchmaker_finding_lobby_direct.player_id')
local player_token = redis.call('JSON.RESP', key_party_member_config, 'state.matchmaker_finding_lobby_direct.player_token')

-- Update party member
redis.call('JSON.SET', key_party_member_config, 'state', party_member_state_json)
redis.call('JSON.SET', key_party_member_config, 'state_change_ts', ts)

return {'ok', {party_id, player_id, player_token}}

