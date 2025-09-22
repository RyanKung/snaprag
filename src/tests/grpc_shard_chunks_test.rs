//! Test for parsing gRPC GetShardChunks response data
//! 
//! This test demonstrates how to parse ShardChunksResponse data
//! from shard 1, requesting blocks from 0 to 42.

use prost::Message;
use crate::generated::grpc_client::{ShardChunksRequest, ShardChunksResponse, ShardChunk, ShardHeader, Height, Transaction, Commits, CommitSignature, ShardHash, Message as FarcasterMessage, MessageData, MessageType, FarcasterNetwork, CastAddBody, HashScheme, SignatureScheme};
use crate::grpc_client::HubServiceClient;
use crate::config::AppConfig;
use tokio;

#[tokio::test]
async fn test_parse_shard_chunks_response_real() {
    // Load configuration to get the gRPC endpoint
    let config = match AppConfig::load() {
        Ok(config) => config,
        Err(e) => {
            println!("⚠️  Could not load config: {}. Using default endpoint.", e);
            AppConfig::default()
        }
    };

    let grpc_endpoint = config.snapchain_grpc_endpoint();
    println!("Using gRPC endpoint: {}", grpc_endpoint);

    // Create gRPC client
    let mut client = match HubServiceClient::new(grpc_endpoint).await {
        Ok(client) => {
            println!("✅ Successfully connected to gRPC service");
            client
        }
        Err(e) => {
            println!("❌ Failed to connect to gRPC service: {}", e);
            println!("⚠️  Skipping real gRPC test - service may not be running");
            return;
        }
    };

    // Create a sample ShardChunksRequest for shard 1, blocks 0 to 10 (smaller range for testing)
    let mut request = ShardChunksRequest::default();
    request.shard_id = 1;
    request.start_block_number = 0;
    request.stop_block_number = Some(10);

    println!("Making gRPC GetShardChunks request for shard {}, blocks {}-{}", 
             request.shard_id, 
             request.start_block_number, 
             request.stop_block_number.unwrap_or(0));

    // Make the actual gRPC call
    let response = match client.get_shard_chunks(request).await {
        Ok(response) => {
            println!("✅ Successfully received gRPC response");
            response
        }
        Err(e) => {
            println!("❌ gRPC call failed: {}", e);
            println!("⚠️  This might be expected if the snapchain service is not running or has no data");
            return;
        }
    };

    // Verify the response
    let chunk_count = response.shard_chunks.len();
    println!("Received {} shard chunks", chunk_count);

    if chunk_count == 0 {
        println!("⚠️  No chunks received - this might be expected if the service has no data for the requested range");
        return;
    }

    // Verify the parsed data
    for (i, chunk) in response.shard_chunks.iter().enumerate() {
               println!("Chunk {}: Block {}, {} transactions, timestamp {}", 
                        i,
                        chunk.header.as_ref().unwrap().height.as_ref().unwrap().block_number,
                        chunk.transactions.len(),
                        chunk.header.as_ref().unwrap().timestamp);
        
        // Verify header
        // Note: shard_index and block_number are u32, so >= 0 is always true
        // assert!(chunk.header.as_ref().unwrap().height.as_ref().unwrap().shard_index >= 0);
        // assert!(chunk.header.as_ref().unwrap().height.as_ref().unwrap().block_number >= 0);
        assert!(chunk.header.as_ref().unwrap().timestamp > 0);
        
        // Verify hash
        assert!(!chunk.hash.is_empty());
        
        // Verify transactions (if any)
        for (tx_idx, transaction) in chunk.transactions.iter().enumerate() {
                   println!("  Transaction {}: FID {}, {} user messages", 
                            tx_idx, 
                            transaction.fid,
                            transaction.user_messages.len());
            
            // Note: fid is u32, so >= 0 is always true
            // assert!(transaction.fid >= 0);
            
            // Verify user messages (if any)
            for (msg_idx, message) in transaction.user_messages.iter().enumerate() {
                println!("    Message {}: Type {:?}, FID {}", 
                         msg_idx,
                         message.data.as_ref().unwrap().r#type,
                         message.data.as_ref().unwrap().fid);
                
                assert!(message.data.as_ref().unwrap().fid > 0);
                assert!(!message.hash.is_empty());
            }
        }
        
        // Verify commits
        // Note: shard_index and block_number are u32, so >= 0 is always true
        // assert!(chunk.commits.as_ref().unwrap().height.as_ref().unwrap().shard_index >= 0);
        // assert!(chunk.commits.as_ref().unwrap().height.as_ref().unwrap().block_number >= 0);
        assert!(!chunk.commits.as_ref().unwrap().signatures.is_empty());
    }

    println!("✅ Successfully parsed {} real shard chunks from gRPC service", chunk_count);
}

#[test]
fn test_parse_shard_chunks_response_mock() {
    // Keep the original mock test for unit testing without external dependencies
    // Create a sample ShardChunksRequest for shard 1, blocks 0 to 42
    let mut request = ShardChunksRequest::default();
    request.shard_id = 1;
    request.start_block_number = 0;
    request.stop_block_number = Some(42);

    // Serialize the request to bytes (simulating gRPC call)
    let request_bytes = request.encode_to_vec();
    println!("Request bytes length: {}", request_bytes.len());

    // Create a sample ShardChunksResponse with mock data
    let mut response = ShardChunksResponse::default();
    
    // Create sample shard chunks for blocks 0-42
    for block_num in 0..=42 {
        let mut shard_chunk = ShardChunk::default();
        
        // Create ShardHeader
        let mut header = ShardHeader::default();
        let mut height = Height::default();
        height.shard_index = 1;
        height.block_number = block_num;
        header.height = Some(height);
        header.timestamp = 1640995200 + block_num * 4; // Mock timestamp
        header.parent_hash = vec![0u8; 32]; // Mock parent hash
        header.shard_root = vec![0u8; 32]; // Mock shard root
        shard_chunk.header = Some(header);
        
        // Set chunk hash
        shard_chunk.hash = vec![block_num as u8; 32];
        
        // Create sample transactions
        let transactions = Vec::new();
        for fid in 1..=3 {
            let mut transaction = Transaction::default();
            transaction.fid = fid;
            
            // Create sample user messages
            let mut user_messages = Vec::new();
            let mut message = FarcasterMessage::default();
            let mut data = MessageData::default();
            data.r#type = MessageType::CastAdd as i32;
            data.fid = fid;
            data.timestamp = (1640995200 + block_num * 4) as u32;
            data.network = FarcasterNetwork::Mainnet as i32;
            
            let mut cast_add_body = CastAddBody::default();
            cast_add_body.text = format!("Test cast from FID {} in block {}", fid, block_num);
            data.body = Some(crate::generated::grpc_client::message_data::Body::CastAddBody(cast_add_body));
            
            message.data = Some(data);
            message.hash = vec![fid as u8; 32];
            message.hash_scheme = HashScheme::Blake3 as i32;
            message.signature = vec![fid as u8; 64];
            message.signature_scheme = SignatureScheme::Ed25519 as i32;
            message.signer = vec![fid as u8; 32];
            
            user_messages.push(message);
            transaction.user_messages = user_messages;
            transaction.account_root = vec![fid as u8; 32];
        }
        shard_chunk.transactions = transactions;
        
        // Create commits
        let mut commits = Commits::default();
        let mut commit_height = Height::default();
        commit_height.shard_index = 1;
        commit_height.block_number = block_num;
        commits.height = Some(commit_height);
        commits.round = 0;
        
        let mut shard_hash = ShardHash::default();
        shard_hash.shard_index = 1;
        shard_hash.hash = vec![block_num as u8; 32];
        commits.value = Some(shard_hash);
        
        // Add commit signatures
        let mut signatures = Vec::new();
        for i in 0..3 {
            let mut signature = CommitSignature::default();
            signature.signer = vec![i as u8; 32];
            signature.signature = vec![i as u8; 64];
            signatures.push(signature);
        }
        commits.signatures = signatures;
        shard_chunk.commits = Some(commits);
        
        response.shard_chunks.push(shard_chunk);
    }

    // Serialize the response to bytes (simulating gRPC response)
    let response_bytes = response.encode_to_vec();
    println!("Response bytes length: {}", response_bytes.len());

    // Parse the response back from bytes
    let parsed_response = ShardChunksResponse::decode(&response_bytes[..])
        .expect("Failed to parse response");

    // Verify the parsed data
    assert_eq!(parsed_response.shard_chunks.len(), 43); // 0 to 42 inclusive
    
    for (i, chunk) in parsed_response.shard_chunks.iter().enumerate() {
        let expected_block_num = i as u64;
        
        // Verify header
        assert_eq!(chunk.header.as_ref().unwrap().height.as_ref().unwrap().shard_index, 1);
        assert_eq!(chunk.header.as_ref().unwrap().height.as_ref().unwrap().block_number, expected_block_num);
        assert_eq!(chunk.header.as_ref().unwrap().timestamp, 1640995200 + expected_block_num * 4);
        
        // Verify hash
        assert_eq!(chunk.hash.len(), 32);
        assert_eq!(chunk.hash[0], expected_block_num as u8);
        
        // Verify transactions
        assert_eq!(chunk.transactions.len(), 3); // 3 FIDs per block
        
        for (fid_idx, transaction) in chunk.transactions.iter().enumerate() {
            let expected_fid = (fid_idx + 1) as u64;
            assert_eq!(transaction.fid, expected_fid);
            assert_eq!(transaction.user_messages.len(), 1);
            
            // Verify user message
            let message = &transaction.user_messages[0];
            assert_eq!(message.data.as_ref().unwrap().fid, expected_fid);
            assert_eq!(message.data.as_ref().unwrap().r#type, MessageType::CastAdd as i32);
            
            if let Some(crate::generated::grpc_client::message_data::Body::CastAddBody(cast_body)) = &message.data.as_ref().unwrap().body {
                assert!(cast_body.text.contains(&format!("FID {}", expected_fid)));
                assert!(cast_body.text.contains(&format!("block {}", expected_block_num)));
            }
        }
        
        // Verify commits
        assert_eq!(chunk.commits.as_ref().unwrap().height.as_ref().unwrap().shard_index, 1);
        assert_eq!(chunk.commits.as_ref().unwrap().height.as_ref().unwrap().block_number, expected_block_num);
        assert_eq!(chunk.commits.as_ref().unwrap().round, 0);
        assert_eq!(chunk.commits.as_ref().unwrap().signatures.len(), 3);
    }

    println!("✅ Successfully parsed {} mock shard chunks from shard 1, blocks 0-42", 
             parsed_response.shard_chunks.len());
    
    // Print summary information
    for chunk in parsed_response.shard_chunks.iter().take(5) {
        println!("Block {}: {} transactions, timestamp {}", 
                 chunk.header.as_ref().unwrap().height.as_ref().unwrap().block_number,
                 chunk.transactions.len(),
                 chunk.header.as_ref().unwrap().timestamp);
    }
    
    if parsed_response.shard_chunks.len() > 5 {
        println!("... and {} more blocks", parsed_response.shard_chunks.len() - 5);
    }
}


#[test]
fn test_shard_chunks_request_serialization() {
    // Test creating and serializing a ShardChunksRequest
    let mut request = ShardChunksRequest::default();
    request.shard_id = 1;
    request.start_block_number = 0;
    request.stop_block_number = Some(42);

    let bytes = request.encode_to_vec();
    let parsed = ShardChunksRequest::decode(&bytes[..])
        .expect("Failed to parse request");

    assert_eq!(parsed.shard_id, 1);
    assert_eq!(parsed.start_block_number, 0);
    assert_eq!(parsed.stop_block_number, Some(42));

    println!("✅ ShardChunksRequest serialization test passed");
}

#[test]
fn test_empty_shard_chunks_response() {
    // Test handling empty response
    let response = ShardChunksResponse::default();
    // No chunks added - empty response

    let bytes = response.encode_to_vec();
    let parsed = ShardChunksResponse::decode(&bytes[..])
        .expect("Failed to parse empty response");

    assert_eq!(parsed.shard_chunks.len(), 0);
    println!("✅ Empty ShardChunksResponse test passed");
}
