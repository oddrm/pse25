-- Make topics nullable and convert from text[] to jsonb
ALTER TABLE entries ALTER COLUMN topics DROP NOT NULL;
ALTER TABLE entries ALTER COLUMN topics TYPE jsonb USING to_jsonb(topics);
