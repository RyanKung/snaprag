use super::Database;
use crate::Result;
use crate::SnapRagError;

impl Database {
    /// Check if database schema is initialized
    /// Returns true if all required tables exist
    pub async fn is_schema_initialized(&self) -> Result<bool> {
        // Check for essential tables
        let required_tables = vec![
            "user_profile_changes", // Event-sourcing table
            "casts",
            "links",
            "processed_messages",
        ];

        for table_name in required_tables {
            let result = sqlx::query_scalar::<_, bool>(
                r"
                SELECT EXISTS (
                    SELECT FROM information_schema.tables 
                    WHERE table_schema = 'public' 
                    AND table_name = $1
                )
                ",
            )
            .bind(table_name)
            .fetch_one(&self.pool)
            .await?;

            if !result {
                tracing::debug!("Missing required table: {}", table_name);
                return Ok(false);
            }
        }

        // Check if event_type column exists in links (key indicator of new event-sourcing schema)
        let has_event_type = sqlx::query_scalar::<_, bool>(
            r"
            SELECT EXISTS (
                SELECT FROM information_schema.columns 
                WHERE table_schema = 'public' 
                AND table_name = 'links' 
                AND column_name = 'event_type'
            )
            ",
        )
        .fetch_one(&self.pool)
        .await?;

        if !has_event_type {
            tracing::debug!("links missing event_type column - old schema detected");
            return Ok(false);
        }

        Ok(true)
    }

    /// Verify database schema or return helpful error
    pub async fn verify_schema_or_error(&self) -> Result<()> {
        if !self.is_schema_initialized().await? {
            return Err(SnapRagError::Custom(
                "❌ Database schema not initialized!\n\n\
                 Please run the following command to initialize the database:\n\n\
                 \x1b[1;32msnaprag init --force\x1b[0m\n\n\
                 Then start sync again."
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Initialize database schema
    pub async fn init_schema(&self) -> Result<()> {
        // Create user_profiles table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_profiles (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fid BIGINT UNIQUE NOT NULL,
                username VARCHAR(255),
                display_name VARCHAR(255),
                bio TEXT,
                pfp_url TEXT,
                banner_url TEXT,
                location VARCHAR(255),
                website_url TEXT,
                twitter_username VARCHAR(255),
                github_username VARCHAR(255),
                primary_address_ethereum VARCHAR(42),
                primary_address_solana VARCHAR(44),
                profile_token VARCHAR(255),
                profile_embedding VECTOR(1536),
                bio_embedding VECTOR(1536),
                interests_embedding VECTOR(1536),
                last_updated_timestamp BIGINT NOT NULL,
                last_updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                shard_id INTEGER,
                block_height BIGINT,
                transaction_fid BIGINT
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Add tracking columns if they don't exist (for existing tables)
        sqlx::query("ALTER TABLE user_profiles ADD COLUMN IF NOT EXISTS shard_id INTEGER")
            .execute(&self.pool)
            .await?;
        sqlx::query("ALTER TABLE user_profiles ADD COLUMN IF NOT EXISTS block_height BIGINT")
            .execute(&self.pool)
            .await?;
        sqlx::query("ALTER TABLE user_profiles ADD COLUMN IF NOT EXISTS transaction_fid BIGINT")
            .execute(&self.pool)
            .await?;

        // Create user_profile_snapshots table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_profile_snapshots (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fid BIGINT NOT NULL,
                snapshot_timestamp BIGINT NOT NULL,
                message_hash BYTEA NOT NULL,
                username VARCHAR(255),
                display_name VARCHAR(255),
                bio TEXT,
                pfp_url TEXT,
                banner_url TEXT,
                location VARCHAR(255),
                website_url TEXT,
                twitter_username VARCHAR(255),
                github_username VARCHAR(255),
                primary_address_ethereum VARCHAR(42),
                primary_address_solana VARCHAR(44),
                profile_token VARCHAR(255),
                profile_embedding VECTOR(1536),
                bio_embedding VECTOR(1536),
                interests_embedding VECTOR(1536),
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                UNIQUE(fid, snapshot_timestamp)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create user_data_changes table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_data_changes (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fid BIGINT NOT NULL,
                data_type SMALLINT NOT NULL,
                old_value TEXT,
                new_value TEXT NOT NULL,
                change_timestamp BIGINT NOT NULL,
                message_hash BYTEA NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create username_proofs table (updated to match 000_complete_init.sql)
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS username_proofs (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fid BIGINT NOT NULL,
                username TEXT NOT NULL,
                owner BYTEA NOT NULL,
                signature BYTEA NOT NULL,
                timestamp BIGINT NOT NULL,
                username_type SMALLINT NOT NULL,
                message_hash BYTEA UNIQUE NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                shard_id INTEGER,
                block_height BIGINT,
                transaction_fid BIGINT,
                UNIQUE(fid, username_type)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // ❌ user_activity_timeline table removed for performance
        // All activity data is in specialized tables (casts, links, reactions, etc.)

        // Create user_profile_trends table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS user_profile_trends (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fid BIGINT NOT NULL,
                trend_period VARCHAR(20) NOT NULL,
                trend_date DATE NOT NULL,
                profile_changes_count INTEGER DEFAULT 0,
                bio_changes_count INTEGER DEFAULT 0,
                username_changes_count INTEGER DEFAULT 0,
                activity_score FLOAT DEFAULT 0.0,
                engagement_score FLOAT DEFAULT 0.0,
                profile_embedding VECTOR(1536),
                bio_embedding VECTOR(1536),
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                UNIQUE(fid, trend_period, trend_date)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        self.create_indexes().await?;

        Ok(())
    }

    async fn create_indexes(&self) -> Result<()> {
        // 🚀 OPTIMIZATION: Use CONCURRENTLY and skip slow index checks
        // Only create truly essential indexes, others should be in migrations

        // User profiles - essential unique constraint index (auto-created)
        // idx_user_profiles_fid already exists via UNIQUE constraint

        // Profile snapshots - only if needed for queries
        sqlx::query("CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_profile_snapshots_fid_timestamp ON user_profile_snapshots(fid, snapshot_timestamp DESC)")
            .execute(&self.pool)
            .await.ok(); // Ignore errors if already exists

        tracing::debug!("Essential indexes ensured");
        Ok(())
    }
}
