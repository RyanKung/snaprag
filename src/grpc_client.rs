//! gRPC client for connecting to snapchain HubService

use anyhow::Result;
use crate::generated::request_response::{ShardChunksRequest, ShardChunksResponse};
use crate::generated::hub_service_client::HubServiceClient as GeneratedHubServiceClient;
use crate::generated::grpc_client::{ShardChunksRequest as GrpcShardChunksRequest, ShardChunksResponse as GrpcShardChunksResponse};

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

    /// Convert protobuf ShardChunksRequest to gRPC ShardChunksRequest
    fn convert_request(request: ShardChunksRequest) -> GrpcShardChunksRequest {
        let mut grpc_request = GrpcShardChunksRequest::default();
        grpc_request.shard_id = request.get_shard_id() as u32;
        grpc_request.start_block_number = request.get_start_block_number();
        grpc_request.stop_block_number = Some(request.get_stop_block_number());
        grpc_request
    }

    /// Convert gRPC ShardChunksResponse to protobuf ShardChunksResponse
    fn convert_response(response: GrpcShardChunksResponse) -> ShardChunksResponse {
        let mut protobuf_response = ShardChunksResponse::new();
        // Note: This is a simplified conversion - in reality you'd need to convert
        // all the fields from the gRPC response to the protobuf response
        protobuf_response
    }

    /// Get shard chunks from the snapchain service
    pub async fn get_shard_chunks(
        &mut self,
        request: ShardChunksRequest,
    ) -> Result<ShardChunksResponse> {
        println!("Making gRPC GetShardChunks request...");
        
        // Convert protobuf request to gRPC request
        let grpc_request = Self::convert_request(request);
        let tonic_request = tonic::Request::new(grpc_request);
        
        // Make the gRPC call using the generated client
        let response = self.client
            .get_shard_chunks(tonic_request)
            .await
            .map_err(|e| anyhow::anyhow!("gRPC call failed: {}", e))?;
        
        println!("Received gRPC response successfully");
        
        // Extract the response data and convert back to protobuf format
        let grpc_response_data = response.into_inner();
        let protobuf_response = Self::convert_response(grpc_response_data);
        Ok(protobuf_response)
    }
}