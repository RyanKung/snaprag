# 🚀 SnapRAG 快速开始指南

## ✅ 当前配置状态

- **Embedding 模型**: nomic-embed-text (Ollama, 768维)
- **LLM 端点**: localhost:11434 (通过 SSH 隧道)
- **数据库**: 已更新为 768 维向量
- **状态**: ✅ 就绪

## 📋 完整使用流程

### Step 1: 启动 SSH 隧道（必须）

```bash
# 在新终端运行（保持运行状态）
ssh -L 11434:localhost:11434 ryan@192.168.1.192 -N

# 或使用后台方式
nohup ssh -L 11434:localhost:11434 ryan@192.168.1.192 -N &
```

### Step 2: 同步 Farcaster 数据

```bash
# 同步一小段数据进行测试
cargo run -- sync start --from 5000000 --to 5000100

# 或查看已有数据
cargo run -- list profiles --limit 10
```

### Step 3: 生成 Embeddings

```bash
# 查看需要生成的数量
cargo run -- embeddings stats

# 为所有 profiles 生成 embeddings
cargo run -- embeddings backfill --force

# 或为单个 profile 生成
cargo run -- embeddings generate --fid 12345
```

### Step 4: 使用 RAG 查询

```bash
# 语义搜索（不用 LLM）
cargo run -- rag search "blockchain developers"

# 完整 RAG 查询（使用 LLM 生成答案）
cargo run -- rag query "Who are AI developers in crypto?"

# 高级选项
cargo run -- rag query "Find rust developers" \
  --limit 20 \
  --method hybrid \
  --verbose
```

## 🧪 测试命令

```bash
# 1. 测试 Ollama 连接
curl http://localhost:11434/api/tags

# 2. 测试 embedding 生成
cargo run -- embeddings test "test message"

# 3. 查看配置
cargo run -- config

# 4. 查看统计
cargo run -- embeddings stats
```

## 🔧 常用命令

```bash
# 查看 profiles 数量
cargo run -- stats

# 搜索用户
cargo run -- search "vitalik" --limit 5

# 查看 sync 状态
cargo run -- sync status
```

## ⚠️ 重要提示

1. **SSH 隧道必须保持运行** - 否则无法连接 Ollama
2. **先同步数据** - 需要有 profiles 才能生成 embeddings
3. **生成 embeddings** - RAG 查询需要 embeddings 才能工作
4. **Ollama 模型** - nomic-embed-text 已在服务器上

## 🎯 完整示例流程

```bash
# Terminal 1: 启动隧道
ssh -L 11434:localhost:11434 ryan@192.168.1.192 -N

# Terminal 2: 使用 SnapRAG
cd /Users/ryan/Dev/farcaster/snaprag

# 同步测试数据
cargo run -- sync start --from 5000000 --to 5000010

# 生成 embeddings
cargo run -- embeddings backfill --force

# RAG 查询
cargo run -- rag query "Find developers interested in AI"
```

## 📊 当前架构

```
你的 Mac
  ↓ SSH 隧道 (localhost:11434)
  ↓
192.168.1.192
  ├── Ollama (localhost:11434)
  │   └── nomic-embed-text 模型
  └── Snapchain (3381/3383)

192.168.1.160
  └── PostgreSQL
      └── SnapRAG 数据库 (768维向量)
```

## 🎉 下一步

1. **同步更多数据**: 增加 `--to` 范围
2. **生成所有 embeddings**: 运行 backfill
3. **测试 RAG 查询**: 尝试各种问题
4. **优化性能**: 调整批处理大小

