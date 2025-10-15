# Cast Query System - Quick Start Guide

## 🚀 Setup

### 1. Sync Cast Data
```bash
# Sync blocks with cast messages
snaprag sync start --from 1250000 --to 1251000
```

### 2. Generate Cast Embeddings
```bash
# Generate embeddings for all casts
snaprag embeddings backfill-casts

# Generate embeddings for first 100 casts
snaprag embeddings backfill-casts --limit 100
```

---

## 📝 Cast Search Commands

### Semantic Search
```bash
# Search for casts about a topic
snaprag cast search "AI agents and automation"

# Adjust sensitivity
snaprag cast search "web3 development" --threshold 0.7 --limit 50

# Show full details
snaprag cast search "Farcaster frames" --detailed
```

### Get User's Recent Casts
```bash
# Show recent casts by FID
snaprag cast recent 374606

# Limit number of casts
snaprag cast recent 374606 --limit 50
```

### Thread Tracking
```bash
# Show full conversation thread
snaprag cast thread abc123def456...

# Limit depth of parent traversal
snaprag cast thread abc123def456... --depth 5
```

**Thread output shows:**
- ⬆️ Parent chain (context leading to the cast)
- 🎯 Target cast
- ⬇️ All replies

---

## 🤖 RAG Q&A on Casts

### Ask Questions About Cast Content
```bash
# Ask about discussions
snaprag rag query-casts "What are people saying about AI agents?"

# With custom parameters
snaprag rag query-casts "Summarize Frame discussions" \
  --limit 20 \
  --threshold 0.6 \
  --temperature 0.8 \
  --max-tokens 1500

# Show source casts
snaprag rag query-casts "What are the main concerns about X?" --verbose
```

**Process:**
1. 🔍 Retrieves relevant casts via semantic search
2. 🔧 Assembles context from multiple casts
3. 💭 Generates insights using LLM
4. 📚 Shows sources with similarity scores

---

## 📊 Example Workflows

### Workflow 1: Explore a Topic
```bash
# 1. Search for relevant casts
snaprag cast search "decentralized social media" --limit 30

# 2. Ask analytical question
snaprag rag query-casts "What are the key benefits of decentralized social mentioned in discussions?"
```

### Workflow 2: Understand a Conversation
```bash
# 1. Find a cast hash from search
snaprag cast search "interesting debate"

# 2. View the full thread
snaprag cast thread <hash>
```

### Workflow 3: Track User Activity
```bash
# 1. See user's recent casts
snaprag cast recent 374608 --limit 20

# 2. See their full activity
snaprag activity 374608 --limit 50

# 3. Filter by activity type
snaprag activity 374608 --activity-type cast_add
```

---

## 🎯 Advanced Features

### Similarity Threshold Tuning
- `--threshold 0.3`: Very broad (may include loosely related)
- `--threshold 0.5`: Balanced (default, good for most cases)
- `--threshold 0.7`: Strict (only highly relevant)
- `--threshold 0.9`: Very strict (almost identical content)

### LLM Parameters
- `--temperature 0.3`: Focused, deterministic answers
- `--temperature 0.7`: Balanced (default)
- `--temperature 1.0`: Creative, diverse answers

### Pagination
```bash
# First page
snaprag cast recent 374606 --limit 20

# Using activity command with offset
snaprag activity 374606 --limit 20 --offset 20
```

---

## 🔧 Maintenance

### Check Embedding Status
```bash
snaprag embeddings stats
```

### Regenerate Embeddings
```bash
# If embeddings are outdated or corrupted
snaprag embeddings backfill-casts --limit 1000
```

---

## 💡 Tips

1. **Run embeddings backfill** after syncing new cast data
2. **Use --verbose** to understand RAG sources
3. **Adjust --threshold** if results are too broad/narrow
4. **Thread tracking** works best with synchronized parent casts

---

## 📈 Performance

- **Embedding generation**: ~100 casts/second (depends on provider)
- **Semantic search**: Sub-second for millions of casts
- **Thread traversal**: Optimized with indexed queries
- **RAG Q&A**: ~2-5 seconds (embedding + search + LLM)

---

## 🐛 Troubleshooting

**No cast embeddings found:**
```bash
snaprag embeddings backfill-casts
```

**Cast not found:**
- Verify hash is correct (hex format)
- Check if block containing cast was synced

**Low quality search results:**
- Lower --threshold
- Sync more cast data
- Regenerate embeddings

---

## 🔮 Coming Soon

- Aggregate cast metrics (reply count, reaction count)
- Multi-modal search (images in casts)
- Cast clustering by topic
- Real-time cast stream with filters

