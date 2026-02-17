-- Recreate entries.topics jsonb column from topics table and drop topics table

-- add topics column back (nullable jsonb)
ALTER TABLE entries ADD COLUMN topics jsonb;

-- populate entries.topics from topics table
WITH agg AS (
  SELECT entry_id, jsonb_agg(jsonb_build_object(
    'name', topic_name,
    'type', COALESCE(topic_type, ''),
    'message_count', message_count,
    'frequency', frequency
  )) AS topics
  FROM topics
  GROUP BY entry_id
)
UPDATE entries
SET topics = agg.topics
FROM agg
WHERE entries.id = agg.entry_id;

DROP TABLE IF EXISTS topics;
