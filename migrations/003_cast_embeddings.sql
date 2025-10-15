-- Migration: Add cast embeddings support
-- Description: Enable semantic search for cast content

-- Create cast_embeddings table
CREATE TABLE IF NOT EXISTS cast_embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_hash BYTEA NOT NULL UNIQUE,
    fid BIGINT NOT NULL,
    text TEXT NOT NULL,
    embedding vector(768),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Foreign key to casts table
    CONSTRAINT fk_cast_embeddings_message_hash 
        FOREIGN KEY (message_hash) 
        REFERENCES casts(message_hash) 
        ON DELETE CASCADE
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_cast_embeddings_message_hash ON cast_embeddings(message_hash);
CREATE INDEX IF NOT EXISTS idx_cast_embeddings_fid ON cast_embeddings(fid);
CREATE INDEX IF NOT EXISTS idx_cast_embeddings_created_at ON cast_embeddings(created_at DESC);

-- Create vector similarity index (IVFFlat)
-- Will be more efficient once we have substantial data
CREATE INDEX IF NOT EXISTS idx_cast_embeddings_vector 
    ON cast_embeddings 
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);

-- Add indexes to casts table for better join performance
CREATE INDEX IF NOT EXISTS idx_casts_fid ON casts(fid);
CREATE INDEX IF NOT EXISTS idx_casts_timestamp ON casts(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_casts_text_not_null ON casts(fid, timestamp DESC) WHERE text IS NOT NULL;

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_cast_embeddings_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger for updated_at
CREATE TRIGGER trigger_cast_embeddings_updated_at
    BEFORE UPDATE ON cast_embeddings
    FOR EACH ROW
    EXECUTE FUNCTION update_cast_embeddings_updated_at();

-- Add comment
COMMENT ON TABLE cast_embeddings IS 'Vector embeddings for cast content to enable semantic search';
COMMENT ON COLUMN cast_embeddings.embedding IS 'Dense vector representation (768-dim) of cast text content';

