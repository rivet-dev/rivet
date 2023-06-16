local party_id = ARGV[1]
local user_id = ARGV[2]
local skip_delete = ARGV[3] == '1'
local skip_party_cleanup = ARGV[4] == '1'

local key_party_config = KEYS[1]
local key_party_member_config = KEYS[2]

if redis.call('EXISTS', key_party_member_config) == 0 then return {'err', 'PARTY_MEMBER_DOES_NOT_EXIST'} end

-- Remove party member
local member_updated = false
if not skip_delete then
	-- Check the player is still in the given party
	if redis.call('JSON.RESP', key_party_member_config, 'party_id') == party_id then
		member_updated = true

		-- Remove the member
		redis.call('UNLINK', key_party_member_config)
	end
end

-- Cleanup party
local remove_party = false
local set_party_leader = false
if not skip_party_cleanup then
	-- Check if the party is empty
	local member_count = redis.call(
		'FT.AGGREGATE', 'party-member-idx',
		'@party_id:{' .. party_id:gsub('-', '\\-') .. '}',
		'GROUPBY', 0,
			'REDUCE', 'COUNT', 0
	)
	if tonumber(member_count[1]) == 0 or tonumber(member_count[2][2]) == 0 then
		remove_party = true
	else
		-- Check if we need a new leader
		if redis.call('JSON.RESP', key_party_config, 'leader_user_id') == user_id then
			set_party_leader = true
		end
	end
end

return {'ok', {member_updated, remove_party, set_party_leader}}

