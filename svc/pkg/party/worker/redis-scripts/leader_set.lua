local party_id = ARGV[1]
local leader_user_id = ARGV[2]

local key_party_config = KEYS[1]

-- Check the party exists
if redis.call('EXISTS', key_party_config) == 0 then
	return {'err', 'PARTY_DOES_NOT_EXIST'}
end

if leader_user_id ~= '' then
	-- Check the new leader is a member
	if redis.call('JSON.RESP', 'party:member:config:' .. leader_user_id, 'party_id') ~= party_id then
		return {'err', 'USER_NOT_PARTY_MEMBER'}
	end
else
	-- Determine a new party leader
	local leader_user_entry = redis.call(
		'FT.SEARCH', 'party-member-idx', '@party_id:{' .. party_id:gsub('-', '\\-') .. '}',
		'SORTBY', 'create_ts', 'ASC',
		'LIMIT', 0, 1,
		'RETURN', 1, '$.user_id'
	)[3]
	if leader_user_entry ~= nil then
		leader_user_id = leader_user_entry[2]
	else
		return {'err', 'COULD_NOT_DECIDE_LEADER'}
	end
end

-- Set the new leader
redis.call('JSON.SET', key_party_config, 'leader_user_id', cjson.encode(leader_user_id))

return {'ok'}

