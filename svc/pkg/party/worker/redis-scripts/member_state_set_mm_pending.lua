local ts = tonumber(ARGV[1])
local party_id = ARGV[2]
local party_member_state_json = ARGV[3]

local key_party_member_config = KEYS[1]

-- Validate in idle state
if redis.call('JSON.TYPE', key_party_member_config, 'state.inactive') ~= 'object' then
	return false
end
if redis.call('JSON.RESP', key_party_member_config, 'party_id') ~= party_id then
	return false
end

redis.call('JSON.SET', key_party_member_config, 'state', party_member_state_json)
redis.call('JSON.SET', key_party_member_config, 'state_change_ts', ts)

return true

