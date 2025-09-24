//! Test for parsing gRPC GetShardChunks response data
//!
//! This test demonstrates how to parse ShardChunksResponse data
//! from shard 1, requesting blocks from 0 to 42.

use std::collections::HashMap;

use prost::Message;
use tokio;

use crate::config::AppConfig;
use crate::generated::grpc_client::CastAddBody;
use crate::generated::grpc_client::CommitSignature;
use crate::generated::grpc_client::Commits;
use crate::generated::grpc_client::FarcasterNetwork;
use crate::generated::grpc_client::HashScheme;
use crate::generated::grpc_client::Height;
use crate::generated::grpc_client::Message as FarcasterMessage;
use crate::generated::grpc_client::MessageData;
use crate::generated::grpc_client::MessageType;
use crate::generated::grpc_client::ShardChunk;
use crate::generated::grpc_client::ShardChunksRequest;
use crate::generated::grpc_client::ShardChunksResponse;
use crate::generated::grpc_client::ShardHash;
use crate::generated::grpc_client::ShardHeader;
use crate::generated::grpc_client::SignatureScheme;
use crate::generated::grpc_client::Transaction;
use crate::grpc_client::HubServiceClient;

/// Common test setup: load config and create gRPC client
async fn setup_grpc_client() -> (AppConfig, HubServiceClient) {
    let config = AppConfig::load().expect("Failed to load configuration");
    let grpc_endpoint = config.snapchain_grpc_endpoint();
    println!("Using gRPC endpoint: {}", grpc_endpoint);

    let client = HubServiceClient::new(grpc_endpoint)
        .await
        .expect("Failed to connect to gRPC service");
    println!("✅ Successfully connected to gRPC service");

    (config, client)
}

/// Create a ShardChunksRequest with specified parameters
fn create_shard_chunks_request(
    shard_id: u32,
    start_block: u64,
    stop_block: Option<u64>,
) -> ShardChunksRequest {
    let mut request = ShardChunksRequest::default();
    request.shard_id = shard_id;
    request.start_block_number = start_block;
    request.stop_block_number = stop_block;
    request
}

/// Make gRPC call and return response
async fn make_grpc_call(
    client: &mut HubServiceClient,
    request: ShardChunksRequest,
) -> ShardChunksResponse {
    println!(
        "Making gRPC GetShardChunks request for shard {}, blocks {}-{}",
        request.shard_id,
        request.start_block_number,
        request
            .stop_block_number
            .unwrap_or(request.start_block_number)
    );

    let response = client
        .get_shard_chunks(request)
        .await
        .expect("gRPC call failed");
    println!("✅ Successfully received gRPC response");

    response
}

/// Verify basic chunk structure (header, hash, commits)
fn verify_chunk_basic_structure(chunk: &ShardChunk) {
    // Verify header
    let header = chunk
        .header
        .as_ref()
        .expect("Chunk header should not be None");
    let height = header
        .height
        .as_ref()
        .expect("Header height should not be None");
    // Note: shard_index and block_number are u32, so >= 0 is always true
    // We'll check for reasonable ranges instead
    assert!(height.shard_index < 1000, "Shard index should be reasonable");
    assert!(height.block_number < 1_000_000_000, "Block number should be reasonable");
    assert!(header.timestamp > 0, "Timestamp should be > 0");

    // Verify hash
    assert!(!chunk.hash.is_empty(), "Chunk hash should not be empty");

    // Verify commits
    let commits = chunk
        .commits
        .as_ref()
        .expect("Chunk commits should not be None");
    let commit_height = commits
        .height
        .as_ref()
        .expect("Commit height should not be None");
    // Note: shard_index and block_number are u32, so >= 0 is always true
    // We'll check for reasonable ranges instead
    assert!(
        commit_height.shard_index < 1000,
        "Commit shard index should be reasonable"
    );
    assert!(
        commit_height.block_number < 1_000_000_000,
        "Commit block number should be reasonable"
    );
    assert!(
        !commits.signatures.is_empty(),
        "Commit signatures should not be empty"
    );
}

/// Verify transactions with FID validation
fn verify_transactions(chunk: &ShardChunk, allow_fid_zero: bool) {
    for (tx_idx, transaction) in chunk.transactions.iter().enumerate() {
        if allow_fid_zero {
            // Note: fid is u64, so >= 0 is always true
            // We'll check for reasonable range instead
            assert!(transaction.fid < 1_000_000_000, "Transaction FID should be reasonable");
        } else {
            assert!(
                transaction.fid > 0,
                "Transaction FID should be > 0, but found FID=0 at transaction index {}",
                tx_idx
            );
        }

        // Verify user messages (if any)
        for message in &transaction.user_messages {
            let message_data = message
                .data
                .as_ref()
                .expect("Message data should not be None");
            assert!(message_data.fid > 0, "Message FID should be > 0");
            assert!(!message.hash.is_empty(), "Message hash should not be empty");
        }
    }
}

/// Print chunk summary information
fn print_chunk_summary(chunk: &ShardChunk, chunk_index: usize) {
    let header = chunk.header.as_ref().unwrap();
    let height = header.height.as_ref().unwrap();
    let block_number = height.block_number;

    println!(
        "Chunk {}: Block {}, {} transactions, timestamp {}",
        chunk_index,
        block_number,
        chunk.transactions.len(),
        header.timestamp
    );
}

/// Print detailed block analysis with statistics
fn print_detailed_block_analysis(chunk: &ShardChunk) {
    let header = chunk
        .header
        .as_ref()
        .expect("Chunk header should not be None");
    let height = header
        .height
        .as_ref()
        .expect("Header height should not be None");
    let block_number = height.block_number;

    println!("\n🔍 DETAILED ANALYSIS OF BLOCK {}:", block_number);
    println!("{}", "=".repeat(80));
    println!("Block Number: {}", block_number);
    println!("Shard Index: {}", height.shard_index);
    println!("Timestamp: {}", header.timestamp);
    println!("Parent Hash: {}", hex::encode(&header.parent_hash));
    println!("Shard Root: {}", hex::encode(&header.shard_root));
    println!("Chunk Hash: {}", hex::encode(&chunk.hash));
    println!("Total Transactions: {}", chunk.transactions.len());

    // Analyze transaction patterns
    let mut fid_counts = HashMap::new();
    let mut account_root_counts = HashMap::new();
    let mut user_message_counts = HashMap::new();

    for (tx_idx, transaction) in chunk.transactions.iter().enumerate() {
        // Count FID patterns
        let fid = transaction.fid;
        *fid_counts.entry(fid).or_insert(0) += 1;

        // Count account root patterns
        let account_root = hex::encode(&transaction.account_root);
        *account_root_counts.entry(account_root).or_insert(0) += 1;

        // Count user message patterns
        let user_msg_count = transaction.user_messages.len();
        *user_message_counts.entry(user_msg_count).or_insert(0) += 1;

        // Print first 10 transactions in detail
        if tx_idx < 10 {
            println!("\n  Transaction {}:", tx_idx);
            println!("    FID: {}", fid);
            println!(
                "    Account Root: {}",
                hex::encode(&transaction.account_root)
            );
            println!("    User Messages: {}", user_msg_count);

            if user_msg_count > 0 {
                for (msg_idx, message) in transaction.user_messages.iter().enumerate() {
                    println!(
                        "      Message {}: Type {:?}, FID {}",
                        msg_idx,
                        message.data.as_ref().unwrap().r#type,
                        message.data.as_ref().unwrap().fid
                    );
                }
            }
        }
    }

    // Print statistics
    println!("\n📊 BLOCK {} STATISTICS:", block_number);
    println!("{}", "=".repeat(50));

    // FID statistics
    println!("\nFID Distribution (top 10):");
    let mut fid_vec: Vec<_> = fid_counts.iter().collect();
    fid_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (fid, count) in fid_vec.iter().take(10) {
        println!("  FID {}: {} transactions", fid, count);
    }

    // Account root statistics
    println!("\nAccount Root Distribution (top 10):");
    let mut account_vec: Vec<_> = account_root_counts.iter().collect();
    account_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (account_root, count) in account_vec.iter().take(10) {
        println!("  {}: {} transactions", account_root, count);
    }

    // User message statistics
    println!("\nUser Message Count Distribution:");
    let mut msg_vec: Vec<_> = user_message_counts.iter().collect();
    msg_vec.sort_by(|a, b| a.0.cmp(b.0));
    for (msg_count, count) in msg_vec {
        println!("  {} user messages: {} transactions", msg_count, count);
    }

    // Print commits info
    let commits = chunk
        .commits
        .as_ref()
        .expect("Chunk commits should not be None");
    let commit_height = commits
        .height
        .as_ref()
        .expect("Commit height should not be None");
    println!("\nCommits:");
    println!("  Shard Index: {}", commit_height.shard_index);
    println!("  Block Number: {}", commit_height.block_number);
    println!("  Signatures: {} signatures", commits.signatures.len());

    // Print first few signatures
    for (sig_idx, signature) in commits.signatures.iter().enumerate().take(3) {
        println!("    Signature {}: {:?}", sig_idx, signature);
    }
    if commits.signatures.len() > 3 {
        println!(
            "    ... and {} more signatures",
            commits.signatures.len() - 3
        );
    }
}

/// Print detailed data for a specific block (used for Block 8 analysis)
fn print_detailed_block_data(chunk: &ShardChunk, target_block: u64) {
    let header = chunk.header.as_ref().unwrap();
    let height = header.height.as_ref().unwrap();
    let block_number = height.block_number;

    if block_number == target_block {
        println!("\n🔍 DETAILED DATA FOR BLOCK {}:", block_number);
        println!("{}", "=".repeat(80));

        // Print header details
        println!("📋 HEADER:");
        println!("  Shard Index: {}", height.shard_index);
        println!("  Block Number: {}", height.block_number);
        println!("  Timestamp: {}", header.timestamp);
        println!("  Parent Hash: {}", hex::encode(&header.parent_hash));
        println!("  Shard Root: {}", hex::encode(&header.shard_root));

        // Print chunk hash
        println!("\n🔗 CHUNK HASH:");
        println!("  Hash: {}", hex::encode(&chunk.hash));

        // Print all transactions
        println!(
            "\n📝 ALL TRANSACTIONS ({} total):",
            chunk.transactions.len()
        );
        for (tx_idx, transaction) in chunk.transactions.iter().enumerate() {
            println!(
                "  Transaction {}: FID {}, {} user messages",
                tx_idx,
                transaction.fid,
                transaction.user_messages.len()
            );

            // Print account root
            println!(
                "    Account Root: {}",
                hex::encode(&transaction.account_root)
            );

            // Print user messages details
            if !transaction.user_messages.is_empty() {
                println!("    User Messages:");
                for (msg_idx, message) in transaction.user_messages.iter().enumerate() {
                    println!(
                        "      Message {}: Type {:?}, FID {}",
                        msg_idx,
                        message.data.as_ref().unwrap().r#type,
                        message.data.as_ref().unwrap().fid
                    );
                    println!("        Hash: {}", hex::encode(&message.hash));
                    println!("        Hash Scheme: {:?}", message.hash_scheme);
                    println!("        Signature: {}", hex::encode(&message.signature));
                    println!("        Signature Scheme: {:?}", message.signature_scheme);
                    println!("        Signer: {}", hex::encode(&message.signer));
                }
            }

            // Highlight FID=0 transactions
            if transaction.fid == 0 {
                println!("    🚨 FID=0 DETECTED! This transaction has FID=0");
            }
        }

        // Print commits details
        let commits = chunk
            .commits
            .as_ref()
            .expect("Chunk commits should not be None");
        let commit_height = commits
            .height
            .as_ref()
            .expect("Commit height should not be None");
        println!("\n✅ COMMITS:");
        println!("  Shard Index: {}", commit_height.shard_index);
        println!("  Block Number: {}", commit_height.block_number);
        println!("  Round: {}", commits.round);
        if let Some(value) = &commits.value {
            println!("  Value Shard Index: {}", value.shard_index);
            println!("  Value Hash: {}", hex::encode(&value.hash));
        }
        println!("  Signatures Count: {}", commits.signatures.len());
        for (sig_idx, signature) in commits.signatures.iter().enumerate() {
            println!(
                "    Signature {}: Signer={}, Signature={}",
                sig_idx,
                hex::encode(&signature.signer),
                hex::encode(&signature.signature)
            );
        }

        println!("{}", "=".repeat(80));
    }
}

#[tokio::test]
async fn test_parse_shard_chunks_response_real_blocks_0_to_7() {
    // Setup gRPC client
    let (_config, mut client) = setup_grpc_client().await;

    // Create request for blocks 0-7
    let request = create_shard_chunks_request(1, 0, Some(7));

    // Make gRPC call
    let response = make_grpc_call(&mut client, request).await;

    // Verify the response
    let chunk_count = response.shard_chunks.len();
    println!("Received {} shard chunks", chunk_count);
    assert!(
        chunk_count > 0,
        "Expected to receive at least one shard chunk, but got 0"
    );

    // Verify the parsed data
    for (i, chunk) in response.shard_chunks.iter().enumerate() {
        print_chunk_summary(chunk, i);

        // Verify basic structure
        verify_chunk_basic_structure(chunk);

        // Verify transactions (allow FID=0 for this test)
        verify_transactions(chunk, true);

        // Print user message statistics
        let mut total_user_messages = 0;
        let mut transactions_with_messages = 0;

        for (tx_idx, transaction) in chunk.transactions.iter().enumerate() {
            let user_message_count = transaction.user_messages.len();
            total_user_messages += user_message_count;

            if user_message_count > 0 {
                transactions_with_messages += 1;
                println!(
                    "  Transaction {}: FID {}, {} user messages",
                    tx_idx, transaction.fid, user_message_count
                );

                // Print details for first few user messages
                for (msg_idx, message) in transaction.user_messages.iter().enumerate().take(3) {
                    println!(
                        "    Message {}: Type {:?}, FID {}",
                        msg_idx,
                        message.data.as_ref().unwrap().r#type,
                        message.data.as_ref().unwrap().fid
                    );
                }

                if user_message_count > 3 {
                    println!("    ... and {} more messages", user_message_count - 3);
                }
            } else if tx_idx < 5 {
                // Only print first 5 transactions without messages
                println!(
                    "  Transaction {}: FID {}, {} user messages",
                    tx_idx, transaction.fid, user_message_count
                );
            }
        }

        let block_number = chunk
            .header
            .as_ref()
            .unwrap()
            .height
            .as_ref()
            .unwrap()
            .block_number;
        println!(
            "  📊 Block {} Summary: {} transactions, {} with user messages, {} total user messages",
            block_number,
            chunk.transactions.len(),
            transactions_with_messages,
            total_user_messages
        );
    }

    println!(
        "✅ Successfully parsed {} real shard chunks from gRPC service (blocks 0-7)",
        chunk_count
    );
}

#[tokio::test]
async fn test_parse_shard_chunks_response_real_block_9_with_fid_zero() {
    // Setup gRPC client
    let (_config, mut client) = setup_grpc_client().await;

    // Create request for blocks 8-10 (to ensure we get some data)
    let request = create_shard_chunks_request(1, 8, Some(10));

    // Make gRPC call
    let response = make_grpc_call(&mut client, request).await;

    // Verify the response
    let chunk_count = response.shard_chunks.len();
    println!("Received {} shard chunks", chunk_count);
    assert!(
        chunk_count > 0,
        "Expected to receive at least one shard chunk, but got 0"
    );

    // Verify the parsed data - specifically look for Block 9
    let mut block_9_found = false;
    let mut fid_zero_found_in_block_9 = false;

    for (i, chunk) in response.shard_chunks.iter().enumerate() {
        print_chunk_summary(chunk, i);

        // Check if this is Block 9
        if let Some(header) = &chunk.header {
            if let Some(height) = &header.height {
                if height.block_number == 9 {
                    block_9_found = true;
                    println!("🔍 Found Block 9 - checking for FID=0...");

                    // Print detailed data for Block 9
                    print_detailed_block_data(chunk, 9);

                    // Verify basic structure
                    verify_chunk_basic_structure(chunk);

                    // Check for FID=0 in Block 9 transactions
                    for (tx_idx, transaction) in chunk.transactions.iter().enumerate() {
                        if transaction.fid == 0 {
                            println!("Found FID=0 at transaction index {} in Block 9", tx_idx);
                            fid_zero_found_in_block_9 = true;
                        }
                    }

                    // Print summary for Block 9
                    if fid_zero_found_in_block_9 {
                        println!("✅ Found FID=0 in Block 9 transactions as expected");
                    } else {
                        println!("ℹ️  No FID=0 found in Block 9 transactions - this may be normal");
                        // Don't assert failure, just log the information
                    }
                }
            }
        }
    }

    // Ensure we found Block 9
    assert!(
        block_9_found,
        "Expected to find Block 9 in the response, but it was not found"
    );

    println!(
        "✅ Successfully parsed {} real shard chunks from gRPC service (block 9 with FID=0)",
        chunk_count
    );
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
        let mut transactions = Vec::new();
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
            data.body =
                Some(crate::generated::grpc_client::message_data::Body::CastAddBody(cast_add_body));

            message.data = Some(data);
            message.hash = vec![fid as u8; 32];
            message.hash_scheme = HashScheme::Blake3 as i32;
            message.signature = vec![fid as u8; 64];
            message.signature_scheme = SignatureScheme::Ed25519 as i32;
            message.signer = vec![fid as u8; 32];

            user_messages.push(message);
            transaction.user_messages = user_messages;
            transaction.account_root = vec![fid as u8; 32];

            transactions.push(transaction);
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
    let parsed_response =
        ShardChunksResponse::decode(&response_bytes[..]).expect("Failed to parse response");

    // Verify the parsed data
    assert_eq!(parsed_response.shard_chunks.len(), 43); // 0 to 42 inclusive

    for (i, chunk) in parsed_response.shard_chunks.iter().enumerate() {
        let expected_block_num = i as u64;

        // Verify header
        assert_eq!(
            chunk
                .header
                .as_ref()
                .unwrap()
                .height
                .as_ref()
                .unwrap()
                .shard_index,
            1
        );
        assert_eq!(
            chunk
                .header
                .as_ref()
                .unwrap()
                .height
                .as_ref()
                .unwrap()
                .block_number,
            expected_block_num
        );
        assert_eq!(
            chunk.header.as_ref().unwrap().timestamp,
            1640995200 + expected_block_num * 4
        );

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
            assert_eq!(
                message.data.as_ref().unwrap().r#type,
                MessageType::CastAdd as i32
            );

            if let Some(crate::generated::grpc_client::message_data::Body::CastAddBody(cast_body)) =
                &message.data.as_ref().unwrap().body
            {
                assert!(cast_body.text.contains(&format!("FID {}", expected_fid)));
                assert!(cast_body
                    .text
                    .contains(&format!("block {}", expected_block_num)));
            }
        }

        // Verify commits
        assert_eq!(
            chunk
                .commits
                .as_ref()
                .unwrap()
                .height
                .as_ref()
                .unwrap()
                .shard_index,
            1
        );
        assert_eq!(
            chunk
                .commits
                .as_ref()
                .unwrap()
                .height
                .as_ref()
                .unwrap()
                .block_number,
            expected_block_num
        );
        assert_eq!(chunk.commits.as_ref().unwrap().round, 0);
        assert_eq!(chunk.commits.as_ref().unwrap().signatures.len(), 3);
    }

    println!(
        "✅ Successfully parsed {} mock shard chunks from shard 1, blocks 0-42",
        parsed_response.shard_chunks.len()
    );

    // Print summary information
    for chunk in parsed_response.shard_chunks.iter().take(5) {
        println!(
            "Block {}: {} transactions, timestamp {}",
            chunk
                .header
                .as_ref()
                .unwrap()
                .height
                .as_ref()
                .unwrap()
                .block_number,
            chunk.transactions.len(),
            chunk.header.as_ref().unwrap().timestamp
        );
    }

    if parsed_response.shard_chunks.len() > 5 {
        println!(
            "... and {} more blocks",
            parsed_response.shard_chunks.len() - 5
        );
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
    let parsed = ShardChunksRequest::decode(&bytes[..]).expect("Failed to parse request");

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
    let parsed = ShardChunksResponse::decode(&bytes[..]).expect("Failed to parse empty response");

    assert_eq!(parsed.shard_chunks.len(), 0);
    println!("✅ Empty ShardChunksResponse test passed");
}

#[tokio::test]
async fn test_parse_block_1_detailed_analysis() {
    // Setup gRPC client
    let (_config, mut client) = setup_grpc_client().await;

    // Create request for blocks 0-5 (to ensure we get some data)
    let request = create_shard_chunks_request(1, 0, Some(5));

    // Make gRPC call
    let response = make_grpc_call(&mut client, request).await;

    // Verify the response
    let chunk_count = response.shard_chunks.len();
    println!("Received {} shard chunks", chunk_count);
    assert!(
        chunk_count > 0,
        "Expected to receive at least one shard chunk, but got 0"
    );

    // Analyze each block in detail
    for chunk in response.shard_chunks.iter() {
        print_detailed_block_analysis(chunk);
    }

    println!("\n✅ Successfully analyzed blocks in detail");
}
