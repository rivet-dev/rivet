CREATE TABLE routes (
    route_id UUID PRIMARY KEY,
    namespace_id UUID NOT NULL,
    name_id TEXT NOT NULL, -- Consistent identifier for the route
    hostname TEXT NOT NULL,
    path TEXT NOT NULL,
    route_subpaths BOOLEAN NOT NULL DEFAULT false,
    strip_prefix BOOLEAN NOT NULL DEFAULT true,
    route_type INT NOT NULL DEFAULT 0,
    actors_selector_tags JSONB,
    create_ts INT NOT NULL,
    update_ts INT NOT NULL,
    delete_ts INT NULL
);

CREATE INDEX routes_namespace_id_idx ON routes (namespace_id);

CREATE UNIQUE INDEX routes_namespace_id_name_id_idx ON routes (namespace_id, name_id) WHERE delete_ts IS NULL;

CREATE INDEX routes_hostname_path_exact_idx ON routes (hostname, path) 
WHERE route_subpaths = false AND delete_ts IS NULL;

CREATE INDEX routes_hostname_subpaths_idx ON routes (hostname, path) 
WHERE route_subpaths = true AND delete_ts IS NULL;

CREATE INDEX routes_hostname_idx ON routes (hostname) 
WHERE delete_ts IS NULL;

