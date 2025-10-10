#!/bin/bash

echo "🚇 Starting SSH Tunnel to Ollama"
echo "================================="
echo ""
echo "This will create a tunnel from:"
echo "  localhost:11434 → 192.168.1.192:11434"
echo ""
echo "Keep this terminal open while using SnapRAG."
echo "Press Ctrl+C to stop the tunnel."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if tunnel already exists
if lsof -i :11434 > /dev/null 2>&1; then
    echo "⚠️  Port 11434 is already in use!"
    echo ""
    echo "Existing connections:"
    lsof -i :11434
    echo ""
    read -p "Kill existing process and continue? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        pkill -f "ssh.*11434.*192.168.1.192"
        sleep 1
    else
        echo "❌ Cancelled"
        exit 1
    fi
fi

echo "🚀 Creating SSH tunnel..."
echo ""

# Start tunnel
ssh -L 11434:localhost:11434 ryan@192.168.1.192 -N

