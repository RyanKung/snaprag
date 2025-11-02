use std::time::Duration;

use redis::AsyncCommands;

use crate::config::RedisConfig;

#[derive(Clone)]
pub struct RedisClient {
    client: redis::Client,
    namespace: String,
    default_ttl: Duration,
    refresh_channel: String,
    stale_threshold: Duration,
}

impl RedisClient {
    pub fn connect(config: &RedisConfig) -> crate::Result<Self> {
        let client = redis::Client::open(config.url.as_str())
            .map_err(|e| crate::SnapRagError::Custom(format!("Redis open error: {e}")))?;

        Ok(Self {
            client,
            namespace: config.namespace.clone(),
            default_ttl: Duration::from_secs(config.default_ttl_secs),
            refresh_channel: config.refresh_channel.clone(),
            stale_threshold: Duration::from_secs(config.stale_threshold_secs),
        })
    }

    fn key(&self, k: &str) -> String {
        format!("{}{}", self.namespace, k)
    }

    pub async fn get_json(&self, key: &str) -> crate::Result<Option<String>> {
        let k = self.key(key);
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|e| crate::SnapRagError::Custom(format!("Redis connect error: {e}")))?;
        let val: Option<String> = conn
            .get(k)
            .await
            .map_err(|e| crate::SnapRagError::Custom(format!("Redis GET error: {e}")))?;
        Ok(val)
    }

    pub async fn set_json_with_ttl(
        &self,
        key: &str,
        json: &str,
        ttl: Option<Duration>,
    ) -> crate::Result<()> {
        let k = self.key(key);
        let ttl = ttl.unwrap_or(self.default_ttl);
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|e| crate::SnapRagError::Custom(format!("Redis connect error: {e}")))?;
        redis::pipe()
            .set(&k, json)
            .ignore()
            .expire(&k, ttl.as_secs() as i64)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| crate::SnapRagError::Custom(format!("Redis SET/EXPIRE error: {e}")))?;
        Ok(())
    }

    pub async fn ttl_secs(&self, key: &str) -> crate::Result<Option<i64>> {
        let k = self.key(key);
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|e| crate::SnapRagError::Custom(format!("Redis connect error: {e}")))?;
        let ttl: i64 = conn
            .ttl(k)
            .await
            .map_err(|e| crate::SnapRagError::Custom(format!("Redis TTL error: {e}")))?;
        if ttl < 0 {
            Ok(None)
        } else {
            Ok(Some(ttl))
        }
    }

    pub fn stale_threshold(&self) -> Duration {
        self.stale_threshold
    }

    pub async fn publish_refresh(&self, subject: &str, key: &str) -> crate::Result<()> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|e| crate::SnapRagError::Custom(format!("Redis connect error: {e}")))?;
        let payload = format!("{}|{}", subject, key);
        conn.publish::<_, _, ()>(&self.refresh_channel, payload)
            .await
            .map_err(|e| crate::SnapRagError::Custom(format!("Redis PUBLISH error: {e}")))?;
        Ok(())
    }
}
