-- Update vector dimensions from 1536 to 768 for Ollama nomic-embed-text
-- Run with: PGPASSWORD=your_password psql -U snaprag -d snaprag -h 192.168.1.160 -f scripts/update_to_768dim.sql

BEGIN;

-- Clear existing embeddings (they're NULL anyway)
UPDATE user_profiles SET 
  profile_embedding = NULL,
  bio_embedding = NULL,
  interests_embedding = NULL;

UPDATE user_profile_snapshots SET
  profile_embedding = NULL,
  bio_embedding = NULL,
  interests_embedding = NULL;

UPDATE user_profile_trends SET
  profile_embedding = NULL,
  bio_embedding = NULL;

-- Drop existing indexes
DROP INDEX IF EXISTS idx_user_profiles_profile_embedding;
DROP INDEX IF EXISTS idx_user_profiles_bio_embedding;
DROP INDEX IF EXISTS idx_profile_snapshots_profile_embedding;
DROP INDEX IF EXISTS idx_profile_snapshots_bio_embedding;

-- Update column types to 768 dimensions
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

-- Recreate indexes with 768 dimensions
CREATE INDEX idx_user_profiles_profile_embedding ON user_profiles 
  USING ivfflat (profile_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX idx_user_profiles_bio_embedding ON user_profiles 
  USING ivfflat (bio_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX idx_profile_snapshots_profile_embedding ON user_profile_snapshots 
  USING ivfflat (profile_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX idx_profile_snapshots_bio_embedding ON user_profile_snapshots 
  USING ivfflat (bio_embedding vector_cosine_ops) WITH (lists = 100);

COMMIT;

SELECT 'Database updated to 768-dim vectors for Ollama!' as status;

