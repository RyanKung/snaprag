use super::Database;
use crate::models::UsernameProof;
use crate::models::UsernameType;
use crate::Result;

impl Database {
    /// Create or update username proof
    pub async fn upsert_username_proof(
        &self,
        fid: i64,
        username: String,
        username_type: UsernameType,
        owner: Vec<u8>,
        signature: Vec<u8>,
        timestamp: i64,
    ) -> Result<UsernameProof> {
        let proof = sqlx::query_as::<_, UsernameProof>(
            r"
            INSERT INTO username_proofs (fid, username, username_type, owner, signature, timestamp, message_hash)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (fid, username_type)
            DO UPDATE SET
                username = EXCLUDED.username,
                owner = EXCLUDED.owner,
                signature = EXCLUDED.signature,
                timestamp = EXCLUDED.timestamp,
                created_at = NOW()
            RETURNING *
            "
        )
        .bind(fid)
        .bind(username)
        .bind(username_type as i16)
        .bind(owner)
        .bind(signature)
        .bind(timestamp)
        .bind(vec![0u8; 32]) // Placeholder message_hash
        .fetch_one(&self.pool)
        .await?;

        Ok(proof)
    }

    /// Get username proof by FID and type
    pub async fn get_username_proof(
        &self,
        fid: i64,
        username_type: UsernameType,
    ) -> Result<Option<UsernameProof>> {
        let proof = sqlx::query_as::<_, UsernameProof>(
            "SELECT * FROM username_proofs WHERE fid = $1 AND username_type = $2",
        )
        .bind(fid)
        .bind(username_type as i32)
        .fetch_optional(&self.pool)
        .await?;

        Ok(proof)
    }

    /// Get all username proofs for a user
    pub async fn get_user_username_proofs(&self, fid: i64) -> Result<Vec<UsernameProof>> {
        let proofs = sqlx::query_as::<_, UsernameProof>(
            "SELECT * FROM username_proofs WHERE fid = $1 ORDER BY timestamp DESC",
        )
        .bind(fid)
        .fetch_all(&self.pool)
        .await?;

        Ok(proofs)
    }
}
