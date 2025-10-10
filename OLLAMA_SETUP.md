# Ollama Setup Guide for SnapRAG

## 问题原因

你遇到 404 错误是因为：
1. **配置**: `model = "text-embedding-ada-002"` (OpenAI 模型名)
2. **Endpoint**: `http://192.168.1.192` (Ollama 服务器)
3. **冲突**: 代码将 OpenAI 模型名识别为 OpenAI provider，但调用的是 Ollama 端点

## ✅ 已修复

代码现在根据 `llm_key` 判断 provider：
- `llm_key = "ollama"` → 使用 Ollama API
- `llm_endpoint` 包含 `api.openai.com` → 使用 OpenAI API

## 🚀 Ollama 配置步骤

### 1. 确认 Ollama 运行中

```bash
# 检查 Ollama 是否在运行
curl http://192.168.1.192:11434/api/tags

# 如果 Ollama 在本地
curl http://localhost:11434/api/tags
```

### 2. 拉取 Embedding 模型

```bash
# 推荐模型：nomic-embed-text (768 维)
ollama pull nomic-embed-text

# 或其他模型：
ollama pull mxbai-embed-large  # 1024 维
ollama pull all-minilm         # 384 维
```

### 3. 更新配置文件

编辑 `config.local.toml`:

```toml
[embeddings]
dimension = 768                # 必须匹配模型维度
model = "nomic-embed-text"     # Ollama 模型名

[llm]
llm_endpoint = "http://192.168.1.192:11434"  # Ollama 端点 + 端口
llm_key = "ollama"             # 标识使用 Ollama
```

**重要**: 根据你的实际情况调整：
- 如果 Ollama 在本地：`http://localhost:11434`
- 如果在其他服务器：`http://IP:11434`
- 默认端口：11434

### 4. 更新数据库向量维度

```bash
# 运行更新脚本（会清空现有 embeddings）
./run_update_dim.sh

# 或手动运行 SQL
psql -U snaprag -d snaprag -h localhost -f update_vector_dim.sql
```

### 5. 测试 Embedding 生成

```bash
# 测试单个文本
cargo run -- embeddings test "Hello, I am a developer"

# 应该看到：
# ✅ Generated embedding in ...
# 📊 Embedding Details:
#   - Dimension: 768
#   - Model: nomic-embed-text
#   - Provider: Ollama
```

### 6. 生成所有 Embeddings

```bash
# 为所有 profile 生成 embeddings
cargo run -- embeddings backfill --force
```

## 🔍 不同模型的维度对照

| 模型 | Provider | 维度 | 说明 |
|------|----------|------|------|
| `text-embedding-ada-002` | OpenAI | 1536 | 需要 API key，付费 |
| `text-embedding-3-small` | OpenAI | 1536 | 更新版本，付费 |
| `nomic-embed-text` | Ollama | 768 | 免费，本地运行 |
| `mxbai-embed-large` | Ollama | 1024 | 免费，本地运行 |
| `all-minilm` | Ollama | 384 | 免费，轻量级 |

## 🧪 验证配置

```bash
# 1. 检查配置
cargo run -- config

# 应该显示：
# 🤖 LLM:
#   Endpoint: http://192.168.1.192:11434
#   Key: ollama

# 2. 测试连接
curl http://192.168.1.192:11434/api/embeddings -d '{
  "model": "nomic-embed-text",
  "prompt": "test"
}'

# 3. 测试 SnapRAG embedding
cargo run -- embeddings test "test message"
```

## 🔧 Troubleshooting

### Error: Connection refused
```bash
# Ollama 可能没在运行
# 在 Ollama 服务器上启动：
ollama serve

# 或检查端口
netstat -an | grep 11434
```

### Error: Model not found
```bash
# 拉取模型
ollama pull nomic-embed-text

# 列出已有模型
ollama list
```

### Error: Dimension mismatch
```bash
# 确保 config.toml 中的 dimension 和模型匹配
# nomic-embed-text = 768
# mxbai-embed-large = 1024
# all-minilm = 384

# 运行数据库更新
./run_update_dim.sh
```

## 📝 完整示例配置

### 使用 Ollama (本地)
```toml
[embeddings]
dimension = 768
model = "nomic-embed-text"

[llm]
llm_endpoint = "http://localhost:11434"
llm_key = "ollama"
```

### 使用 OpenAI (云端)
```toml
[embeddings]
dimension = 1536
model = "text-embedding-ada-002"

[llm]
llm_endpoint = "https://api.openai.com/v1"
llm_key = "sk-your-api-key-here"
```

## 🎯 快速测试流程

```bash
# 1. 测试 Ollama 连接
curl http://192.168.1.192:11434/api/tags

# 2. 测试 embedding
cargo run -- embeddings test "test"

# 3. 查看 embedding 统计
cargo run -- embeddings stats

# 4. 如果一切正常，回填所有数据
cargo run -- embeddings backfill --force
```

