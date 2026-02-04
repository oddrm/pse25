CREATE TABLE entries (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    path VARCHAR NOT NULL UNIQUE,
    platform VARCHAR NOT NULL,
    size BIGINT NOT NULL,
    -- Rosbag/MCAP metadata fields
    start_time_ns BIGINT, -- Starting time in nanoseconds
    duration_ns BIGINT, -- Duration in nanoseconds
    total_message_count BIGINT, 
    storage_identifier VARCHAR, -- Storage format (e.g., 'mcap')
    compression_format VARCHAR,
    compression_mode VARCHAR, 
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (updated_at >= created_at)
);


CREATE TABLE topics (
    id BIGSERIAL PRIMARY KEY,
    entry_id BIGINT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    topic_name VARCHAR NOT NULL,
    message_count BIGINT NOT NULL DEFAULT 0,
    type VARCHAR,
    type_description_hash VARCHAR,
    serialization_format VARCHAR,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    UNIQUE (entry_id, topic_name)
);


CREATE TABLE tags (
    id BIGSERIAL PRIMARY KEY,
    entry_id BIGINT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    name VARCHAR NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (entry_id, name)
);


CREATE TABLE sequences (
    id BIGSERIAL PRIMARY KEY,
    entry_id BIGINT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    start_timestamp BIGINT NOT NULL,
    end_timestamp BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (end_timestamp >= start_timestamp)
);


CREATE TABLE metadata (
    id BIGSERIAL PRIMARY KEY,
    entry_id BIGINT NOT NULL UNIQUE REFERENCES entries(id) ON DELETE CASCADE,
    metadata_json JSONB,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (updated_at >= created_at)
);






-- Create indexes for better query performance
CREATE INDEX idx_entries_path ON entries(path);
CREATE INDEX idx_entries_name ON entries(name);
CREATE INDEX idx_tags_entry_id ON tags(entry_id);
-- Wichtig: Index auf topic_name für effiziente Suche nach Topics
CREATE INDEX idx_topics_topic_name ON topics(topic_name);
CREATE INDEX idx_topics_entry_id ON topics(entry_id);
CREATE INDEX idx_sequences_entry_id ON sequences(entry_id);
CREATE INDEX idx_metadata_entry_id ON metadata(entry_id);



--ChatGPT Vorschlag

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers to automatically update updated_at
CREATE TRIGGER update_entries_updated_at BEFORE UPDATE ON entries
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_sequences_updated_at BEFORE UPDATE ON sequences
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_metadata_updated_at BEFORE UPDATE ON metadata
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
