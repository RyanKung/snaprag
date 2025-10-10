#!/bin/bash

echo "🔍 SnapRAG Ollama Configuration Checker"
echo "========================================"
echo ""

# Read config
CONFIG_FILE="config.local.toml"
if [ ! -f "$CONFIG_FILE" ]; then
    CONFIG_FILE="config.toml"
fi

echo "📋 Using config file: $CONFIG_FILE"
echo ""

# Extract endpoint and model from config
ENDPOINT=$(grep "llm_endpoint" $CONFIG_FILE | cut -d'"' -f2)
MODEL=$(grep "^model" $CONFIG_FILE | cut -d'"' -f2)
DIMENSION=$(grep "^dimension" $CONFIG_FILE | cut -d'=' -f2 | tr -d ' ')

echo "📊 Current Configuration:"
echo "  Endpoint: $ENDPOINT"
echo "  Model: $MODEL"
echo "  Dimension: $DIMENSION"
echo ""

# Check if Ollama is running
echo "🔌 Checking Ollama connection..."
if curl -s "${ENDPOINT}/api/tags" > /dev/null 2>&1; then
    echo "  ✅ Ollama is reachable at $ENDPOINT"
else
    echo "  ❌ Cannot connect to Ollama at $ENDPOINT"
    echo "  💡 Tip: Check if Ollama is running: ollama serve"
    exit 1
fi
echo ""

# List available models
echo "📦 Available Ollama models:"
MODELS=$(curl -s "${ENDPOINT}/api/tags" | grep -o '"name":"[^"]*"' | cut -d'"' -f4)
if [ -z "$MODELS" ]; then
    echo "  ❌ No models found"
    echo "  💡 Tip: Pull a model: ollama pull nomic-embed-text"
else
    echo "$MODELS" | while read -r model; do
        echo "  - $model"
    done
fi
echo ""

# Check if configured model exists
echo "🎯 Checking configured model: $MODEL"
if echo "$MODELS" | grep -q "^${MODEL}$"; then
    echo "  ✅ Model '$MODEL' is available"
else
    echo "  ⚠️  Model '$MODEL' not found in Ollama"
    echo "  💡 Tip: Pull the model: ollama pull $MODEL"
    echo ""
    echo "  📋 Popular embedding models:"
    echo "     • nomic-embed-text (768 dim) - recommended"
    echo "     • mxbai-embed-large (1024 dim)"
    echo "     • all-minilm (384 dim)"
fi
echo ""

# Test embedding generation
echo "🧪 Testing embedding generation..."
TEST_RESPONSE=$(curl -s -X POST "${ENDPOINT}/api/embeddings" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"${MODEL}\",\"prompt\":\"test\"}" 2>&1)

if echo "$TEST_RESPONSE" | grep -q "embedding"; then
    echo "  ✅ Embedding generation works!"
    EMB_DIM=$(echo "$TEST_RESPONSE" | grep -o '"embedding":\[[^]]*\]' | grep -o ',' | wc -l)
    EMB_DIM=$((EMB_DIM + 1))
    echo "  📏 Actual embedding dimension: $EMB_DIM"
    
    if [ "$EMB_DIM" != "$DIMENSION" ]; then
        echo "  ⚠️  Dimension mismatch!"
        echo "     Config: $DIMENSION, Actual: $EMB_DIM"
        echo "  💡 Update config.local.toml:"
        echo "     dimension = $EMB_DIM"
    fi
else
    echo "  ❌ Embedding generation failed"
    echo "  Response: $TEST_RESPONSE"
fi
echo ""

echo "✅ Diagnosis complete!"
echo ""
echo "Next steps:"
echo "  1. Ensure Ollama is running: ollama serve"
echo "  2. Pull model if missing: ollama pull $MODEL"
echo "  3. Update vector dimensions: ./scripts/run_update_dim.sh"
echo "  4. Test with SnapRAG: cargo run -- embeddings test \"test\""

