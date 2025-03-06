CREATE TABLE config (
	id INT PRIMARY KEY DEFAULT 1,
	cluster_id UUID NOT NULL,
	CONSTRAINT single_row CHECK (id = 1)
);

