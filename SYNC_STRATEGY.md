# SnapRAG Data Synchronization Strategy

## Overview

SnapRAG implements a comprehensive data synchronization system that efficiently syncs all Farcaster data from snapchain nodes using the optimal gRPC APIs. The system is designed to handle both historical and real-time data synchronization.

## Architecture

### 1. API Selection

We use **ReplicationService** as the primary API for historical synchronization because it provides:

- **Shard-based sync**: Organized by shard IDs (0 = block shard, 1+ = user shards)
- **Trie iteration**: Complete coverage of all data using trie virtual shards (0-255)
- **Pagination support**: Handles large datasets efficiently
- **Snapshot-based**: Provides consistent point-in-time snapshots

### 2. Synchronization Flow

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Snapchain     │    │   SnapRAG        │    │   PostgreSQL    │
│   Node          │───▶│   Sync Service   │───▶│   Database      │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

#### Historical Sync Process:
1. **Get Shard Snapshots**: Query available snapshots for each shard
2. **Trie Iteration**: For each snapshot, iterate through trie virtual shards (0-255)
3. **Message Processing**: Process all messages in each trie entry
4. **Data Storage**: Store processed data in appropriate tables

#### Real-time Sync Process:
1. **Subscribe to Events**: Use HubService Subscribe API for real-time updates
2. **Process New Messages**: Handle new messages as they arrive
3. **Update Database**: Store new data incrementally

## Data Processing

### Message Types Handled

| Message Type | Processing | Storage |
|-------------|------------|---------|
| `CAST_ADD` | Parse cast content, generate embeddings | `casts` table + embeddings |
| `CAST_REMOVE` | Mark cast as removed | Update `casts` table |
| `REACTION_ADD` | Track engagement metrics | `reactions` table |
| `REACTION_REMOVE` | Remove reaction | Update `reactions` table |
| `USER_DATA_ADD` | Update profile, create snapshot | `user_profiles` + `user_profile_snapshots` |
| `USERNAME_PROOF` | Store username proof | `username_proofs` table |
| `LINK_ADD` | Track user connections | `links` table |
| `VERIFICATION_ADD` | Store address verification | `verifications` table |
| `FRAME_ACTION` | Track frame interactions | `frame_actions` table |

### Database Tables

#### Core Tables:
- `user_profiles`: Current state of user profiles
- `user_profile_snapshots`: Historical profile states
- `user_data_changes`: Track all profile changes
- `user_activity_timeline`: User activity events
- `username_proofs`: Username ownership proofs

#### Message Tables:
- `processed_messages`: Track processed messages to avoid duplicates
- `sync_progress`: Track sync progress per shard

## Configuration

### Sync Configuration (`config.toml`)

```toml
[sync]
snapchain_endpoint = "http://localhost:3383"
enable_realtime_sync = true
enable_historical_sync = true
historical_sync_from_event_id = 0
batch_size = 100
sync_interval_ms = 1000
shard_ids = [0, 1, 2]  # Block shard + user shards
```

### Shard Strategy

- **Shard 0**: Block shard (consensus data)
- **Shard 1+**: User shards (user messages and data)
- **Trie Virtual Shards**: 0-255 for complete data coverage

## Usage

### Run Comprehensive Sync

```bash
cargo run --bin sync_comprehensive
```

This will:
1. Load configuration from `config.toml`
2. Initialize database and run migrations
3. Start historical sync using ReplicationService
4. Switch to real-time sync using Subscribe API
5. Process all message types and store in appropriate tables

### Sync Progress Tracking

The system tracks:
- Last processed height per shard
- Total messages processed
- Total users discovered
- Sync errors and retries
- Processing performance metrics

## Error Handling

- **Retry Logic**: Automatic retry for transient failures
- **Progress Persistence**: Resume from last successful position
- **Error Logging**: Comprehensive error tracking
- **Graceful Degradation**: Continue processing other shards on individual failures

## Performance Considerations

- **Batch Processing**: Process messages in configurable batches
- **Parallel Shards**: Process multiple shards concurrently
- **Database Optimization**: Use prepared statements and transactions
- **Memory Management**: Stream large datasets to avoid memory issues

## Monitoring

The sync service provides:
- Real-time progress updates
- Performance metrics (messages/second, users/second)
- Error rates and types
- Database growth statistics

## Future Enhancements

1. **Incremental Sync**: Only sync new data since last run
2. **Selective Sync**: Sync only specific message types or users
3. **Embedding Generation**: Generate embeddings during sync
4. **Analytics**: Real-time analytics on synced data
5. **Multi-node Support**: Sync from multiple snapchain nodes
