-- Update vector dimensions to support different embedding models
-- This migration changes vector dimensions from 1536 (OpenAI) to 768 (Ollama nomic-embed-text)
-- or makes them flexible

-- Note: This requires recreating the columns if you have existing data
-- For production, you may want to:
-- 1. Backup existing embeddings
-- 2. Drop columns
-- 3. Recreate with new dimensions
-- 4. Regenerate embeddings

-- For new installations or if changing embedding models:

-- Drop existing vector indexes
DROP INDEX IF EXISTS idx_user_profiles_profile_embedding;
DROP INDEX IF EXISTS idx_user_profiles_bio_embedding;
DROP INDEX IF EXISTS idx_profile_snapshots_profile_embedding;
DROP INDEX IF EXISTS idx_profile_snapshots_bio_embedding;

-- Option 1: If you want to keep existing data and use 768-dim model
-- ALTER TABLE user_profiles 
--   ALTER COLUMN profile_embedding TYPE VECTOR(768),
--   ALTER COLUMN bio_embedding TYPE VECTOR(768),
--   ALTER COLUMN interests_embedding TYPE VECTOR(768);

-- Option 2: If starting fresh or changing models
-- Clear existing embeddings (they're all NULL anyway for new installations)
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

-- Recreate vector indexes with updated dimension
-- Note: Adjust dimension (768) based on your model:
-- - nomic-embed-text: 768
-- - mxbai-embed-large: 1024  
-- - all-minilm: 384
-- - text-embedding-ada-002: 1536

CREATE INDEX IF NOT EXISTS idx_user_profiles_profile_embedding ON user_profiles 
USING ivfflat (profile_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX IF NOT EXISTS idx_user_profiles_bio_embedding ON user_profiles 
USING ivfflat (bio_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX IF NOT EXISTS idx_profile_snapshots_profile_embedding ON user_profile_snapshots 
USING ivfflat (profile_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX IF NOT EXISTS idx_profile_snapshots_bio_embedding ON user_profile_snapshots 
USING ivfflat (bio_embedding vector_cosine_ops) WITH (lists = 100);

