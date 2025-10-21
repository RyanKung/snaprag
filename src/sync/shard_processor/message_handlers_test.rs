/// Unit tests for handlers module
/// These tests verify each message type handler function independently
/// 
/// Test Requirements:
/// - All assertions must use assert_eq!, assert!, assert_ne!
/// - NO println! for pass/fail determination
/// - Tests must panic on unexpected behavior
/// - Use .expect() with descriptive messages for Result unwrapping

#[cfg(test)]
mod tests {
    use super::super::handlers::*;
    use super::super::types::BatchedData;
    use crate::models::ShardBlockInfo;
    use serde_json::json;

    fn test_shard_info() -> ShardBlockInfo {
        ShardBlockInfo {
            shard_id: 1,
            block_height: 1000,
            transaction_fid: 99,
            timestamp: 1698765432,
        }
    }

    #[test]
    fn test_collect_message_data_cast_add() {
        // Test that CastAdd message is correctly parsed and collected
        let message = create_test_message(1, json!({
            "cast_add_body": {
                "text": "Hello Farcaster!",
                "embeds": [],
                "mentions": [],
                "parent_url": null
            }
        }));

        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        // This should collect a cast
        tokio_test::block_on(async {
            collect_message_data(&message, &shard_info, 0, &mut batched)
                .await
                .expect("Failed to collect cast");
        });

        // Strict assertions
        assert_eq!(batched.casts.len(), 1, "Should collect exactly 1 cast");
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
        assert!(batched.fids_to_ensure.contains(&99), "Should ensure FID 99");
        
        // Verify cast data
        let (fid, text, timestamp, hash, _, _, _, _, _) = &batched.casts[0];
        assert_eq!(*fid, 99, "Cast FID must be 99");
        assert!(text.is_some(), "Cast text must not be None");
        assert_eq!(text.as_ref().unwrap(), "Hello Farcaster!", "Cast text must match");
        assert_eq!(*timestamp, 1698765432, "Timestamp must match");
        assert_eq!(hash.len(), 4, "Message hash must be 4 bytes");
    }

    #[test]
    fn test_collect_message_data_reaction_add_cast() {
        // Test ReactionAdd to a cast
        let message = create_test_message(3, json!({
            "reaction_body": {
                "type": 1, // like
                "target_cast_id": {
                    "fid": 100,
                    "hash": "0123456789abcdef"
                }
            }
        }));

        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            collect_message_data(&message, &shard_info, 0, &mut batched)
                .await
                .expect("Failed to collect reaction");
        });

        // Strict assertions
        assert_eq!(batched.reactions.len(), 1, "Should collect exactly 1 reaction");
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
        assert!(batched.fids_to_ensure.contains(&99), "Should ensure FID 99");
        
        // Verify reaction data
        let (fid, target_hash, target_fid, reaction_type, timestamp, hash, _) = &batched.reactions[0];
        assert_eq!(*fid, 99, "Reaction FID must be 99");
        assert_eq!(*reaction_type, 1, "Reaction type must be 1 (like)");
        assert_eq!(*target_fid, Some(100), "Target FID must be 100");
        assert_eq!(target_hash.len(), 8, "Target hash must be decoded (8 bytes from '0123456789abcdef')");
        assert_eq!(*timestamp, 1698765432, "Timestamp must match");
        assert_eq!(hash.len(), 4, "Message hash must be 4 bytes");
    }

    #[test]
    fn test_collect_message_data_reaction_add_url() {
        // Test ReactionAdd to a URL
        let message = create_test_message(3, json!({
            "reaction_body": {
                "type": 1,
                "target_url": "https://opensea.io/collection/test"
            }
        }));

        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            collect_message_data(&message, &shard_info, 0, &mut batched)
                .await
                .expect("Failed to collect URL reaction");
        });

        // Strict assertions
        assert_eq!(batched.reactions.len(), 1, "Should collect exactly 1 URL reaction");
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
        
        // Verify URL reaction data
        let (fid, target_hash, target_fid, reaction_type, timestamp, hash, _) = &batched.reactions[0];
        assert_eq!(*fid, 99, "Reaction FID must be 99");
        assert_eq!(*reaction_type, 1, "Reaction type must be 1 (like)");
        assert_eq!(*target_fid, None, "URL reactions must have None target_fid");
        assert!(target_hash.len() > 0, "Target hash must not be empty");
        assert!(target_hash.starts_with(b"url_"), "URL hash must start with 'url_' prefix");
        assert_eq!(*timestamp, 1698765432, "Timestamp must match");
        assert_eq!(hash.len(), 4, "Message hash must be 4 bytes");
    }

    #[test]
    fn test_collect_message_data_link_add() {
        // Test LinkAdd message
        let message = create_test_message(5, json!({
            "link_body": {
                "type": "follow",
                "target_fid": 100
            }
        }));

        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            collect_message_data(&message, &shard_info, 0, &mut batched)
                .await
                .expect("Failed to collect link");
        });

        // Strict assertions
        assert_eq!(batched.links.len(), 1, "Should collect exactly 1 link");
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
        
        // Verify link data
        let (fid, target_fid, link_type, timestamp, hash, _) = &batched.links[0];
        assert_eq!(*fid, 99, "Link FID must be 99");
        assert_eq!(*target_fid, 100, "Target FID must be 100");
        assert_eq!(link_type, "follow", "Link type must be 'follow'");
        assert_eq!(*timestamp, 1698765432, "Timestamp must match");
        assert_eq!(hash.len(), 4, "Message hash must be 4 bytes");
    }

    #[test]
    fn test_collect_message_data_link_remove() {
        // Test LinkRemove message
        let message = create_test_message(6, json!({
            "link_body": {
                "type": "follow",
                "target_fid": 100
            }
        }));

        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            collect_message_data(&message, &shard_info, 0, &mut batched)
                .await
                .expect("Failed to collect link remove");
        });

        // Strict assertions
        assert_eq!(batched.link_removes.len(), 1, "Should collect exactly 1 link remove");
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
        assert_eq!(batched.links.len(), 0, "Should not collect link add");
        
        // Verify link remove data
        let (fid, target_fid, removed_at, hash) = &batched.link_removes[0];
        assert_eq!(*fid, 99, "Link remove FID must be 99");
        assert_eq!(*target_fid, 100, "Target FID must be 100");
        assert_eq!(*removed_at, 1698765432, "Removed timestamp must match");
        assert_eq!(hash.len(), 4, "Message hash must be 4 bytes");
    }

    #[test]
    fn test_collect_message_data_verification_add_eth() {
        // Test VerificationAdd for ETH address
        let message = create_test_message(7, json!({
            "verification_add_eth_address_body": {
                "address": "1234567890abcdef1234567890abcdef12345678",
                "claim_signature": "abcd",
                "block_hash": "ef01",
                "verification_type": 0,
                "chain_id": 1
            }
        }));

        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            collect_message_data(&message, &shard_info, 0, &mut batched)
                .await
                .expect("Failed to collect verification");
        });

        // Strict assertions
        assert_eq!(batched.verifications.len(), 1, "Should collect exactly 1 verification");
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
        
        // Verify ETH verification data
        let (fid, address, claim_sig, block_hash, verification_type, chain_id, timestamp, hash, _) = &batched.verifications[0];
        assert_eq!(*fid, 99, "Verification FID must be 99");
        assert_eq!(address.len(), 20, "ETH address must be exactly 20 bytes");
        assert!(claim_sig.is_some(), "Claim signature must be present");
        assert!(block_hash.is_some(), "Block hash must be present");
        assert_eq!(*verification_type, Some(0), "Verification type must be 0 (EOA)");
        assert_eq!(*chain_id, Some(1), "Chain ID must be 1 (Ethereum mainnet)");
        assert_eq!(*timestamp, 1698765432, "Timestamp must match");
        assert_eq!(hash.len(), 4, "Message hash must be 4 bytes");
    }

    #[test]
    fn test_collect_message_data_verification_add_solana() {
        // Test VerificationAdd for Solana address
        let message = create_test_message(7, json!({
            "verification_add_solana_address_body": {
                "address": "SolanaAddressBase58",
                "claim_signature": "abcd",
                "block_hash": "ef01"
            }
        }));

        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            collect_message_data(&message, &shard_info, 0, &mut batched)
                .await
                .expect("Failed to collect Solana verification");
        });

        // Strict assertions
        assert_eq!(batched.verifications.len(), 1, "Should collect exactly 1 Solana verification");
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
        
        // Verify Solana verification data
        let (fid, address, claim_sig, block_hash, verification_type, chain_id, timestamp, hash, _) = &batched.verifications[0];
        assert_eq!(*fid, 99, "Verification FID must be 99");
        assert!(address.len() > 0, "Solana address must not be empty");
        assert!(claim_sig.is_some(), "Claim signature must be present");
        assert!(block_hash.is_some(), "Block hash must be present");
        assert_eq!(*verification_type, Some(2), "Verification type must be 2 (Solana)");
        assert_eq!(*chain_id, Some(900), "Chain ID must be 900 (Solana standard)");
        assert_eq!(*timestamp, 1698765432, "Timestamp must match");
        assert_eq!(hash.len(), 4, "Message hash must be 4 bytes");
    }

    #[test]
    fn test_collect_message_data_user_data_add_all_types() {
        // Test all 13 UserDataAdd types
        let test_cases = vec![
            (1, "pfp_url", "https://example.com/avatar.png"),
            (2, "display_name", "Test User"),
            (3, "bio", "Test bio"),
            (5, "website_url", "https://example.com"),
            (6, "username", "testuser"),
            (7, "location", "San Francisco"),
            (8, "twitter_username", "testuser"),
            (9, "github_username", "testuser"),
            (10, "banner_url", "https://example.com/banner.png"),
            (11, "primary_address_ethereum", "0x1234..."),
            (12, "primary_address_solana", "Sol1..."),
            (13, "profile_token", "eip155:1/erc721:0x..."),
        ];

        for (data_type, expected_field, value) in test_cases {
            let message = create_test_message(11, json!({
                "user_data_body": {
                    "type": data_type,
                    "value": value
                }
            }));

            let mut batched = BatchedData::new();
            let shard_info = test_shard_info();
            
            tokio_test::block_on(async {
                collect_message_data(&message, &shard_info, 0, &mut batched)
                    .await
                    .expect(&format!("Failed to collect UserDataAdd type {}", data_type));
            });

            assert_eq!(batched.profile_updates.len(), 1, 
                "Should collect 1 profile update for type {}", data_type);
            let (fid, field_name, field_value, _, _) = &batched.profile_updates[0];
            assert_eq!(*fid, 99, "FID should match");
            assert_eq!(field_name, expected_field, 
                "Field name should be {} for type {}", expected_field, data_type);
            assert_eq!(field_value.as_ref().unwrap(), value,
                "Value should match for type {}", data_type);
        }
    }

    #[test]
    fn test_collect_message_data_username_proof() {
        // Test UsernameProof message
        let message = create_test_message(12, json!({
            "username_proof_body": {
                "name": "testuser",
                "owner": "1234567890abcdef1234567890abcdef12345678",
                "signature": "abcdef1234567890",
                "type": 1 // FNAME
            }
        }));

        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            collect_message_data(&message, &shard_info, 0, &mut batched)
                .await
                .expect("Failed to collect username proof");
        });

        // Strict assertions
        assert_eq!(batched.username_proofs.len(), 1, "Should collect exactly 1 username proof");
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
        
        // Verify username proof data
        let (fid, username, owner, signature, username_type, timestamp, hash, _) = &batched.username_proofs[0];
        assert_eq!(*fid, 99, "Username proof FID must be 99");
        assert_eq!(username, "testuser", "Username must be 'testuser'");
        assert_eq!(owner.len(), 20, "Owner must be exactly 20 bytes (ETH address)");
        assert!(signature.len() > 0, "Signature must not be empty");
        assert_eq!(*username_type, 1, "Username type must be 1 (FNAME)");
        assert_eq!(*timestamp, 1698765432, "Timestamp must match");
        assert_eq!(hash.len(), 4, "Message hash must be 4 bytes");
    }

    #[test]
    fn test_collect_message_data_frame_action() {
        // Test FrameAction message
        let message = create_test_message(13, json!({
            "frame_action_body": {
                "url": "https://example.com/frame",
                "button_index": 1,
                "cast_id": {
                    "fid": 100,
                    "hash": "abcdef1234567890"
                },
                "input_text": "test input",
                "state": "1234",
                "transaction_id": "5678"
            }
        }));

        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            collect_message_data(&message, &shard_info, 0, &mut batched)
                .await
                .expect("Failed to collect frame action");
        });

        // Strict assertions
        assert_eq!(batched.frame_actions.len(), 1, "Should collect exactly 1 frame action");
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
        
        // Verify frame action data
        let (fid, url, button_index, cast_hash, cast_fid, input_text, state, transaction_id, timestamp, hash, _) = &batched.frame_actions[0];
        assert_eq!(*fid, 99, "Frame action FID must be 99");
        assert_eq!(url, "https://example.com/frame", "URL must match exactly");
        assert_eq!(*button_index, Some(1), "Button index must be 1");
        assert_eq!(*cast_fid, Some(100), "Cast FID must be 100");
        assert!(cast_hash.is_some(), "Cast hash must be present");
        assert_eq!(cast_hash.as_ref().unwrap().len(), 8, "Cast hash must be 8 bytes (decoded from hex)");
        assert!(input_text.is_some(), "Input text must be present");
        assert_eq!(input_text.as_ref().unwrap(), "test input", "Input text must match exactly");
        assert!(state.is_some(), "State must be present");
        assert!(transaction_id.is_some(), "Transaction ID must be present");
        assert_eq!(*timestamp, 1698765432, "Timestamp must match");
        assert_eq!(hash.len(), 4, "Message hash must be 4 bytes");
    }

    #[test]
    fn test_remove_events_collection() {
        // Test ReactionRemove
        let message = create_test_message(4, json!({
            "reaction_body": {
                "type": 1,
                "target_cast_id": {
                    "fid": 100,
                    "hash": "0123456789abcdef"
                }
            }
        }));

        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            collect_message_data(&message, &shard_info, 0, &mut batched)
                .await
                .expect("Failed to collect reaction remove");
        });

        // Strict assertions
        assert_eq!(batched.reaction_removes.len(), 1, "Should collect exactly 1 reaction remove");
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
        assert_eq!(batched.reactions.len(), 0, "Should not collect reaction add");
        
        // Verify reaction remove data
        let (fid, target_hash, removed_at, hash) = &batched.reaction_removes[0];
        assert_eq!(*fid, 99, "Reaction remove FID must be 99");
        assert_eq!(target_hash.len(), 8, "Target hash must be 8 bytes (decoded)");
        assert_eq!(*removed_at, 1698765432, "Removed timestamp must match");
        assert_eq!(hash.len(), 4, "Message hash must be 4 bytes");
    }

    #[test]
    fn test_fid_ensure_for_all_types() {
        // Verify that all message types ensure FID exists
        // This is CRITICAL for data integrity - every message must ensure its FID exists
        let message_types = vec![1, 3, 5, 7, 11, 12, 13]; // All Add types
        
        for msg_type in message_types {
            let message = create_test_message(msg_type, json!({}));
            let mut batched = BatchedData::new();
            let shard_info = test_shard_info();
            
            // Ensure FID collection doesn't depend on successful body parsing
            tokio_test::block_on(async {
                collect_message_data(&message, &shard_info, 0, &mut batched)
                    .await
                    .ok(); // Might fail parsing, but should still ensure FID
            });

            // CRITICAL assertion - must pass for ALL types
            assert!(
                batched.fids_to_ensure.contains(&99),
                "CRITICAL: Message type {} MUST ensure FID 99 exists, but didn't. This would cause foreign key violations!",
                msg_type
            );
            assert_eq!(
                batched.fids_to_ensure.len(), 1,
                "Message type {} should ensure exactly 1 FID", msg_type
            );
        }
    }

    #[test]
    fn test_unknown_message_type() {
        // Test that unknown message types don't cause panic or error
        // This is important for forward compatibility
        let message = create_test_message(99, json!({}));
        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            let result = collect_message_data(&message, &shard_info, 0, &mut batched).await;
            assert!(result.is_ok(), "Unknown message type (99) must not cause error - forward compatibility requirement");
        });

        // CRITICAL: Unknown types must still ensure FID for data integrity
        assert!(
            batched.fids_to_ensure.contains(&99),
            "CRITICAL: Unknown message type must still ensure FID exists"
        );
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
        
        // Verify no data collected for unknown type
        assert_eq!(batched.casts.len(), 0, "Unknown type should not collect casts");
        assert_eq!(batched.links.len(), 0, "Unknown type should not collect links");
        assert_eq!(batched.reactions.len(), 0, "Unknown type should not collect reactions");
    }

    #[test]
    fn test_empty_body_handling() {
        // Test that messages with empty body don't panic
        use crate::sync::client::proto::{Message as FarcasterMessage, MessageData};
        
        let message = FarcasterMessage {
            data: Some(MessageData {
                r#type: 3, // ReactionAdd
                fid: 99,
                timestamp: 1698765432,
                body: None, // Empty body
            }),
            hash: vec![1, 2, 3, 4],
        };

        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            let result = collect_message_data(&message, &shard_info, 0, &mut batched).await;
            assert!(result.is_ok(), "Empty body must not cause error - robustness requirement");
        });

        // Strict assertions for graceful degradation
        assert_eq!(batched.reactions.len(), 0, "Empty body must not collect reaction");
        assert_eq!(batched.links.len(), 0, "Empty body must not collect link");
        assert_eq!(batched.verifications.len(), 0, "Empty body must not collect verification");
        
        // CRITICAL: FID must still be ensured even with empty body
        assert!(
            batched.fids_to_ensure.contains(&99),
            "CRITICAL: Empty body must still ensure FID exists"
        );
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure exactly 1 FID");
    }

    // Helper function to create test messages
    fn create_test_message(message_type: i32, body: serde_json::Value) -> crate::sync::client::proto::Message {
        use crate::sync::client::proto::{Message as FarcasterMessage, MessageData};
        
        FarcasterMessage {
            data: Some(MessageData {
                r#type: message_type,
                fid: 99,
                timestamp: 1698765432,
                body: Some(body),
            }),
            hash: vec![1, 2, 3, 4],
        }
    }
}

