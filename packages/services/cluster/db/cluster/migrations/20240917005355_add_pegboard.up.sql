ALTER TABLE servers
	-- Null until pegboard client successfully registers
	ADD COLUMN pegboard_client_id UUID;
