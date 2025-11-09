<div align="center">

<img src="./logo.png" alt="SnapRAG Logo" width="200" height="200">

# SnapRAG

**PostgreSQL-based RAG Foundation Framework with Database Query Capabilities**

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![PostgreSQL](https://img.shields.io/badge/postgresql-15+-blue.svg)](https://www.postgresql.org/)
[![License](https://img.shields.io/badge/license-GPTv3-blue.svg)](LICENSE)

*A comprehensive PostgreSQL-based RAG foundation framework that provides data synchronization, vector search, and advanced database query capabilities for Farcaster protocol data*

<br/>

![Demo](./quick_demo.gif)

</div>

## 💭 Philosophy: Dancing in Chains

In recent years, as Crypto and LLM technology evolved in parallel, a subtle yet profound realization began to surface: philosophically, LLM might stand at the exact opposite end of Crypto. These two domains represent radically different human responses to entropy — one seeks to impose rational order, the other chooses to dance faster in the chaos.

**Crypto** — especially in its cryptopunk ethos — is an art of entropy control, using cryptography as a blade to carve order into a high-entropy world. Meanwhile, the **LLM community** behaves more like a tribe of intuitive dancers around a black box, embracing vibes, emergence, and generative chaos, without insisting on transparency, verification, or absolute control.

This core divergence creates two entirely different cultures:
- **Crypto** worships openness, transparency, and permissionless access.
- **LLM culture** seems content with black-box outputs, hardware aristocracy, and closed creative loops.

**To fuse the two is to dance in chains.**

SnapRAG is precisely such a Don Quixote–style endeavor.

## 🎯 What is SnapRAG?

It is **fully open-source**, written in Rust, with extreme performance and absolutely no hidden backend logic. No vendor lock-in, no gated APIs — anyone can run it locally and hold full sovereignty over their AI stack. Built on top of the decentralized social protocol **Farcaster**, SnapRAG leverages a full Snapchain node to synchronize every piece of raw network data, forming the foundation of a truly omniscient Farcaster AI.

### 📊 Current Farcaster Network Scale

As of today, the Snapchain network has produced:

- **17,894,257 blocks**
- **1,393,099 registered users**  
- **224,749,776 casts**

This is a dataset of **massive scale**.

But unlike Twitter, Farcaster grants radical **data freedom** — run a node, and you instantly gain access to everyone's data. No rate limits, no API gatekeeping, no silent throttling. On top of this freedom, SnapRAG offers a powerful operational toolkit:

- ✅ Import all Farcaster data into a local database
- ✅ Vectorize all Casts into embeddings
- ✅ Perform high-performance semantic search — locally

For developers, this is the moment of **reclaiming AI computation sovereignty**:

> **Your node. Your database. Your embeddings. Your AI** — not rented from someone else's API ceiling.

### 🚀 Full Capability Surface

SnapRAG is not just a tool — it is a **full capability surface**.

It is a **CLI**, an **API service**, and an **MCP-compatible interface**, all with built-in support for our Farcaster-native protocol **x402**. This means SnapRAG can serve both individual hackers and teams seeking to build minimal-profitable open AI infrastructure on Farcaster.

We deeply respect the work done by **Neynar** — it's a fantastic product. But it remains closed. SnapRAG openly invites teams like Neynar to migrate onto this open infrastructure and run truly real-time, open, permissionless AI services at Farcaster scale.

**Because that's what blockchain ethos demands.**

Everything we do is so that the spirit of Crypto does not dissolve in the age of AI.

---

## 🎯 Technical Overview

SnapRAG is a PostgreSQL-based RAG foundation framework designed specifically for Farcaster protocol data. It provides a complete data synchronization layer, vector search capabilities, and advanced database query functionality, making it an ideal foundation for building RAG (Retrieval-Augmented Generation) applications on top of Farcaster data.

### Key Features
- 🏗️ **RAG Foundation**: PostgreSQL-based framework for building RAG applications
- 📚 **Library + CLI**: Use as Rust library OR standalone CLI tool
- 🔄 **Data Synchronization**: Complete historical + real-time Farcaster data sync
- 🔍 **Vector Search**: Built-in pgvector support for semantic similarity search
- 📊 **Advanced Queries**: Rich database query capabilities and analytics
- 🚀 **High Performance**: Rust-based with async PostgreSQL integration
- 🛡️ **Data Integrity**: Complete audit trail and change tracking
- 🎛️ **CLI Tools**: Full command-line interface for all operations

## 📋 Table of Contents

- [Quick Start](#-quick-start)
- [**Docker Deployment**](#-docker-deployment) 🐳 NEW
- [Features](#-features)
- [**Library Usage**](#-using-as-a-library) ⭐ NEW
- [CLI Commands](#️-available-cli-commands)
- [Database Schema](#️-database-schema)
- [Text Search Capabilities](#-text-search-capabilities)
- [Block Data Distribution](#block-data-distribution)
- [Installation](#-installation)
- [Usage](#-usage)
- [Architecture](#️-architecture)
- [Testing](#-testing)
- [Configuration](#-configuration)
- [Performance Tuning](#-performance-tuning)
- [Troubleshooting](#-troubleshooting)
- [Contributing](#-contributing)

## 🚀 Quick Start

> **Note:** SnapRAG can be built **without a database connection**! The build process uses SQLx offline mode by default, so you can compile the project before setting up your database. This makes it easy to get started quickly.

### As CLI Tool

```bash
# 1. Clone and setup
git clone <repository-url> && cd snaprag

# 2. Build the project (no database required!)
cargo build

# 3. Create configuration file
cp config.example.toml config.toml

# 4. Edit config.toml with your database connection details
# Update the database.url field with your actual database connection string

# 5. Check your configuration
make check-config  # Verify config.toml is valid

# 6. Ensure required extensions are enabled on your database
# Connect to your database and run:
# CREATE EXTENSION IF NOT EXISTS vector;
# CREATE EXTENSION IF NOT EXISTS pg_trgm;

# 7. Run database migrations and application
make migrate     # Run database migrations
make run         # Run the application
```

### As Rust Library

```toml
# Cargo.toml
[dependencies]
snaprag = { path = "../snaprag" }
tokio = { version = "1.0", features = ["full"] }
```

```rust
// src/main.rs
use snaprag::{SnapRag, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    
    // Search profiles
    let results = snaprag.semantic_search_profiles(
        "developers interested in crypto", 
        10, 
        Some(0.7)
    ).await?;
    
    println!("Found {} profiles", results.len());
    Ok(())
}
```

**See examples in [examples/](./examples/) directory!**

## 🐳 Docker Deployment

Deploy SnapRAG as a Docker container. Connects to your existing PostgreSQL and Redis.

```bash
# Quick start (all in one)
make -f Makefile.docker docker-quick-start

# Or step by step
make -f Makefile.docker setup-config   # Create config.toml
make -f Makefile.docker docker-build   # Build image
make -f Makefile.docker docker-run     # Run container
```

📚 **[Complete Docker Documentation →](./DOCKER_DEPLOYMENT.md)**

**Design Philosophy:**
- ✅ **Single-container deployment** - Simple and fast
- ✅ **Config always external** - Security best practice (no secrets in image)
- ✅ **Connects to external services** - Use your existing PostgreSQL/Redis
- ✅ **Production-ready** - Non-root user, health checks, logging

**Key Commands:**
```bash
make -f Makefile.docker help          # Show all commands
make -f Makefile.docker docker-build  # Build image
make -f Makefile.docker docker-run    # Run API server
make -f Makefile.docker docker-logs   # View logs
make -f Makefile.docker docker-stop   # Stop container
```

**Requirements:**
- PostgreSQL 15+ with pgvector (running separately)
- Redis 7+ (optional, for caching)
- config.toml configured with your database URL

## ✨ Features

### RAG Foundation Framework
- **PostgreSQL-based Architecture**: Built on PostgreSQL for robust data management
- **Vector Search Ready**: Built-in pgvector support for semantic similarity search
- **Query Interface**: Rich database query capabilities for complex analytics
- **Data Synchronization Layer**: Complete Farcaster data sync from snapchain
- **RAG Application Ready**: Designed as a foundation for building RAG applications

### Core Synchronization
- **Historical Data Sync**: Complete synchronization of past Farcaster data from snapchain
- **Real-time Monitoring**: Live monitoring of new blocks and user activities
- **Shard-based Processing**: Efficient processing of data across multiple shards
- **Lock File Management**: Prevents concurrent sync processes with PID tracking
- **Progress Tracking**: Real-time sync progress and status monitoring

### Data Management & Query Capabilities
- **Historical Profile Preservation**: Complete snapshot history of user profile changes
- **Efficient Current State Access**: Fast queries for current profile data
- **Vector Embeddings Support**: Built-in support for pgvector for semantic search
- **Advanced Database Queries**: Complex analytics and data exploration capabilities
- **Change Tracking**: Detailed audit trail of all profile modifications
- **Username Proofs**: Support for Farcaster-style username verification
- **Activity Timeline**: Comprehensive user activity tracking
- **No Data Cleanup**: All historical data is preserved indefinitely

### CLI Tools
- **Comprehensive CLI**: Full command-line interface for all operations
- **Sync Management**: Start, stop, and monitor synchronization processes
- **Data Querying**: List and search FIDs, profiles, casts, and relationships
- **Database Operations**: Migration, reset, and maintenance commands

## 🛠️ Available CLI Commands

### Main Commands
```bash
# Show help
cargo run -- --help

# List available data
cargo run list fid --limit 50
cargo run list profiles --limit 20
cargo run list casts --limit 100
cargo run list follows --limit 50

# Reset all data
cargo run reset --force

# Show configuration
cargo run config
```

### Synchronization Commands
```bash
# Sync all data (historical + real-time)
cargo run sync all

# Start historical sync with optional range
cargo run sync start
cargo run sync start --from 1000000 --to 2000000

# Start real-time sync
cargo run sync realtime

# Show sync status
cargo run sync status

# Stop running sync
cargo run sync stop
```

## 🗄️ Database Schema

The system uses the following main tables:

### Core Tables
- `user_profiles`: Current profile state (latest values only)
- `user_profile_snapshots`: Historical profile snapshots
- `user_data_changes`: Detailed change tracking
- `user_activity_timeline`: User activity history

### Farcaster-specific Tables
- `fids`: Farcaster ID registry
- `fname_transfers`: Username transfer history
- `signers`: User signer keys
- `signer_history`: Signer key changes
- `storage_rent_events`: Storage rent events
- `id_register_events`: ID registration events

### Sync Tracking Tables
- `sync_state`: Synchronization state and progress
- `shard_block_info`: Shard and block tracking for data origin

## 🔍 Text Search Capabilities

SnapRAG includes built-in support for PostgreSQL's `pg_trgm` extension, providing powerful trigram-based text search capabilities across all text fields.

### Supported Text Search Operations

#### 1. Similarity Search
Find text with high similarity using trigram matching:

```sql
-- Find casts with text similar to "crypto" (similarity threshold: 0.3)
SELECT fid, text, similarity(text, 'crypto') as sim
FROM casts 
WHERE text % 'crypto' 
ORDER BY sim DESC 
LIMIT 10;

-- Find usernames similar to "alice" 
SELECT fid, username, similarity(username, 'alice') as sim
FROM username_proofs 
WHERE username % 'alice' 
ORDER BY sim DESC;
```

#### 2. Pattern Matching
Use ILIKE with trigram optimization for pattern searches:

```sql
-- Find casts containing "bitcoin" (case-insensitive)
SELECT fid, text 
FROM casts 
WHERE text ILIKE '%bitcoin%' 
ORDER BY timestamp DESC 
LIMIT 20;

-- Find usernames starting with "crypto"
SELECT fid, username 
FROM username_proofs 
WHERE username ILIKE 'crypto%';
```

#### 3. Fuzzy Text Search
Find text with typos or variations:

```sql
-- Find casts mentioning "ethereum" with fuzzy matching
SELECT fid, text, similarity(text, 'ethereum') as sim
FROM casts 
WHERE text % 'ethereum' 
  AND similarity(text, 'ethereum') > 0.4
ORDER BY sim DESC;
```

### Optimized Indexes

The following GIN indexes are automatically created for optimal text search performance:

- `idx_casts_text_trgm`: Fast text search on cast content
- `idx_cast_embeddings_text_trgm`: Text search on embedding text
- `idx_cast_embedding_chunks_text_trgm`: Text search on embedding chunks
- `idx_user_profile_changes_value_trgm`: Text search on profile field values
- `idx_username_proofs_username_trgm`: Text search on usernames

### Performance Tips

1. **Use similarity thresholds**: Always specify minimum similarity thresholds for better performance
2. **Combine with other filters**: Use text search with timestamp or user filters
3. **Index utilization**: The trigram indexes are automatically used by PostgreSQL when appropriate

```sql
-- Example: Efficient combined query
SELECT fid, text, timestamp
FROM casts 
WHERE text % 'defi' 
  AND similarity(text, 'defi') > 0.3
  AND timestamp > extract(epoch from now() - interval '7 days') * 1000
ORDER BY timestamp DESC 
LIMIT 50;
```

## Block Data Distribution

Based on our analysis of the snapchain network, here's the distribution of user messages across different block ranges:

### Early Blocks (Genesis - ~625,000)
- **Blocks 0-10**: No user messages, only system messages
- **Blocks 0-1000**: No user messages found
- **Blocks 5000-6000**: No user messages found
- **Blocks 10000-10100**: No user messages found
- **Blocks 50000-50100**: No user messages found
- **Blocks 625000-625100**: No user messages found

### User Message Start Range (~625,000 - 1,250,000)
- **Blocks 1250000-1250100**: ✅ User messages found
- **Blocks 2500000-2500100**: ✅ User messages found

### High Activity Range (5,000,000+)
- **Blocks 5000000-5001000**: ✅ High user message activity
- **Profile Creation Messages**: Found Type 11 (UserDataAdd) messages
- **Current Network Height**: ~15,550,000 blocks

### Key Findings
1. **User messages start around block 625,000+**
2. **Early blocks contain only system messages** (ValidatorMessage types)
3. **Profile creation/modification messages** (Type 11) are common in higher blocks
4. **Block production rate**: ~1 second per block
5. **Timestamp format**: snapchain-specific timestamps (not Unix epoch)

### Recommended Test Ranges
- **No user messages**: 0-1000, 5000-6000, 10000-10100
- **First user messages**: 625000-625100
- **Active user activity**: 1250000-1250100, 2500000-2500100
- **High activity**: 5000000-5001000

## 🔧 Installation

### Prerequisites

- Rust 1.70+ 
- PostgreSQL 15+ with pgvector and pg_trgm extensions
- Remote database access

### Quick Start with Remote Database

```bash
# 1. Clone the repository
git clone <repository-url>
cd snaprag

# 2. Build the project (no database required!)
cargo build

# 3. Create and configure config.toml
cp config.example.toml config.toml
# Edit config.toml and update the database.url field

# 4. Ensure required extensions are enabled on your remote database
# Connect to your database and run:
# CREATE EXTENSION IF NOT EXISTS vector;
# CREATE EXTENSION IF NOT EXISTS pg_trgm;

# 5. Run database migrations
make migrate

# 6. Run the application
cargo run
```

### Remote Database Setup

For production or development with remote databases:

```bash
# 1. Ensure your remote PostgreSQL has required extensions
# Connect to your remote database and run:
psql -h your-db-host -U your-username -d your-database
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
\q

# 2. Create and configure config.toml
cp config.example.toml config.toml
# Edit config.toml and update the database.url field with your connection string

# 3. Run database migrations
make migrate

# 4. Build and run
cargo build
cargo run
```

### Local Development Setup

If you need to set up PostgreSQL locally for development:

```bash
# 1. Build the project first (no database required!)
cargo build

# 2. Install PostgreSQL and required extensions
# On Ubuntu/Debian:
sudo apt-get install postgresql-15 postgresql-15-pgvector postgresql-15-pgtrgm

# On macOS with Homebrew:
brew install postgresql@15 pgvector

# On CentOS/RHEL:
sudo yum install postgresql15-server postgresql15-contrib
# Then compile pgvector from source

# 3. Create database and user
sudo -u postgres psql
CREATE DATABASE snaprag;
CREATE USER snaprag WITH PASSWORD 'snaprag123';
GRANT ALL PRIVILEGES ON DATABASE snaprag TO snaprag;
\q

# 4. Connect to database and enable required extensions
psql -U snaprag -d snaprag -h localhost
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
\q

# 5. Create and configure config.toml
cp config.example.toml config.toml
# Edit config.toml and update database.url to: postgresql://snaprag:snaprag123@localhost/snaprag

# 6. Run migrations and start the application
make migrate
cargo run
```

### Using Makefile Commands

For easier development workflow:

```bash
# Development Commands
make check-config  # Check configuration file
make migrate       # Run database migrations  
make run           # Run the application
make run-example   # Run basic usage example

# Testing Commands
make test          # Run all tests

# Build Commands
make build         # Build the project
make build-release # Build in release mode
make clean         # Clean build artifacts

# Code Quality Commands
make check         # Run clippy and format checks
make fix           # Fix clippy and format issues
make docs          # Generate and open documentation
make bench         # Run benchmarks
```

### All Available Commands

| Command | Description |
|---------|-------------|
| `make help` | Show all available commands |
| `make check-config` | Check configuration file |
| `make migrate` | Run database migrations |
| `make run` | Run the application |
| `make run-example` | Run basic usage example |
| `make test` | Run all tests |
| `make test-strict` | Run tests with strict settings (warnings as errors) |
| `make test-quick` | Run quick tests (unit tests only) |
| `make test-integration` | Run integration tests only |
| `make build` | Build the project |
| `make build-release` | Build in release mode |
| `make clean` | Clean build artifacts |
| `make check` | Run clippy and format checks |
| `make fix` | Fix clippy and format issues |
| `make docs` | Generate documentation |
| `make bench` | Run benchmarks |

## 📚 Using as a Library

SnapRAG can be used as a Rust library in your projects:

### Basic Example

```rust
use snaprag::{SnapRag, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    
    // Query data
    let profiles = snaprag.search_profiles("developer").await?;
    let stats = snaprag.get_statistics().await?;
    
    println!("Found {} developers, {} total users", 
        profiles.len(), stats.total_fids);
    
    Ok(())
}
```

### Semantic Search

```rust
// Profile semantic search
let results = snaprag.semantic_search_profiles(
    "AI and blockchain developers",
    10,
    Some(0.7)  // similarity threshold
).await?;

// Cast semantic search with engagement metrics
let casts = snaprag.semantic_search_casts(
    "discussions about frames",
    15,
    Some(0.7)
).await?;

for cast in casts {
    println!("{} ({}% match, {} replies, {} reactions)",
        cast.text,
        (cast.similarity * 100.0) as i32,
        cast.reply_count,
        cast.reaction_count
    );
}
```

### RAG Queries

```rust
// Create RAG service
let rag = snaprag.create_rag_service().await?;

// Natural language query
let response = rag.query("Find the most active builders on Farcaster").await?;
println!("Answer: {}", response.answer);
println!("Sources: {} profiles", response.sources.len());
```

### Available API Methods

```rust
impl SnapRag {
    // Initialization
    pub async fn new(config: &AppConfig) -> Result<Self>;
    pub async fn init_database(&self) -> Result<()>;
    
    // Sync
    pub async fn start_sync(&mut self) -> Result<()>;
    pub async fn start_sync_with_range(&mut self, from: u64, to: u64) -> Result<()>;
    
    // Queries
    pub async fn get_profile(&self, fid: i64) -> Result<Option<UserProfile>>;
    pub async fn search_profiles(&self, query: &str) -> Result<Vec<UserProfile>>;
    pub async fn list_casts(&self, limit: Option<i64>) -> Result<Vec<Cast>>;
    pub async fn get_user_activity(...) -> Result<Vec<UserActivityTimeline>>;
    
    // Semantic Search
    pub async fn semantic_search_profiles(...) -> Result<Vec<SearchResult>>;
    pub async fn semantic_search_casts(...) -> Result<Vec<CastSearchResult>>;
    
    // Services
    pub async fn create_rag_service(&self) -> Result<RagService>;
    pub fn create_embedding_service(&self) -> Result<Arc<EmbeddingService>>;
    pub fn create_llm_service(&self) -> Result<Arc<LlmService>>;
    
    // Embeddings
    pub async fn backfill_profile_embeddings(&self, limit: Option<usize>) -> Result<...>;
    pub async fn backfill_cast_embeddings(&self, limit: Option<usize>) -> Result<...>;
}
```

### Runnable Examples

```bash
# Run example code
cargo run --example simple_query
cargo run --example semantic_search
cargo run --example rag_query
cargo run --example custom_pipeline
```

## 🤖 RAG Architecture

### Pipeline Overview

```
User Query
    ↓
【Retrieval】
  ├─ Semantic Search (vector similarity)
  ├─ Keyword Search (text matching)
  ├─ Hybrid Search (RRF fusion)
  └─ Auto Search (intelligent selection)
    ↓
【Ranking】
  ├─ Vector Similarity Scoring
  ├─ RRF (Reciprocal Rank Fusion)
  └─ Score Normalization
    ↓
【Context Assembly】
  ├─ Profile/Cast Formatting
  ├─ Author Information
  ├─ Engagement Metrics
  └─ Length Management (4096 tokens)
    ↓
【Generation】
  ├─ Prompt Template
  ├─ LLM Query (OpenAI/Ollama)
  └─ Streaming Response
    ↓
Answer + Sources
```

### Retrieval Methods

| Method | Use Case | Performance |
|--------|----------|-------------|
| **Semantic** | Conceptual queries ("find AI developers") | ~10ms |
| **Keyword** | Exact matches (names, specific terms) | ~5ms |
| **Hybrid** | Complex queries (combines both with RRF) | ~15ms |
| **Auto** | Unknown - system chooses best method | Adaptive |

### Performance Metrics

- Profile search: ~10ms (10K profiles)
- Cast search: ~50ms (100K casts)
- Embedding generation: ~200ms (OpenAI)
- Embedding backfill: ~50 casts/sec (5x parallel)
- Sync processing: 38% faster (batch optimization)

## 🚀 Usage

### Starting Synchronization

```bash
# Start historical sync (recommended for first run)
cargo run sync start

# Start with specific block range
cargo run sync start --from 1000000 --to 2000000

# Start real-time sync (after historical sync)
cargo run sync realtime

# Run both historical and real-time sync
cargo run sync all
```

### Monitoring Sync Progress

```bash
# Check sync status
cargo run sync status

# Stop running sync
cargo run sync stop

# Reset all data and start fresh
cargo run reset --force
```

### Querying Data

```bash
# List FIDs
cargo run list fid --limit 50

# List user profiles
cargo run list profiles --limit 20

# List casts
cargo run list casts --limit 100

# List follow relationships
cargo run list follows --limit 50
```

### Programmatic Usage

```rust
use snaprag::models::*;
use snaprag::database::Database;

// Create a user profile
let create_request = CreateUserProfileRequest {
    fid: 12345,
    username: Some("alice".to_string()),
    display_name: Some("Alice Smith".to_string()),
    bio: Some("Blockchain enthusiast".to_string()),
    message_hash: vec![1, 2, 3, 4, 5],
    timestamp: 1640995200,
};

let profile = db.create_user_profile(create_request).await?;

// Update a profile
let update_request = UpdateUserProfileRequest {
    fid: 12345,
    data_type: UserDataType::Bio,
    new_value: "Senior blockchain developer".to_string(),
    message_hash: vec![6, 7, 8, 9, 10],
    timestamp: 1640995800,
};

let updated_profile = db.update_user_profile(update_request).await?;

// Query historical data
let snapshot_query = ProfileSnapshotQuery {
    fid: 12345,
    start_timestamp: Some(1640995200),
    end_timestamp: Some(1640995800),
    limit: Some(10),
    offset: None,
};

let snapshots = db.get_profile_snapshots(snapshot_query).await?;
```

### Semantic Search with Vectors

The system supports vector embeddings for semantic search:

```sql
-- Search for similar profiles
SELECT 
    fid,
    username,
    display_name,
    bio,
    (profile_embedding <=> query_embedding) as similarity_score
FROM user_profile_snapshots
WHERE (profile_embedding <=> query_embedding) < 0.8
ORDER BY similarity_score
LIMIT 20;
```

## 🏗️ Architecture

### RAG Foundation Framework
- **PostgreSQL Core**: Robust database foundation for RAG applications
- **Vector Search Engine**: pgvector integration for semantic similarity
- **Query Interface**: Rich database query capabilities and analytics
- **Data Synchronization**: Complete Farcaster data sync from snapchain
- **RAG Application Layer**: Ready-to-use foundation for building RAG apps

### Sync Service
- **SyncService**: Orchestrates the synchronization process
- **ShardProcessor**: Processes individual shard chunks
- **SnapchainClient**: Communicates with snapchain gRPC service
- **StateManager**: Manages sync state persistence

### Lock File System
- **SyncLockFile**: Tracks running sync processes
- **PID Management**: Prevents concurrent sync operations
- **Progress Tracking**: Real-time sync progress monitoring
- **Graceful Shutdown**: Clean process termination

### Database Layer
- **SQLx Integration**: Async PostgreSQL operations
- **Migration System**: Schema versioning and updates
- **Connection Pooling**: Efficient database connections
- **Vector Support**: pgvector integration for semantic search
- **Query Engine**: Advanced database query capabilities

## 🧪 Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run strict tests (recommended for development)
make test-strict

# Run quick tests (unit tests only)
make test-quick

# Run integration tests only
make test-integration

# Run specific test categories
cargo test integration_sync_test
cargo test grpc_shard_chunks_test
cargo test database_tests

# Run with verbose output
cargo test -- --nocapture
```

### Strict Testing Configuration

SnapRAG includes a comprehensive strict testing setup that ensures high code quality:

- **Smart Warning Handling**: Automatically distinguishes between generated code and hand-written code warnings
- **Timeout Protection**: Prevents tests from hanging indefinitely
- **Comprehensive Validation**: Tests strict configuration functionality
- **Intelligent Error Detection**: Differentiates between actual test failures and generated code warnings

```bash
# Run strict tests with intelligent warning handling
./scripts/run_strict_tests.sh

# Or use the Makefile target
make test-strict
```

### Test Categories

- **Integration Tests**: End-to-end CLI functionality testing
- **gRPC Tests**: Real snapchain service interaction tests
- **Database Tests**: Database operations and schema tests
- **Unit Tests**: Individual component testing
- **Strict Validation Tests**: Test configuration and warning handling

## 🔍 Data Types

### UserDataType

- `Pfp`: Profile Picture
- `Display`: Display Name
- `Bio`: Bio/Description
- `Url`: Website URL
- `Username`: Username
- `Location`: Location
- `Twitter`: Twitter username
- `Github`: GitHub username
- `Banner`: Banner image
- `PrimaryAddressEthereum`: Ethereum address
- `PrimaryAddressSolana`: Solana address
- `ProfileToken`: Profile token (CAIP-19)

### UsernameType

- `Fname`: Farcaster name
- `EnsL1`: ENS L1
- `Basename`: Basename

## 🔧 Configuration

### Configuration File

Create a `config.toml` file in your project root (copy from `config.example.toml`):

```toml
# Database Configuration
[database]
url = "postgresql://username:password@your-db-host:5432/your-database"
max_connections = 20
min_connections = 5
connection_timeout = 30

# Snapchain Configuration
[snapchain]
http_endpoint = "http://your-snapchain-host:8080"
grpc_endpoint = "your-snapchain-host:8080"

# Logging Configuration
[logging]
level = "info"
backtrace = true

# Embeddings Configuration
[embeddings]
dimension = 1536
model = "text-embedding-ada-002"

# Performance Configuration
[performance]
enable_vector_indexes = true
vector_index_lists = 100
```

## 🎯 Key Design Principles

1. **No Data Loss**: All historical data is preserved
2. **Efficient Queries**: Current state is optimized for fast access
3. **Complete Audit Trail**: Every change is tracked with timestamps and message hashes
4. **Vector Support**: Built-in support for semantic search and RAG applications
5. **Snapshot-based History**: Complete profile snapshots at each change point

## 🚨 Troubleshooting

### Common Issues

#### 1. Database Connection Failed
```bash
# Check if your remote database is accessible
psql -h your-db-host -U your-username -d your-database -c "SELECT 1;"

# Check network connectivity
ping your-db-host

# Verify configuration file exists and is valid
ls -la config.toml
cargo run --bin migrate  # This will show detailed connection info
```

#### 2. Required Extensions Not Found
```bash
# Connect to your remote database and enable required extensions:
psql -h your-db-host -U your-username -d your-database -c "CREATE EXTENSION IF NOT EXISTS vector;"
psql -h your-db-host -U your-username -d your-database -c "CREATE EXTENSION IF NOT EXISTS pg_trgm;"

# If extensions are not installed on your remote database, contact your database administrator
# or install them on your local development environment:
sudo apt-get install postgresql-15-pgvector postgresql-15-pgtrgm  # Ubuntu/Debian
brew install pgvector                        # macOS (pg_trgm is included in PostgreSQL contrib)
```

#### 3. Sync Process Issues
```bash
# Check if sync is running
cargo run sync status

# Stop any running sync
cargo run sync stop

# Reset and start fresh
cargo run reset --force
cargo run sync start
```

#### 4. gRPC Connection Issues
```bash
# Check snapchain endpoint connectivity
curl http://your-snapchain-host:8080/v1/info

# Verify gRPC endpoint
telnet your-snapchain-host 8080
```

## 📈 Performance Tuning

### PostgreSQL Configuration
Add to your `postgresql.conf`:

```conf
# Memory settings
shared_buffers = 256MB
effective_cache_size = 1GB
work_mem = 4MB

# Vector-specific settings
max_connections = 200
shared_preload_libraries = 'vector'

# Index settings
maintenance_work_mem = 64MB
```

### Connection Pool Settings
```rust
// In your application code
let pool = PgPool::builder()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(30))
    .build(&database_url)
    .await?;
```

## 🔗 Dependencies

- `sqlx`: Async PostgreSQL driver
- `serde`: Serialization/deserialization
- `chrono`: Date/time handling
- `uuid`: UUID generation
- `pgvector`: Vector similarity search
- `tokio`: Async runtime
- `anyhow`: Error handling
- `thiserror`: Custom error types
- `tonic`: gRPC client
- `prost`: Protocol buffers
- `reqwest`: HTTP client
- `libc`: System calls

## 📝 License

This project is licensed under the GPTv3 License - see the [LICENSE](LICENSE) file for details.

## 🛠️ Development Workflow

### Code Quality Standards

SnapRAG follows strict development standards to ensure high code quality:

- **Strict Testing**: All tests must pass with zero warnings (except generated code)
- **Code Formatting**: Automatic formatting with `rustfmt`
- **Linting**: Comprehensive clippy checks with strict settings
- **Documentation**: All public APIs must be documented

### Development Commands

```bash
# Set up development environment
make check-config  # Verify configuration
make migrate       # Set up database

# Development workflow
make test-strict   # Run strict tests (recommended)
make check         # Run clippy and format checks
make fix           # Auto-fix formatting and clippy issues
make docs          # Generate documentation

# Before committing
make test-strict && make check && make docs
```

### Cursor IDE Integration

SnapRAG includes comprehensive Cursor IDE rules for enhanced development experience:

- **Project-specific rules**: Tailored for Farcaster data synchronization
- **CI/CD guidelines**: Automated testing and deployment rules
- **Rust standards**: Best practices for Rust development
- **Notification system**: Task completion notifications

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes following the development workflow
4. Add tests for new functionality
5. Run the strict test suite (`make test-strict`)
6. Ensure all checks pass (`make check`)
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Submit a pull request

### Pull Request Guidelines

- Ensure all tests pass with `make test-strict`
- Follow the existing code style and formatting
- Add documentation for new public APIs
- Include tests for new functionality
- Update README if adding new features or changing behavior

## 📞 Support

For questions, issues, or contributions, please open an issue on the GitHub repository.

### Getting Help

- 📖 **Documentation**: Check this README and inline code documentation
- 🐛 **Bug Reports**: Use GitHub issues with detailed reproduction steps
- 💡 **Feature Requests**: Open a GitHub issue with use case description
- 💬 **Discussions**: Use GitHub Discussions for questions and ideas
