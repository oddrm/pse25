-- Drop triggers
DROP TRIGGER IF EXISTS update_metadata_updated_at ON metadata;
DROP TRIGGER IF EXISTS update_sequences_updated_at ON sequences;
DROP TRIGGER IF EXISTS update_entries_updated_at ON entries;

-- Drop function
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop indexes
DROP INDEX IF EXISTS idx_metadata_entry_id;
DROP INDEX IF EXISTS idx_sequences_entry_id;
DROP INDEX IF EXISTS idx_topics_entry_id;
DROP INDEX IF EXISTS idx_topics_topic_name;
DROP INDEX IF EXISTS idx_entry_tags_tag_id;
DROP INDEX IF EXISTS idx_entry_tags_entry_id;
DROP INDEX IF EXISTS idx_entries_name;
DROP INDEX IF EXISTS idx_entries_path;

-- Drop tables in reverse order of dependencies
DROP TABLE IF EXISTS metadata;
DROP TABLE IF EXISTS sequences;
DROP TABLE IF EXISTS topics;
DROP TABLE IF EXISTS entry_tags;
DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS entries;
