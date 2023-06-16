local ts = ARGV[1]
local party_id = ARGV[2]
local user_id = ARGV[3]
local party_member_config = ARGV[4]

local key_party_config = KEYS[1]
local key_party_member_config = KEYS[2]

-- Check if party is full
if redis.call('EXISTS', key_party_config) == 0 then
	return {'err', 'PARTY_DOES_NOT_EXIST'}
end
local party_size = tonumber(redis.call('JSON.RESP', key_party_config, 'party_size'))
local member_count = redis.call(
	'FT.AGGREGATE', 'party-member-idx',
	'@party_id:{' .. party_id:gsub('-', '\\-') .. '}',
	'GROUPBY', 0,
		'REDUCE', 'COUNT', 0
)
if tonumber(member_count[1]) > 0 and tonumber(member_count[2][2]) >= party_size then
	return {'err', 'PARTY_FULL'}
end

-- Remove member from another party if needed
local old_party_id = ''
if redis.call('EXISTS', key_party_member_config) == 1 then
	old_party_id = redis.call('JSON.RESP', key_party_member_config, 'party_id')
	if old_party_id == party_id then
		return {'err', 'ALREADY_IN_PARTY'}
	end
end

-- Create party member
redis.call('UNLINK', key_party_member_config)
redis.call('JSON.SET', key_party_member_config, '$', party_member_config)

return {'ok', {old_party_id}}

