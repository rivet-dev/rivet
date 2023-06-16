local ts = tonumber(ARGV[1])
local party_id = ARGV[2]
local user_id = ARGV[3]
local party_member_state_finding_lobby_direct_json = ARGV[4]
local direct_player_id = ARGV[5]

local key_party_config = KEYS[1]
local key_party_member_config = KEYS[2]

-- Check party exists, party member exists, and party member is in party
if redis.call('EXISTS', key_party_config) == 0 then return {'err', 'PARTY_DOES_NOT_EXIST'} end
if redis.call('EXISTS', key_party_member_config) == 0 then return {'err', 'PARTY_MEMBER_DOES_NOT_EXIST'} end
if redis.call('JSON.RESP', key_party_member_config, 'party_id') ~= party_id then return {'err', 'PARTY_MEMBER_NOT_IN_PARTY'} end

local party_state = redis.call('JSON.OBJKEYS', key_party_config, 'state')[0]
local member_state = redis.call('JSON.OBJKEYS', key_party_member_config, 'state')[0]

if party_state == 'matchmaker_lobby' and member_state == 'matchmaker_ready' then
	-- Member needs to find lobby

	-- Read lobby config
	local namespace_id = redis.call('JSON.RESP', key_party_config, 'state.matchmaker_lobby.namespace_id')
	local lobby_id = redis.call('JSON.RESP', key_party_config, 'state.matchmaker_lobby.lobby_id')

	-- Update member
	--
	-- Player token will be set once token-create is called after this.
	redis.call('JSON.SET', key_party_member_config, 'state', party_member_state_finding_lobby_direct_json)
	redis.call('JSON.SET', key_party_member_config, 'state_change_ts', ts)
	local client_info = redis.call('JSON.GET', key_party_member_config, 'client_info')

	return {'ok', cjson.encode({
		mm_finding_lobby = {
			namespace_id = namespace_id,
			lobby_id = lobby_id,
			client_info = client_info,
		}
	})}
else
	return {'ok', nil}
end

