#!/bin/bash
# ============================================================================
# SnapRAG Docker Build Test Script
# ============================================================================

set -e

echo "🔍 SnapRAG Docker Build Test"
echo "=============================="
echo ""

# Check Docker daemon
echo "1️⃣ Checking Docker daemon..."
if ! docker info &> /dev/null; then
    echo "❌ Docker daemon is not running."
    echo ""
    echo "Please start Docker daemon:"
    echo "  - Docker Desktop: Open Docker Desktop application"
    echo "  - colima: brew install colima && colima start"
    echo "  - OrbStack: Open OrbStack application"
    echo ""
    exit 1
fi
echo "✅ Docker daemon is running"
echo ""

# Show Docker info
echo "2️⃣ Docker environment:"
docker --version
docker-compose --version
echo ""

# Build test image
echo "3️⃣ Building Docker image (this may take 2-3 minutes)..."
echo ""
docker build \
    --progress=plain \
    -t snaprag:test \
    -f Dockerfile \
    . 2>&1 | tail -50

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Docker image built successfully!"
    echo ""
    
    # Show image info
    echo "4️⃣ Image information:"
    docker images snaprag:test
    echo ""
    
    # Test run
    echo "5️⃣ Testing image..."
    docker run --rm snaprag:test --version
    echo ""
    
    echo "✅ All tests passed!"
    echo ""
    echo "Next steps:"
    echo "  1. Edit config.toml (update database and snapchain URLs)"
    echo "  2. Run: docker-compose up -d"
    echo "  3. Access: http://localhost:3000"
else
    echo ""
    echo "❌ Docker build failed. Check the output above for errors."
    exit 1
fi

