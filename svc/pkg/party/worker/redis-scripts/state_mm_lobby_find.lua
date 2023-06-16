local req = cjson.decode(ARGV[1])

local key_party_config = KEYS[1]

-- Update party
if redis.call('EXISTS', key_party_config) == 0 then return {'err', 'PARTY_DOES_NOT_EXIST'} end

-- Update config
redis.call('JSON.SET', key_party_config, 'state', req.state_json)
redis.call('JSON.SET', key_party_config, 'state_change_ts', req.ts)

-- Update members if they exist
--
-- We always force the leader to join the lobby so there's at least one user joining
--
-- Track which members are actually going to join the lobby.
local leader_user_id = redis.call('JSON.RESP', key_party_config, 'leader_user_id')
local joining_user_ids = {}
for i, member in ipairs(req.members) do
	local key_party_member_config = KEYS[1 + i]

	if redis.call('EXISTS', key_party_member_config) == 1 then
		local old_state = redis.call('JSON.OBJKEYS', key_party_member_config, 'state')[1]
		if
			member.user_id == leader_user_id or
			old_state == 'matchmaker_ready' or
			old_state == 'matchmaker_finding_lobby' or
			old_state == 'matchmaker_finding_lobby_direct' or
			old_state == 'matchmaker_lobby'
		then
			table.insert(joining_user_ids, member.user_id)

			-- Update config
			redis.call('JSON.SET', key_party_member_config, 'state', member.state_json)
			redis.call('JSON.SET', key_party_member_config, 'state_change_ts', req.ts)
		end
	end
end

return {'ok', joining_user_ids}

