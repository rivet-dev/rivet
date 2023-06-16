local party_id = ARGV[1]
local publicity_public = ARGV[2]
local publicity_friends = ARGV[3]
local publicity_teams = ARGV[4]

local key_party_config = KEYS[1]

-- Check the party exists
if redis.call('EXISTS', key_party_config) == 0 then
	return {'err', 'PARTY_DOES_NOT_EXIST'}
end

-- Set publicity values
if publicity_public ~= '' then
	redis.call('JSON.SET', key_party_config, 'publicity.public', publicity_public)
end
if publicity_friends ~= '' then
	redis.call('JSON.SET', key_party_config, 'publicity.friends', publicity_friends)
end
if publicity_teams ~= '' then
	redis.call('JSON.SET', key_party_config, 'publicity.teams', publicity_teams)
end

return {'ok'}
