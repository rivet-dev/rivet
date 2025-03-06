ALTER TABLE teams ADD COLUMN display_name_len INT AS (char_length(display_name)) STORED;
CREATE INDEX display_name_index ON teams (display_name, display_name_len ASC);