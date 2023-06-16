local key_thread_typing_statuses = KEYS[1]
local key_thread_typing_statuses_update_ts = KEYS[2]

local expire_duration = ARGV[1]
local now_ts = ARGV[2]

local expired_list = redis.call('ZRANGE', key_thread_typing_statuses_update_ts, 0, now_ts - expire_duration, 'BYSCORE', 'WITHSCORES')

-- Prune all expired statuses in hashmap
for i=1,#expired_list,2 do
	local user_id = expired_list[i]
	local update_ts = tonumber(expired_list[i + 1])

	redis.call('HDEL', key_thread_typing_statuses, user_id)
end

redis.call('ZREMRANGEBYSCORE', key_thread_typing_statuses_update_ts, '-inf', now_ts - expire_duration)