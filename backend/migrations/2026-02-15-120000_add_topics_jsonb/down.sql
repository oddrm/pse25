-- Revert topics column back to text[] and set NOT NULL
ALTER TABLE entries ALTER COLUMN topics TYPE text[] USING (
  CASE WHEN topics IS NULL THEN ARRAY[]::text[] ELSE ARRAY(SELECT jsonb_array_elements_text(topics)) END
);
ALTER TABLE entries ALTER COLUMN topics SET NOT NULL;
