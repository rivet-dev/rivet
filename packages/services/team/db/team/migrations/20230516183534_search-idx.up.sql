CREATE INDEX search_index ON teams
	USING GIN(display_name gin_trgm_ops)
	WHERE is_searchable = TRUE;
