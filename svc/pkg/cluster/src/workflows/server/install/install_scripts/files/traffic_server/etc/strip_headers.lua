-- Strip all request headers to ensure that:
-- (a) the cache response is consistent
-- (b) prevent causing any errors from S3 from headers that are not supported
function do_remap()
	-- Delete all other headers
	local req_headers = ts.client_request.get_headers()
	for k, _ in pairs(req_headers) do
		if k ~= 'Host' then
			ts.client_request.header[k] = nil
		end
	end

	-- Write custom headers
	ts.client_request.header['User-Agent'] = 'Rivet-ATS/' .. ts.get_traffic_server_version()
	ts.client_request.header['Accept'] = '*/*'

	return 0
end

