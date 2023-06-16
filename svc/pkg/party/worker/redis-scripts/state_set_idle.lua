local ts = tonumber(ARGV[1])
local party_id = ARGV[2]
local party_state_json = ARGV[3]
local party_member_state_json_idle_json = ARGV[4]
local party_member_state_matchmaker_ready_json = ARGV[5]

local key_party_config = KEYS[1]

if redis.call('EXISTS', key_party_config) == 0 then return {'err', 'PARTY_DOES_NOT_EXIST'} end
if redis.call('JSON.TYPE', key_party_config, 'state.inactive') == 'object' then return {'ok', nil} end

-- MARK: Party
-- Update config
redis.call('JSON.SET', key_party_config, 'state', party_state_json)
redis.call('JSON.SET', key_party_config, 'state_change_ts', ts)

-- MARK: Party members
-- Update members if they exist
local party_member_search = redis.call('FT.SEARCH', 'party-member-idx', '@party_id:{' .. party_id:gsub('-', '\\-') .. '}', 'NOCONTENT')
for i=1,tonumber(party_member_search[1]) do
	local key_party_member_config = party_member_search[i + 1]

	local party_member_state_keys = redis.call('JSON.OBJKEYS', key_party_member_config, 'state')
	if party_member_state_keys ~= nil then
		local party_member_state = party_member_state_keys[1]
		if party_member_state == 'matchmaker_ready' or party_member_state == 'idle' then
			-- Do nothing
		elseif party_member_state == 'matchmaker_finding_lobby' or party_member_state == 'matchmaker_finding_lobby_direct' or party_member_state == 'matchmaker_lobby' then
			-- Set to pending player when lobby set to idle
			redis.call('JSON.SET', key_party_member_config, 'state', party_member_state_matchmaker_ready_json)
			redis.call('JSON.SET', key_party_member_config, 'state_change_ts', ts)
		else
			-- Set to idle state
			redis.call('JSON.SET', key_party_member_config, 'state', party_member_state_json_idle_json)
			redis.call('JSON.SET', key_party_member_config, 'state_change_ts', ts)
		end
	end
end

return {'ok', nil}

