ALTER TABLE builds ADD COLUMN image_tag TEXT NOT NULL UNIQUE DEFAULT gen_random_uuid()::TEXT;
