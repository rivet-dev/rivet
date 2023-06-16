ALTER TABLE users RENAME COLUMN profile_id TO profile_id_old;
ALTER TABLE users ADD COLUMN profile_id UUID;