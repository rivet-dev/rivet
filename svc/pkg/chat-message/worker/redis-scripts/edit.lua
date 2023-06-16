local key_thread_tail_message = KEYS[1]

local message_id = ARGV[1]
local message_buf = ARGV[2]

local tail_message_id = redis.call('HGET', key_thread_tail_message, 'm')
if tail_message_id == message_id then
	redis.call('HSET', key_thread_tail_message, 'mb', message_buf)
end

