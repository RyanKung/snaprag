#!/bin/bash

# Check Ollama server configuration and GPU utilization
echo "🔍 Checking Ollama server configuration..."

# Check if Ollama is running and accessible
echo "📡 Testing Ollama connection..."
curl -s http://192.168.1.192:80/api/tags > /dev/null
if [ $? -eq 0 ]; then
    echo "✅ Ollama server is accessible"
else
    echo "❌ Ollama server is not accessible"
    exit 1
fi

# Check available models
echo "📋 Available embedding models:"
curl -s http://192.168.1.192:80/api/tags | jq -r '.models[] | select(.name | contains("embed")) | .name'

# Check current model being used
echo "🎯 Current embedding model: nomic-embed-text"

# Test embedding generation speed
echo "⚡ Testing embedding generation speed..."
start_time=$(date +%s.%N)
curl -s -X POST http://192.168.1.192:80/api/embeddings \
  -H "Content-Type: application/json" \
  -d '{"model": "nomic-embed-text", "prompt": "Hello world"}' > /dev/null
end_time=$(date +%s.%N)
duration=$(echo "$end_time - $start_time" | bc)
echo "⏱️  Single embedding generation time: ${duration}s"

# Check GPU utilization on the server (if accessible)
echo "🖥️  GPU utilization check:"
echo "Note: Run 'nvidia-smi' on the server to check GPU utilization"

# Recommendations
echo ""
echo "💡 Optimization recommendations:"
echo "1. Increase Ollama server's num_parallel if not already set"
echo "2. Ensure GPU memory is properly allocated to Ollama"
echo "3. Check if Ollama is using GPU acceleration"
echo "4. Consider using a larger embedding model for better GPU utilization"
