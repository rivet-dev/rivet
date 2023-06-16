local key_thread_tail_message = KEYS[1]

local thread_tail_message_expire_duration = ARGV[1]
local user_thread_history_expire_duration = ARGV[2]
local thread_id = ARGV[3]
local message_id = ARGV[4]
local send_ts = tonumber(ARGV[5])
local message_buf = ARGV[6]

-- Save the message tail if:
-- * tail message does not exist or
-- * the send ts is newer
local old_tail_send_ts = redis.call('HGET', key_thread_tail_message, 'st')
if not old_tail_send_ts or tonumber(old_tail_send_ts) < send_ts then
	redis.call('HSET', key_thread_tail_message, 'm', message_id, 'st', send_ts, 'mb', message_buf)
	redis.call('EXPIRE', key_thread_tail_message, thread_tail_message_expire_duration)
end

-- Update user threads
for i=2,#KEYS,2 do
	local key_user_thread_history_loaded = KEYS[i]
	local key_user_thread_history = KEYS[i+1]

	-- Check if user thread history exists
	if redis.call('GET', key_user_thread_history_loaded) == '1' then
		-- TODO: Trim length of threads if needed
		-- Add thread id to thread history
		redis.call('ZADD', key_user_thread_history, send_ts, thread_id)

		-- Expire cache
		redis.call('EXPIRE', key_user_thread_history_loaded, user_thread_history_expire_duration)
		redis.call('EXPIRE', key_user_thread_history, user_thread_history_expire_duration)
	end
end

