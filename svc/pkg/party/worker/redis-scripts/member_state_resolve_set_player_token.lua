local query_id = ARGV[1]
local player_token = ARGV[2]

local key_party_member_config = KEYS[1]

if redis.call('EXISTS', key_party_member_config) == 0 then return nil end
if redis.call('JSON.TYPE', key_party_member_config, 'state.matchmaker_finding_lobby_direct') ~= 'object' then return nil end
if redis.call('JSON.RESP', key_party_member_config, 'state.matchmaker_finding_lobby_direct.direct_query_id') != query_id then return nil end

redis.call('JSON.SET', key_party_member_config, 'state.matchmaker_finding_lobby_direct.player_token', cjson.encode(player_token))

