local ts = ARGV[1]

if redis.call('SETNX', 'party:migrations:party_idx', ts) == 1 then
	redis.call(
		'FT.CREATE', 'party-idx', 'ON', 'JSON',
		'PREFIX', 1, 'party:party:config:',
		'SCHEMA',
			'$.state.matchmaker_finding_lobby.query_id', 'AS', 'mm_query_id', 'TAG', 'CASESENSITIVE',
			'$.state.matchmaker_lobby.lobby_id', 'AS', 'mm_lobby_id', 'TAG', 'CASESENSITIVE'
	)
end

if redis.call('SETNX', 'party:migrations:party_member_idx', ts) == 1 then
	redis.call(
		'FT.CREATE', 'party-member-idx', 'ON', 'JSON',
		'PREFIX', 1, 'party:member:config:',
		'SCHEMA',
			'$.party_id', 'AS', 'party_id', 'TAG', 'CASESENSITIVE',
			'$.create_ts', 'AS', 'create_ts', 'NUMERIC', 'SORTABLE',
			'$.state_change_ts', 'AS', 'state_change_ts', 'NUMERIC', 'SORTABLE',
			'$.state.matchmaker_finding_lobby_direct.direct_query_id', 'AS', 'mm_direct_query_id', 'TAG', 'CASESENSITIVE',
			'$.state.*.player_id', 'AS', 'mm_player_id', 'TAG', 'CASESENSITIVE'
	)
end

local idx = redis.call('FT.INFO', 'party-member-idx')

-- Check if `client_info` already exists
local exists = false
for _, field in ipairs(idx[8]) do
	if field[2] == 'client_info' then
		exists = true
		break
	end
end
if exists then
	redis.call(
		'FT.ALTER', 'party-member-idx',
		'SCHEMA', 'ADD',
			'$.state.*.player_id', 'AS', 'mm_player_id', 'TAG', 'CASESENSITIVE'
	)
end

if redis.call('SETNX', 'party:migrations:party_invite_idx', ts) == 1 then
	redis.call(
		'FT.CREATE', 'party-invite-idx', 'ON', 'JSON',
		'PREFIX', 1, 'party:invite:config:',
		'SCHEMA',
			'$.party_id', 'AS', 'party_id', 'TAG', 'CASESENSITIVE',
			'$.alias.namespace_id', 'AS', 'alias_namespace_id', 'TAG', 'CASESENSITIVE',
			'$.alias.alias', 'AS', 'alias', 'TAG', 'CASESENSITIVE'
	)
end

