-- Create topics table for per-entry topic metadata.
CREATE TABLE IF NOT EXISTS topics (
  id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
  entry_id BIGINT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
  topic_name TEXT NOT NULL,
  topic_type TEXT,
  message_count BIGINT DEFAULT 0 NOT NULL,
  frequency DOUBLE PRECISION,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT now() NOT NULL,
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT now() NOT NULL
);
ALTER TABLE "entries" ADD COLUMN "status" VARCHAR NOT NULL;