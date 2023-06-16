ALTER TABLE teams ADD COLUMN owner_user_id UUID NOT NULL DEFAULT gen_random_uuid();
