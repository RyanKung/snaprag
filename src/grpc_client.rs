//! gRPC client for connecting to snapchain HubService

use anyhow::Result;
use crate::generated::grpc_client::{ShardChunksRequest, ShardChunksResponse};
use crate::generated::grpc_client::hub_service_client::HubServiceClient as GeneratedHubServiceClient;

/// Wrapper around the generated gRPC client for HubService
pub struct HubServiceClient {
    client: GeneratedHubServiceClient<tonic::transport::Channel>,
}

impl HubServiceClient {
    /// Create a new gRPC client
    pub async fn new(endpoint: &str) -> Result<Self> {
        // Parse the endpoint and ensure it has the correct format
        let endpoint_url = if endpoint.starts_with("http://") {
            endpoint.to_string()
        } else {
            format!("http://{}", endpoint)
        };
        
        println!("Creating gRPC client for endpoint: {}", endpoint_url);
        
        // Use the generated gRPC client
        let client = GeneratedHubServiceClient::connect(endpoint_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to gRPC endpoint: {}", e))?;
        
        Ok(Self { client })
    }

    /// Get shard chunks from the snapchain service
    pub async fn get_shard_chunks(
        &mut self,
        request: ShardChunksRequest,
    ) -> Result<ShardChunksResponse> {
        println!("Making gRPC GetShardChunks request...");
        
        // Make the gRPC call using the generated client
        let response = self.client
            .get_shard_chunks(request)
            .await
            .map_err(|e| anyhow::anyhow!("gRPC call failed: {}", e))?;
        
        println!("Received gRPC response successfully");
        
        // Extract the response data
        let response_data = response.into_inner();
        Ok(response_data)
    }
}