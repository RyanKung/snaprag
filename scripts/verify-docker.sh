#!/bin/bash
# ============================================================================
# Docker Environment Verification Script
# ============================================================================

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔍 SnapRAG Docker Environment Check${NC}"
echo "======================================"
echo ""

# Function to check command
check_command() {
    if command -v $1 &> /dev/null; then
        VERSION=$($1 --version 2>&1 | head -1)
        echo -e "${GREEN}✅ $1${NC}: $VERSION"
        return 0
    else
        echo -e "${RED}❌ $1${NC}: Not installed"
        return 1
    fi
}

# Check Docker CLI
check_command docker

# Check Docker Compose
check_command docker-compose || check_command "docker compose"

echo ""

# Check Docker daemon
echo "Checking Docker daemon..."
if docker info &> /dev/null; then
    echo -e "${GREEN}✅ Docker daemon is running${NC}"
    echo ""
    
    # Show Docker info
    echo "Docker configuration:"
    docker info | grep -E "(Server Version|Operating System|CPUs|Total Memory|Docker Root Dir)" | sed 's/^/  /'
    echo ""
    
    DOCKER_OK=true
else
    echo -e "${RED}❌ Docker daemon is NOT running${NC}"
    echo ""
    echo -e "${YELLOW}To start Docker daemon, choose one option:${NC}"
    echo ""
    echo "Option 1: OrbStack (Recommended - fast and lightweight)"
    echo "  brew install orbstack"
    echo "  open -a OrbStack"
    echo ""
    echo "Option 2: Colima (Open source, lightweight)"
    echo "  brew install colima"
    echo "  colima start --cpu 4 --memory 8"
    echo ""
    echo "Option 3: Docker Desktop (Official)"
    echo "  Download from: https://www.docker.com/products/docker-desktop/"
    echo ""
    DOCKER_OK=false
fi

# Check if Dockerfile exists
echo "Checking Docker configuration files..."
if [ -f "Dockerfile" ]; then
    echo -e "${GREEN}✅ Dockerfile${NC} exists"
else
    echo -e "${RED}❌ Dockerfile${NC} not found"
fi

if [ -f "docker-compose.yml" ]; then
    echo -e "${GREEN}✅ docker-compose.yml${NC} exists"
else
    echo -e "${RED}❌ docker-compose.yml${NC} not found"
fi

if [ -f "config.toml" ]; then
    echo -e "${GREEN}✅ config.toml${NC} exists"
else
    echo -e "${YELLOW}⚠️  config.toml${NC} not found (run: cp config.example.toml config.toml)"
fi

echo ""

# Summary
if [ "$DOCKER_OK" = true ]; then
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}✅ Environment Ready!${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo "You can now:"
    echo ""
    echo "  1. Test build:"
    echo -e "     ${BLUE}./scripts/test-docker-build.sh${NC}"
    echo ""
    echo "  2. Start services:"
    echo -e "     ${BLUE}docker-compose up -d${NC}"
    echo ""
    echo "  3. View logs:"
    echo -e "     ${BLUE}docker-compose logs -f${NC}"
    echo ""
else
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}⚠️  Action Required${NC}"
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo "Please start Docker daemon first, then run this script again."
    echo ""
    exit 1
fi

