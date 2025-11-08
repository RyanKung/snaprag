#!/bin/bash
# ============================================================================
# SnapRAG Docker Quick Start Script
# ============================================================================

set -e

echo "🐳 SnapRAG Docker Quick Start"
echo "=============================="
echo ""

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first:"
    echo "   https://docs.docker.com/get-docker/"
    exit 1
fi

# Check if Docker is running
if ! docker info &> /dev/null; then
    echo "❌ Docker daemon is not running. Please start Docker Desktop."
    exit 1
fi

# Check if config.toml exists
if [ ! -f "config.toml" ]; then
    echo "📝 Creating config.toml from example..."
    cp config.example.toml config.toml
    echo ""
    echo "⚠️  IMPORTANT: Please edit config.toml before starting services:"
    echo "   1. Update database URL to: postgresql://snaprag:snaprag_password@postgres:5432/snaprag"
    echo "   2. Update snapchain endpoints to your snapchain node"
    echo ""
    read -p "Press Enter after editing config.toml, or Ctrl+C to exit..."
fi

# Build Docker image
echo ""
echo "🏗️  Building Docker image..."
docker build -t snaprag:latest .

echo ""
echo "✅ Docker image built successfully!"
echo ""

# Ask if user wants to start docker-compose
read -p "Do you want to start all services with docker-compose? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo "🚀 Starting all services..."
    docker-compose up -d
    
    echo ""
    echo "✅ All services started!"
    echo ""
    echo "📊 Service URLs:"
    echo "   - SnapRAG API: http://localhost:3000"
    echo "   - PostgreSQL:  localhost:5432"
    echo "   - Redis:       localhost:6379"
    echo ""
    echo "🔍 View logs:"
    echo "   docker-compose logs -f"
    echo ""
    echo "📚 Full documentation:"
    echo "   cat DOCKER_DEPLOYMENT.md"
else
    echo ""
    echo "ℹ️  To start services later, run:"
    echo "   docker-compose up -d"
fi

echo ""
echo "🎉 Setup complete!"

