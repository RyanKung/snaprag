/// Unit tests for message_handlers module
/// These tests verify each message type handler function independently

#[cfg(test)]
mod tests {
    use super::super::message_handlers::*;
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

        assert_eq!(batched.casts.len(), 1, "Should collect 1 cast");
        assert_eq!(batched.fids_to_ensure.len(), 1, "Should ensure FID exists");
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

        assert_eq!(batched.reactions.len(), 1, "Should collect 1 reaction");
        let (fid, target_hash, target_fid, reaction_type, _, _, _) = &batched.reactions[0];
        assert_eq!(*fid, 99, "FID should match");
        assert_eq!(*reaction_type, 1, "Should be like (type 1)");
        assert_eq!(*target_fid, Some(100), "Target FID should be 100");
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

        assert_eq!(batched.reactions.len(), 1, "Should collect 1 URL reaction");
        let (fid, target_hash, target_fid, _, _, _, _) = &batched.reactions[0];
        assert_eq!(*fid, 99, "FID should match");
        assert_eq!(*target_fid, None, "URL reactions have no target_fid");
        assert!(target_hash.len() > 0, "Should have URL hash");
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

        assert_eq!(batched.links.len(), 1, "Should collect 1 link");
        let (fid, target_fid, link_type, _, _, _) = &batched.links[0];
        assert_eq!(*fid, 99, "FID should match");
        assert_eq!(*target_fid, 100, "Target FID should be 100");
        assert_eq!(link_type, "follow", "Link type should be follow");
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

        assert_eq!(batched.link_removes.len(), 1, "Should collect 1 link remove");
        let (fid, target_fid, _, _) = &batched.link_removes[0];
        assert_eq!(*fid, 99, "FID should match");
        assert_eq!(*target_fid, 100, "Target FID should be 100");
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

        assert_eq!(batched.verifications.len(), 1, "Should collect 1 verification");
        let (fid, address, _, _, verification_type, chain_id, _, _, _) = &batched.verifications[0];
        assert_eq!(*fid, 99, "FID should match");
        assert_eq!(address.len(), 20, "ETH address should be 20 bytes");
        assert_eq!(*verification_type, Some(0), "Should be EOA");
        assert_eq!(*chain_id, Some(1), "Should be Ethereum mainnet");
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

        assert_eq!(batched.verifications.len(), 1, "Should collect 1 Solana verification");
        let (fid, address, _, _, verification_type, chain_id, _, _, _) = &batched.verifications[0];
        assert_eq!(*fid, 99, "FID should match");
        assert_eq!(*verification_type, Some(2), "Should be Solana type");
        assert_eq!(*chain_id, Some(900), "Should be Solana chain_id");
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

        assert_eq!(batched.username_proofs.len(), 1, "Should collect 1 username proof");
        let (fid, username, owner, signature, username_type, _, _, _) = &batched.username_proofs[0];
        assert_eq!(*fid, 99, "FID should match");
        assert_eq!(username, "testuser", "Username should match");
        assert_eq!(owner.len(), 20, "Owner should be 20 bytes (ETH address)");
        assert_eq!(*username_type, 1, "Should be FNAME type");
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

        assert_eq!(batched.frame_actions.len(), 1, "Should collect 1 frame action");
        let (fid, url, button_index, cast_hash, cast_fid, input_text, _, _, _, _, _) = &batched.frame_actions[0];
        assert_eq!(*fid, 99, "FID should match");
        assert_eq!(url, "https://example.com/frame", "URL should match");
        assert_eq!(*button_index, Some(1), "Button index should be 1");
        assert_eq!(*cast_fid, Some(100), "Cast FID should be 100");
        assert!(cast_hash.is_some(), "Should have cast hash");
        assert_eq!(input_text.as_ref().unwrap(), "test input", "Input text should match");
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

        assert_eq!(batched.reaction_removes.len(), 1, "Should collect 1 reaction remove");
    }

    #[test]
    fn test_fid_ensure_for_all_types() {
        // Verify that all message types ensure FID exists
        let message_types = vec![1, 3, 5, 7, 11, 12, 13]; // All Add types
        
        for msg_type in message_types {
            let message = create_test_message(msg_type, json!({}));
            let mut batched = BatchedData::new();
            let shard_info = test_shard_info();
            
            tokio_test::block_on(async {
                collect_message_data(&message, &shard_info, 0, &mut batched)
                    .await
                    .ok(); // Might fail parsing, but should still ensure FID
            });

            assert!(batched.fids_to_ensure.contains(&99),
                "Type {} should ensure FID 99 exists", msg_type);
        }
    }

    #[test]
    fn test_unknown_message_type() {
        // Test that unknown message types don't cause panic
        let message = create_test_message(99, json!({}));
        let mut batched = BatchedData::new();
        let shard_info = test_shard_info();
        
        tokio_test::block_on(async {
            let result = collect_message_data(&message, &shard_info, 0, &mut batched).await;
            assert!(result.is_ok(), "Unknown message type should not error");
        });

        // Should still ensure FID
        assert!(batched.fids_to_ensure.contains(&99), "Should ensure FID even for unknown type");
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
            assert!(result.is_ok(), "Empty body should not error");
        });

        // Should not collect anything, but FID should be ensured
        assert_eq!(batched.reactions.len(), 0, "Should not collect reaction with empty body");
        assert!(batched.fids_to_ensure.contains(&99), "Should still ensure FID");
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

