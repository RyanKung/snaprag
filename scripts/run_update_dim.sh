#!/bin/bash
echo "🔄 Updating vector dimensions from 1536 to 768 for Ollama..."
echo ""
echo "This will:"
echo "  1. Drop existing vector indexes"
echo "  2. Change vector columns from VECTOR(1536) to VECTOR(768)"
echo "  3. Recreate indexes with new dimensions"
echo ""
echo "⚠️  All existing embeddings will be cleared!"
echo ""
read -p "Continue? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]
then
    psql -U snaprag -d snaprag -h localhost -f update_vector_dim.sql
    echo ""
    echo "✅ Database updated! Now run:"
    echo "   cargo run -- embeddings backfill --force"
else
    echo "❌ Cancelled"
fi
