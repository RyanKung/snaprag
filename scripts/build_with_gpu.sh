#!/bin/bash

# SnapRAG GPU Build Script
# This script builds SnapRAG with local GPU support, handling CUDA compilation issues

set -e

echo "🚀 Building SnapRAG with local GPU support..."

# Check if we're on macOS (Metal) or Linux/Windows (CUDA)
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "🍎 Detected macOS - using Metal GPU acceleration"
    cargo build --release --features local-gpu
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "🐧 Detected Linux - using CUDA GPU acceleration"
    # Set NVCC compiler to avoid CUDA header conflicts
    export NVCC_CCBIN=/usr/bin/gcc
    cargo build --release --features local-gpu
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]]; then
    echo "🪟 Detected Windows - using CUDA GPU acceleration"
    # Set NVCC compiler to avoid CUDA header conflicts
    export NVCC_CCBIN=gcc
    cargo build --release --features local-gpu
else
    echo "❓ Unknown OS: $OSTYPE"
    echo "Building without GPU support..."
    cargo build --release
fi

echo "✅ Build completed successfully!"
echo ""
echo "📝 Usage:"
echo "  ./target/release/snaprag --help"
echo ""
echo "🔧 GPU Configuration:"
echo "  Add to config.toml:"
echo "  [[embeddings.endpoints]]"
echo "  name = \"local_gpu\""
echo "  endpoint = \"local\""
echo "  model = \"nomic-ai/nomic-embed-text-v1\""
echo "  provider = \"local_gpu\""
