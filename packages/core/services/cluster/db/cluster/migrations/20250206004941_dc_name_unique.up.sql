ALTER TABLE datacenters
ADD CONSTRAINT name_id_unique UNIQUE (name_id);
