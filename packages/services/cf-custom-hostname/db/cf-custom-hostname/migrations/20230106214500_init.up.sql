CREATE TABLE custom_hostnames (
    identifier UUID PRIMARY KEY,
    namespace_id UUID NOT NULL,
	hostname TEXT NOT NULL,
	challenge UUID NOT NULL,
    status INT NOT NULL,
    create_ts INT NOT NULL,
    UNIQUE (hostname)
);
CREATE INDEX ON custom_hostnames (namespace_id);
CREATE INDEX ON custom_hostnames (hostname);
CREATE INDEX ON custom_hostnames (status);
