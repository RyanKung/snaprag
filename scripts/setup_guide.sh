#!/bin/bash
# SnapRAG Database Setup Guide
# This script provides step-by-step instructions for setting up PostgreSQL for SnapRAG

set -e

echo "=========================================="
echo "SnapRAG Database Setup Guide"
echo "=========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect platform
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    PLATFORM="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    PLATFORM="macos"
else
    PLATFORM="unknown"
fi

print_info "Detected platform: $PLATFORM"
echo ""

# ==============================================================================
# STEP 1: Install PostgreSQL
# ==============================================================================

echo "=========================================="
echo "STEP 1: Install PostgreSQL with pgvector"
echo "=========================================="
echo ""

if [ "$PLATFORM" == "linux" ]; then
    print_info "For Ubuntu/Debian:"
    echo "  sudo apt update"
    echo "  sudo apt install -y postgresql postgresql-contrib"
    echo ""
    print_info "For Amazon Linux 2023 / EC2:"
    echo "  sudo dnf install -y postgresql15 postgresql15-server postgresql15-contrib"
    echo "  sudo postgresql-setup --initdb"
    echo "  sudo systemctl enable postgresql"
    echo "  sudo systemctl start postgresql"
    echo ""
elif [ "$PLATFORM" == "macos" ]; then
    print_info "For macOS (using Homebrew):"
    echo "  brew install postgresql@15"
    echo "  brew services start postgresql@15"
    echo ""
fi

print_warn "pgvector extension is REQUIRED for vector similarity search"
print_info "Install pgvector:"
echo "  git clone https://github.com/pgvector/pgvector.git"
echo "  cd pgvector"
echo "  make"
echo "  sudo make install"
echo ""

# ==============================================================================
# STEP 2: Create Database and User
# ==============================================================================

echo "=========================================="
echo "STEP 2: Create Database and User"
echo "=========================================="
echo ""

print_info "Run the setup SQL script as PostgreSQL superuser:"
echo ""

if [ "$PLATFORM" == "linux" ]; then
    echo "  sudo -u postgres psql -f scripts/setup_database.sql"
elif [ "$PLATFORM" == "macos" ]; then
    echo "  psql postgres -f scripts/setup_database.sql"
fi

echo ""
print_info "Or run commands manually:"
echo ""
echo "  # Connect to PostgreSQL"
if [ "$PLATFORM" == "linux" ]; then
    echo "  sudo -u postgres psql"
elif [ "$PLATFORM" == "macos" ]; then
    echo "  psql postgres"
fi
echo ""
echo "  -- Create user and database"
echo "  CREATE USER snaprag WITH PASSWORD 'your-password';"
echo "  CREATE DATABASE snaprag OWNER snaprag;"
echo "  GRANT ALL PRIVILEGES ON DATABASE snaprag TO snaprag;"
echo ""
echo "  -- Connect to database and enable extensions"
echo "  \\c snaprag"
echo "  CREATE EXTENSION IF NOT EXISTS vector;"
echo "  CREATE EXTENSION IF NOT EXISTS pg_trgm;"
echo "  CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";"
echo ""
echo "  -- Grant permissions"
echo "  GRANT ALL ON ALL TABLES IN SCHEMA public TO snaprag;"
echo "  GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO snaprag;"
echo "  GRANT ALL ON ALL FUNCTIONS IN SCHEMA public TO snaprag;"
echo ""
echo "  ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO snaprag;"
echo "  ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO snaprag;"
echo "  ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO snaprag;"
echo ""

# ==============================================================================
# STEP 3: Configure PostgreSQL for External Connections (Optional)
# ==============================================================================

echo "=========================================="
echo "STEP 3: Configure External Connections (Optional)"
echo "=========================================="
echo ""

print_warn "Only needed if connecting from a different machine"
echo ""

if [ "$PLATFORM" == "linux" ]; then
    print_info "Edit PostgreSQL configuration:"
    echo "  sudo vim /var/lib/pgsql/15/data/postgresql.conf"
    echo ""
    echo "  Change:"
    echo "    listen_addresses = 'localhost'"
    echo "  To:"
    echo "    listen_addresses = '*'"
    echo ""
    print_info "Edit client authentication:"
    echo "  sudo vim /var/lib/pgsql/15/data/pg_hba.conf"
    echo ""
    echo "  Add line:"
    echo "    host    all    all    0.0.0.0/0    md5"
    echo ""
    print_info "Restart PostgreSQL:"
    echo "  sudo systemctl restart postgresql"
    echo ""
elif [ "$PLATFORM" == "macos" ]; then
    print_info "Edit PostgreSQL configuration:"
    echo "  vim /opt/homebrew/var/postgresql@15/postgresql.conf"
    echo ""
    echo "  Change:"
    echo "    listen_addresses = 'localhost'"
    echo "  To:"
    echo "    listen_addresses = '*'"
    echo ""
    print_info "Restart PostgreSQL:"
    echo "  brew services restart postgresql@15"
    echo ""
fi

# ==============================================================================
# STEP 4: Configure SnapRAG
# ==============================================================================

echo "=========================================="
echo "STEP 4: Configure SnapRAG"
echo "=========================================="
echo ""

print_info "Copy example configuration:"
echo "  cp config.example.toml config.toml"
echo ""

print_info "Edit config.toml and update database settings:"
echo "  [database]"
echo "  url = \"postgresql://snaprag:your-password@localhost/snaprag\""
echo "  max_connections = 100"
echo "  min_connections = 2"
echo ""

# ==============================================================================
# STEP 5: Initialize Database
# ==============================================================================

echo "=========================================="
echo "STEP 5: Initialize Database Schema"
echo "=========================================="
echo ""

print_info "Run database initialization:"
echo "  cargo build --release"
echo "  ./target/release/snaprag init --force"
echo ""

print_info "Verify setup:"
echo "  ./target/release/snaprag check"
echo ""

# ==============================================================================
# STEP 6: AWS/Cloud Setup (Optional)
# ==============================================================================

echo "=========================================="
echo "STEP 6: AWS RDS Setup (Optional)"
echo "=========================================="
echo ""

print_info "For AWS RDS PostgreSQL:"
echo ""
echo "1. Create RDS PostgreSQL 15+ instance with pgvector support"
echo "2. Enable public accessibility (if needed)"
echo "3. Configure security group to allow port 5432"
echo "4. Run setup script:"
echo "   psql -h your-rds.region.rds.amazonaws.com -U postgres -f scripts/setup_database_aws.sql"
echo ""
echo "5. Update config.toml with RDS endpoint:"
echo "   [database]"
echo "   url = \"postgresql://snaprag:password@your-rds.region.rds.amazonaws.com:5432/snaprag\""
echo ""

# ==============================================================================
# Verification
# ==============================================================================

echo "=========================================="
echo "Verification Commands"
echo "=========================================="
echo ""

print_info "Test PostgreSQL connection:"
echo "  psql -U snaprag -d snaprag -c 'SELECT version();'"
echo ""

print_info "Verify extensions:"
echo "  psql -U snaprag -d snaprag -c 'SELECT extname, extversion FROM pg_extension;'"
echo ""

print_info "Check SnapRAG setup:"
echo "  snaprag check"
echo ""

# ==============================================================================
# Troubleshooting
# ==============================================================================

echo "=========================================="
echo "Troubleshooting"
echo "=========================================="
echo ""

print_warn "Common Issues:"
echo ""
echo "1. 'extension \"vector\" not found'"
echo "   → Install pgvector: https://github.com/pgvector/pgvector"
echo ""
echo "2. 'connection refused'"
echo "   → Check PostgreSQL is running: systemctl status postgresql"
echo "   → Verify connection string in config.toml"
echo ""
echo "3. 'password authentication failed'"
echo "   → Check password in config.toml matches database user"
echo "   → Verify pg_hba.conf allows md5 authentication"
echo ""
echo "4. 'permission denied'"
echo "   → Ensure snaprag user has all privileges"
echo "   → Run GRANT commands as PostgreSQL superuser"
echo ""

print_info "For more help, see: https://github.com/your-org/snaprag/issues"
echo ""

echo "=========================================="
echo "Setup guide complete!"
echo "=========================================="


