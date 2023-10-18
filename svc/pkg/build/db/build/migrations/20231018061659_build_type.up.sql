ALTER TABLE builds
	ADD COLUMN build_kind INT NOT NULL DEFAULT 0,  -- rivet.backend.build.BuildKind
	ADD COLUMN build_compression INT NOT NULL DEFAULT 0;  -- rivet.backend.build.Compression

