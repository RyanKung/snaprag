-- Update vector dimensions from 1536 to 768 for Ollama models
-- Run this with: psql -U snaprag -d snaprag -f update_vector_dim.sql

BEGIN;

-- Drop existing vector indexes
DROP INDEX IF EXISTS idx_user_profiles_profile_embedding;
DROP INDEX IF EXISTS idx_user_profiles_bio_embedding;
DROP INDEX IF EXISTS idx_profile_snapshots_profile_embedding;
DROP INDEX IF EXISTS idx_profile_snapshots_bio_embedding;

-- Alter column types to new dimension
ALTER TABLE user_profiles 
  ALTER COLUMN profile_embedding TYPE VECTOR(768),
  ALTER COLUMN bio_embedding TYPE VECTOR(768),
  ALTER COLUMN interests_embedding TYPE VECTOR(768);

ALTER TABLE user_profile_snapshots 
  ALTER COLUMN profile_embedding TYPE VECTOR(768),
  ALTER COLUMN bio_embedding TYPE VECTOR(768),
  ALTER COLUMN interests_embedding TYPE VECTOR(768);

ALTER TABLE user_profile_trends 
  ALTER COLUMN profile_embedding TYPE VECTOR(768),
  ALTER COLUMN bio_embedding TYPE VECTOR(768);

-- Recreate vector indexes
CREATE INDEX idx_user_profiles_profile_embedding ON user_profiles 
USING ivfflat (profile_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX idx_user_profiles_bio_embedding ON user_profiles 
USING ivfflat (bio_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX idx_profile_snapshots_profile_embedding ON user_profile_snapshots 
USING ivfflat (profile_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX idx_profile_snapshots_bio_embedding ON user_profile_snapshots 
USING ivfflat (bio_embedding vector_cosine_ops) WITH (lists = 100);

COMMIT;

