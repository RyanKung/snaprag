//! Snapchain gRPC client for synchronization

use crate::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import generated protobuf types (these would be generated from .proto files)
// For now, we'll use placeholder types until we generate the actual protobuf bindings

/// Placeholder for generated protobuf types
pub mod proto {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Block {
        pub header: Option<BlockHeader>,
        pub hash: Vec<u8>,
        pub transactions: Vec<Transaction>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BlockHeader {
        pub height: Option<Height>,
        pub timestamp: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Height {
        pub shard_index: u32,
        pub block_number: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Transaction {
        pub fid: u64,
        pub user_messages: Vec<Message>,
        pub system_messages: Vec<ValidatorMessage>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Message {
        pub data: Option<MessageData>,
        pub hash: Vec<u8>,
        pub signature: Vec<u8>,
        pub signer: Vec<u8>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MessageData {
        pub r#type: i32, // MessageType enum
        pub fid: u64,
        pub timestamp: u32,
        pub network: i32,                    // FarcasterNetwork enum
        pub body: Option<serde_json::Value>, // Oneof body
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ValidatorMessage {
        pub on_chain_event: Option<OnChainEvent>,
        pub fname_transfer: Option<FnameTransfer>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OnChainEvent {
        pub r#type: i32,
        pub block_number: u64,
        pub block_hash: Vec<u8>,
        pub transaction_hash: Vec<u8>,
        pub log_index: u32,
        pub fid: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FnameTransfer {
        pub id: u64,
        pub from_fid: u64,
        pub to_fid: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ShardChunk {
        pub header: Option<ShardHeader>,
        pub hash: Vec<u8>,
        pub transactions: Vec<Transaction>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ShardHeader {
        pub height: Option<Height>,
        pub timestamp: u64,
        pub parent_hash: Vec<u8>,
        pub shard_root: Vec<u8>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HubEvent {
        pub id: u64,
        pub r#type: i32, // HubEventType enum
        pub block_number: u64,
        pub block_hash: Vec<u8>,
        pub block_timestamp: u64,
        pub transaction_hash: Vec<u8>,
        pub log_index: u32,
        pub fid: u64,
        pub message: Option<Message>,
    }

    // Request/Response types
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BlocksRequest {
        pub shard_id: u32,
        pub start_block_number: u64,
        pub stop_block_number: Option<u64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct ShardChunksRequest {
        pub shard_id: u32,
        pub start_block_number: u64,
        pub stop_block_number: Option<u64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ShardChunksResponse {
        pub shard_chunks: Vec<ShardChunk>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GetInfoRequest {}

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GetInfoResponse {
        pub version: String,
        pub db_stats: Option<DbStats>,
        pub peer_id: String,
        pub num_shards: u32,
        pub shard_infos: Vec<ShardInfo>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DbStats {
        pub num_messages: u64,
        pub num_fid_registrations: u64,
        pub approx_size: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ShardInfo {
        pub shard_id: u32,
        pub max_height: u64,
        pub num_messages: u64,
        pub num_fid_registrations: u64,
        pub approx_size: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SubscribeRequest {
        pub event_types: Vec<i32>, // HubEventType enum values
        pub from_id: Option<u64>,
        pub shard_index: Option<u32>,
    }

    // Replication service types
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GetShardSnapshotMetadataRequest {
        pub shard_id: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GetShardSnapshotMetadataResponse {
        pub snapshots: Vec<ShardSnapshotMetadata>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ShardSnapshotMetadata {
        pub shard_id: u32,
        pub height: u64,
        pub timestamp: u64,
        pub shard_chunk: Option<ShardChunk>,
        pub block: Option<Block>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GetShardTransactionsRequest {
        pub shard_id: u32,
        pub height: u64,
        pub trie_virtual_shard: u32,
        pub page_token: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GetShardTransactionsResponse {
        pub trie_messages: Vec<ShardTrieEntryWithMessage>,
        pub fid_account_roots: Vec<FidAccountRootHash>,
        pub next_page_token: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FidsResponse {
        pub fids: Vec<u64>,
        pub next_page_token: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ShardTrieEntryWithMessage {
        pub trie_key: Vec<u8>,
        pub user_message: Option<Message>,
        pub on_chain_event: Option<OnChainEvent>,
        pub fname_transfer: Option<FnameTransfer>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FidAccountRootHash {
        pub fid: u64,
        pub account_root: Vec<u8>,
    }
}

/// Snapchain gRPC client
#[derive(Clone)]
pub struct SnapchainClient {
    client: Client,
    base_url: String,
}

impl SnapchainClient {
    /// Create a new Snapchain client
    pub async fn new(endpoint: &str) -> Result<Self> {
        let client = Client::new();
        let base_url = endpoint.trim_end_matches('/').to_string();

        Ok(Self { client, base_url })
    }

    /// Create a new Snapchain client from AppConfig
    pub async fn from_config(config: &crate::AppConfig) -> Result<Self> {
        Self::new(config.snapchain_endpoint()).await
    }

    /// Get node information
    pub async fn get_info(&self) -> Result<proto::GetInfoResponse> {
        let url = format!("{}/v1/info", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(crate::errors::SnapRagError::Custom(format!(
                "Failed to get info: HTTP {}",
                response.status()
            )));
        }

        let info: InfoResponse = response.json().await?;
        
        Ok(proto::GetInfoResponse {
            version: info.version.unwrap_or_else(|| "unknown".to_string()),
            db_stats: Some(proto::DbStats {
                num_messages: info.db_stats.as_ref().map(|s| s.num_messages).unwrap_or(0),
                num_fid_registrations: info.db_stats.as_ref().map(|s| s.num_fid_registrations).unwrap_or(0),
                approx_size: info.db_stats.as_ref().map(|s| s.approx_size).unwrap_or(0),
            }),
            peer_id: info.peer_id.unwrap_or_else(|| "unknown".to_string()),
            num_shards: info.num_shards.unwrap_or(1),
            shard_infos: vec![], // TODO: Parse from response if available
        })
    }

    /// Get blocks for a shard
    pub async fn get_blocks(&self, _request: proto::BlocksRequest) -> Result<Vec<proto::Block>> {
        // TODO: Implement actual gRPC streaming call
        // For now, return empty result
        Ok(vec![])
    }

    /// Get shard chunks for a shard
    pub async fn get_shard_chunks(
        &self,
        _request: proto::ShardChunksRequest,
    ) -> Result<proto::ShardChunksResponse> {
        // TODO: Implement actual gRPC call
        // For now, return empty result
        Ok(proto::ShardChunksResponse {
            shard_chunks: vec![],
        })
    }

    /// Subscribe to real-time events
    pub async fn subscribe(
        &self,
        _request: proto::SubscribeRequest,
    ) -> Result<Vec<proto::HubEvent>> {
        // TODO: Implement actual gRPC streaming call
        // For now, return empty result
        Ok(vec![])
    }

    /// Get shard snapshot metadata (for replication-based sync)
    pub async fn get_shard_snapshot_metadata(
        &self,
        _request: proto::GetShardSnapshotMetadataRequest,
    ) -> Result<proto::GetShardSnapshotMetadataResponse> {
        // TODO: Implement actual gRPC call
        // For now, return empty result
        Ok(proto::GetShardSnapshotMetadataResponse { snapshots: vec![] })
    }

    /// Get shard transactions (for replication-based sync)
    pub async fn get_shard_transactions(
        &self,
        _request: proto::GetShardTransactionsRequest,
    ) -> Result<proto::GetShardTransactionsResponse> {
        // TODO: Implement actual gRPC call
        // For now, return empty result
        Ok(proto::GetShardTransactionsResponse {
            trie_messages: vec![],
            fid_account_roots: vec![],
            next_page_token: None,
        })
    }

    /// Get links by target FID (like your curl example)
    pub async fn get_links_by_target_fid(
        &self,
        target_fid: u64,
        link_type: &str,
        page_size: Option<u32>,
        next_page_token: Option<&str>,
    ) -> Result<LinksByTargetFidResponse> {
        let mut url = format!(
            "{}/v1/linksByTargetFid?target_fid={}&link_type={}",
            self.base_url, target_fid, link_type
        );

        if let Some(size) = page_size {
            url.push_str(&format!("&pageSize={}", size));
        }

        if let Some(token) = next_page_token {
            url.push_str(&format!("&nextPageToken={}", token));
        }

        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(crate::errors::SnapRagError::Custom(format!(
                "Failed to get links by target FID: HTTP {}",
                response.status()
            )));
        }

        let links_response: LinksByTargetFidResponse = response.json().await?;
        Ok(links_response)
    }

    /// Get casts by FID
    pub async fn get_casts_by_fid(
        &self,
        fid: u64,
        page_size: Option<u32>,
        next_page_token: Option<&str>,
    ) -> Result<CastsByFidResponse> {
        let mut url = format!("{}/v1/castsByFid?fid={}", self.base_url, fid);

        if let Some(size) = page_size {
            url.push_str(&format!("&pageSize={}", size));
        }

        if let Some(token) = next_page_token {
            url.push_str(&format!("&nextPageToken={}", token));
        }

        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(crate::errors::SnapRagError::Custom(format!(
                "Failed to get casts by FID: HTTP {}",
                response.status()
            )));
        }

        let casts_response: CastsByFidResponse = response.json().await?;
        Ok(casts_response)
    }

    /// Get user data by FID
    pub async fn get_user_data_by_fid(
        &self,
        fid: u64,
        user_data_type: Option<&str>,
    ) -> Result<UserDataByFidResponse> {
        let mut url = format!("{}/v1/userDataByFid?fid={}", self.base_url, fid);

        if let Some(data_type) = user_data_type {
            url.push_str(&format!("&user_data_type={}", data_type));
        }

        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(crate::errors::SnapRagError::Custom(format!(
                "Failed to get user data by FID: HTTP {}",
                response.status()
            )));
        }

        let user_data_response: UserDataByFidResponse = response.json().await?;
        Ok(user_data_response)
    }

    /// Get all FIDs for a specific shard
    pub async fn get_fids_by_shard(
        &self,
        shard_id: u32,
        page_size: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<proto::FidsResponse> {
        let mut url = format!("{}/v1/fids", self.base_url);
        
        // Add query parameters
        let mut params = vec![format!("shard_id={}", shard_id)];
        if let Some(size) = page_size {
            params.push(format!("pageSize={}", size));
        }
        if let Some(token) = page_token {
            params.push(format!("pageToken={}", token));
        }
        if !params.is_empty() {
            url.push_str("?");
            url.push_str(&params.join("&"));
        }

        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(crate::errors::SnapRagError::Custom(format!(
                "Failed to get FIDs for shard {}: HTTP {}",
                shard_id, response.status()
            )));
        }

        let fids_response: proto::FidsResponse = response.json().await?;
        Ok(fids_response)
    }

    /// Get all FIDs across all shards (for comprehensive sync)
    pub async fn get_all_fids(&self) -> Result<Vec<u64>> {
        let mut all_fids = std::collections::HashSet::new();
        
        // Get node info to determine number of shards
        let info = self.get_info().await?;
        
        // Collect FIDs from user shards (skip shard 0 which is the block shard)
        for shard_id in 1..info.num_shards {
            let mut page_token: Option<String> = None;
            
            loop {
                let response = self.get_fids_by_shard(shard_id, Some(1000), page_token.as_deref()).await?;
                
                // Add FIDs to our set
                for fid in response.fids {
                    all_fids.insert(fid);
                }
                
                // Check if there are more pages
                page_token = response.next_page_token.clone();
                if page_token.is_none() {
                    break;
                }
            }
        }
        
        let mut fids_vec: Vec<u64> = all_fids.into_iter().collect();
        fids_vec.sort();
        
        tracing::info!("Discovered {} unique FIDs across {} shards", fids_vec.len(), info.num_shards);
        Ok(fids_vec)
    }
}

// Response types for the HTTP API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoResponse {
    pub version: Option<String>,
    #[serde(rename = "dbStats")]
    pub db_stats: Option<DbStatsResponse>,
    #[serde(rename = "peer_id")]
    pub peer_id: Option<String>,
    #[serde(rename = "numShards")]
    pub num_shards: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStatsResponse {
    #[serde(rename = "numMessages")]
    pub num_messages: u64,
    #[serde(rename = "numFidRegistrations")]
    pub num_fid_registrations: u64,
    #[serde(rename = "approxSize")]
    pub approx_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinksByTargetFidResponse {
    pub messages: Vec<FarcasterMessage>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastsByFidResponse {
    pub messages: Vec<FarcasterMessage>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDataByFidResponse {
    pub messages: Vec<FarcasterMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FarcasterMessage {
    pub data: Option<FarcasterMessageData>,
    pub hash: String,
    #[serde(rename = "hashScheme")]
    pub hash_scheme: String,
    pub signature: String,
    #[serde(rename = "signatureScheme")]
    pub signature_scheme: String,
    pub signer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FarcasterMessageData {
    #[serde(rename = "type")]
    pub message_type: String,
    pub fid: u64,
    pub timestamp: u64,
    pub network: String,
    #[serde(flatten)]
    pub body: HashMap<String, serde_json::Value>,
}
