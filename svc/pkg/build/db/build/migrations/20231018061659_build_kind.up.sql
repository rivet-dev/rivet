ALTER TABLE builds
	ADD COLUMN kind INT NOT NULL DEFAULT 0,  -- rivet.backend.build.BuildKind
	ADD COLUMN compression INT NOT NULL DEFAULT 0;  -- rivet.backend.build.Compression

