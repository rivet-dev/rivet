CREATE TABLE ips (
    ip STRING PRIMARY KEY,
	
	-- ipinfo.io
	ip_info_io_data JSONB,
	ip_info_io_fetch_ts INT
);

