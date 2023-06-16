local THREAD_COUNT = 64

local key_user_thread_history_loaded = KEYS[1]
local key_user_thread_history = KEYS[2]

local after_ts = ARGV[1]

if redis.call('GET', key_user_thread_history_loaded) == '1' then
	local thread_ids = {}
	local threads = {}

	-- Get thread ids
	if after_ts ~= nil then
		thread_ids = redis.call('ZRANGE', key_user_thread_history, '(' .. after_ts, '+inf', 'BYSCORE')
	else
		thread_ids = redis.call('ZRANGE', key_user_thread_history, -THREAD_COUNT, '+inf', 'BYSCORE')
	end

	-- Get tail messages
	for _, thread_id in ipairs(thread_ids) do
		local key_thread_tail_message = string.format('chat:thread:%s:tail_message', thread_id)

		local tail_message = redis.call('HGET', key_thread_tail_message, 'mb')
		if tail_message then
			table.insert(threads, {thread_id, tail_message[1]})
		else
			table.insert(threads, {thread_id, nil})
		end
	end

	return threads
else
	-- Threads are not loaded in to cache
	return nil
end

