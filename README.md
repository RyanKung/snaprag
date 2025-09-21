# SnapRAG - Historical Profile Management System

A Rust-based system for managing historical user profiles with PostgreSQL, designed for RAG (Retrieval-Augmented Generation) applications. This system preserves complete profile history while providing efficient current state access.

## 🚀 Quick Start

```bash
# 1. Clone and setup
git clone <repository-url> && cd snaprag

# 2. Create configuration file
cp config.example.toml config.toml

# 3. Edit config.toml with your database connection details
# Update the database.url field with your actual database connection string

# 4. Check your configuration
make check-config  # Verify config.toml is valid

# 5. Ensure pgvector extension is enabled on your database
# Connect to your database and run:
# CREATE EXTENSION IF NOT EXISTS vector;

# 6. Run database migrations and application
make migrate     # Run database migrations
make run         # Run the application
```

## Features

- **Historical Profile Preservation**: Complete snapshot history of user profile changes
- **Efficient Current State Access**: Fast queries for current profile data
- **Vector Embeddings Support**: Built-in support for pgvector for semantic search
- **Change Tracking**: Detailed audit trail of all profile modifications
- **Username Proofs**: Support for Farcaster-style username verification
- **Activity Timeline**: Comprehensive user activity tracking
- **No Data Cleanup**: All historical data is preserved indefinitely

## Database Schema

The system uses the following main tables:

- `user_profiles`: Current profile state (latest values only)
- `user_profile_snapshots`: Historical profile snapshots
- `user_data_changes`: Detailed change tracking
- `username_proofs`: Username verification records
- `user_activity_timeline`: User activity history
- `user_profile_trends`: Aggregated trend data

## Installation

### Prerequisites

- Rust 1.70+ 
- PostgreSQL 15+ with pgvector extension
- Remote database access

### Quick Start with Remote Database

```bash
# 1. Clone the repository
git clone <repository-url>
cd snaprag

# 2. Create and configure config.toml
cp config.example.toml config.toml
# Edit config.toml and update the database.url field

# 3. Ensure pgvector extension is enabled on your remote database
# Connect to your database and run:
# CREATE EXTENSION IF NOT EXISTS vector;

# 4. Run database migrations
make migrate

# 5. Build and run the application
cargo build
cargo run
```

### Remote Database Setup

For production or development with remote databases:

```bash
# 1. Ensure your remote PostgreSQL has pgvector extension
# Connect to your remote database and run:
psql -h your-db-host -U your-username -d your-database
CREATE EXTENSION IF NOT EXISTS vector;
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
# 1. Install PostgreSQL and pgvector extension
# On Ubuntu/Debian:
sudo apt-get install postgresql-15 postgresql-15-pgvector

# On macOS with Homebrew:
brew install postgresql@15 pgvector

# On CentOS/RHEL:
sudo yum install postgresql15-server postgresql15-contrib
# Then compile pgvector from source

# 2. Create database and user
sudo -u postgres psql
CREATE DATABASE snaprag;
CREATE USER snaprag WITH PASSWORD 'snaprag123';
GRANT ALL PRIVILEGES ON DATABASE snaprag TO snaprag;
\q

# 3. Connect to database and enable pgvector extension
psql -U snaprag -d snaprag -h localhost
CREATE EXTENSION IF NOT EXISTS vector;
\q

# 4. Create and configure config.toml
cp config.example.toml config.toml
# Edit config.toml and update database.url to: postgresql://snaprag:snaprag123@localhost/snaprag

# 5. Run migrations and build
make migrate
cargo build
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
| `make build` | Build the project |
| `make build-release` | Build in release mode |
| `make clean` | Clean build artifacts |
| `make check` | Run clippy and format checks |
| `make fix` | Fix clippy and format issues |
| `make docs` | Generate documentation |
| `make bench` | Run benchmarks |

## Usage

### CLI Commands

SnapRAG provides a comprehensive CLI tool for managing data synchronization and database operations:

#### Data Synchronization

```bash
# Run all sync (historical + real-time)
cargo run --bin cli sync all

# Run historical sync only (sync all past data)
cargo run --bin cli sync historical

# Run real-time sync only (monitor new data)
cargo run --bin cli sync realtime

# Show sync status and statistics
cargo run --bin cli sync status
```

#### Data Management

```bash
# List FIDs from database
cargo run --bin cli list fid --limit 50

# List user profiles
cargo run --bin cli list profiles --limit 20

# List casts
cargo run --bin cli list casts --limit 100

# List follow relationships
cargo run --bin cli list follows --limit 50

# Clear all synchronized data (with confirmation)
cargo run --bin cli clear

# Clear all data without confirmation (force)
cargo run --bin cli clear --force
```

#### Database Operations

```bash
# Run database migrations
cargo run --bin migrate

# Check configuration
cargo run --bin check_config
```

### Creating a User Profile

```rust
use snaprag::models::*;
use snaprag::database::Database;

let create_request = CreateUserProfileRequest {
    fid: 12345,
    username: Some("alice".to_string()),
    display_name: Some("Alice Smith".to_string()),
    bio: Some("Blockchain enthusiast".to_string()),
    // ... other fields
    message_hash: vec![1, 2, 3, 4, 5],
    timestamp: 1640995200,
};

let profile = db.create_user_profile(create_request).await?;
```

### Updating a Profile

```rust
let update_request = UpdateUserProfileRequest {
    fid: 12345,
    data_type: UserDataType::Bio,
    new_value: "Senior blockchain developer".to_string(),
    message_hash: vec![6, 7, 8, 9, 10],
    timestamp: 1640995800,
};

let updated_profile = db.update_user_profile(update_request).await?;
```

### Querying Historical Data

```rust
// Get profile snapshots
let snapshot_query = ProfileSnapshotQuery {
    fid: 12345,
    start_timestamp: Some(1640995200),
    end_timestamp: Some(1640995800),
    limit: Some(10),
    offset: None,
};

let snapshots = db.get_profile_snapshots(snapshot_query).await?;

// Get profile at specific timestamp
let snapshot = db.get_profile_snapshot_at_timestamp(12345, 1640995500).await?;
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

## Data Types

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

## Key Design Principles

1. **No Data Loss**: All historical data is preserved
2. **Efficient Queries**: Current state is optimized for fast access
3. **Complete Audit Trail**: Every change is tracked with timestamps and message hashes
4. **Vector Support**: Built-in support for semantic search and RAG applications
5. **Snapshot-based History**: Complete profile snapshots at each change point

## Error Handling

The system uses a comprehensive error handling approach:

```rust
use snaprag::Result;

match db.get_user_profile(12345).await {
    Ok(Some(profile)) => println!("Found profile: {:?}", profile),
    Ok(None) => println!("Profile not found"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Database Configuration

### Configuration File

Create a `config.toml` file in your project root (copy from `config.example.toml`):

```toml
# Database Configuration
[database]
url = "postgresql://username:password@your-db-host:5432/your-database"
max_connections = 20
min_connections = 5
connection_timeout = 30

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

### Database Connection Options

You can customize the database connection in `Cargo.toml`:

```toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid", "json"] }
```

### Vector Embedding Configuration

The system supports configurable vector dimensions. Update the schema migration if needed:

```sql
-- Default: 1536 dimensions (OpenAI ada-002)
profile_embedding VECTOR(1536)

-- For other models, adjust accordingly:
-- profile_embedding VECTOR(384)   -- sentence-transformers
-- profile_embedding VECTOR(768)   -- BERT-base
```

## Troubleshooting

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

#### 2. pgvector Extension Not Found
```bash
# Connect to your remote database and enable pgvector:
psql -h your-db-host -U your-username -d your-database -c "CREATE EXTENSION IF NOT EXISTS vector;"

# If pgvector is not installed on your remote database, contact your database administrator
# or install it on your local development environment:
sudo apt-get install postgresql-15-pgvector  # Ubuntu/Debian
brew install pgvector                        # macOS
```

#### 3. Migration Errors
```bash
# Check if you have proper permissions on your remote database
psql -h your-db-host -U your-username -d your-database -c "SELECT current_user, current_database();"

# If you need to recreate tables, connect to your database and drop them manually:
psql -h your-db-host -U your-username -d your-database
DROP TABLE IF EXISTS user_profile_trends CASCADE;
DROP TABLE IF EXISTS user_activity_timeline CASCADE;
DROP TABLE IF EXISTS username_proofs CASCADE;
DROP TABLE IF EXISTS user_data_changes CASCADE;
DROP TABLE IF EXISTS user_profile_snapshots CASCADE;
DROP TABLE IF EXISTS user_profiles CASCADE;
\q

# Then run migrations again
make migrate
# Or run the migration binary directly:
cargo run --bin migrate
```

#### 4. Permission Denied Errors
```bash
# Contact your database administrator to grant proper permissions:
# GRANT ALL PRIVILEGES ON DATABASE your_database TO your_username;
# GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO your_username;
# GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO your_username;

# Or if you have admin access:
psql -h your-db-host -U admin-username -d your-database
GRANT ALL PRIVILEGES ON DATABASE your_database TO your_username;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO your_username;
\q
```

#### 5. Rust Compilation Issues
```bash
# Update Rust toolchain
rustup update

# Clean and rebuild
cargo clean
cargo build

# Check for dependency conflicts
cargo tree
```

### Performance Tuning

#### PostgreSQL Configuration
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

#### Connection Pool Settings
```rust
// In your application code
let pool = PgPool::builder()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(30))
    .build(&database_url)
    .await?;
```

## Dependencies

- `sqlx`: Async PostgreSQL driver
- `serde`: Serialization/deserialization
- `chrono`: Date/time handling
- `uuid`: UUID generation
- `pgvector`: Vector similarity search
- `tokio`: Async runtime
- `anyhow`: Error handling
- `thiserror`: Custom error types
