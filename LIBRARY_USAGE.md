# SnapRAG 库使用指南

SnapRAG不仅可以作为CLI工具使用，还可以作为Rust库集成到你的项目中。

## 📦 添加依赖

### Cargo.toml

```toml
[dependencies]
snaprag = { path = "../snaprag" }
# 或从crates.io (未发布时使用path)
# snaprag = "0.1.0"

# 必需的运行时依赖
tokio = { version = "1.0", features = ["full"] }
```

## 🚀 快速开始

### 1. 基础使用

```rust
use snaprag::{SnapRag, AppConfig, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // 加载配置
    let config = AppConfig::load()?;
    
    // 创建SnapRAG实例
    let snaprag = SnapRag::new(&config).await?;
    
    // 初始化数据库
    snaprag.init_database().await?;
    
    println!("SnapRAG初始化成功!");
    
    Ok(())
}
```

### 2. 数据同步

```rust
use snaprag::{SnapRag, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let mut snaprag = SnapRag::new(&config).await?;
    
    // 方式1: 完整同步（从genesis开始）
    snaprag.start_sync().await?;
    
    // 方式2: 指定区块范围
    snaprag.start_sync_with_range(0, 100000).await?;
    
    // 方式3: 自定义配置
    snaprag.override_sync_config(
        vec![1, 2],      // 只同步shard 1和2
        Some(50),        // 批处理大小
        Some(1000),      // 间隔(ms)
    )?;
    snaprag.start_sync().await?;
    
    // 停止同步
    snaprag.stop_sync(false).await?;
    
    Ok(())
}
```

### 3. 查询用户数据

```rust
use snaprag::{SnapRag, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    
    // 获取单个用户profile
    if let Some(profile) = snaprag.get_profile(3).await? {
        println!("Username: {:?}", profile.username);
        println!("Bio: {:?}", profile.bio);
    }
    
    // 搜索用户
    let developers = snaprag.search_profiles("developer").await?;
    println!("Found {} developers", developers.len());
    
    // 列出casts
    let recent_casts = snaprag.list_casts(Some(20)).await?;
    for cast in recent_casts {
        println!("Cast: {:?}", cast.text);
    }
    
    // 获取用户活动
    let activities = snaprag.get_user_activity(
        3,              // FID
        50,             // limit
        0,              // offset  
        None,           // 所有类型
    ).await?;
    
    for activity in activities {
        println!("Activity: {} at block {}", 
            activity.activity_type, 
            activity.block_height.unwrap_or(0)
        );
    }
    
    // 获取follows
    let follows = snaprag.list_follows(Some(3), Some(100)).await?;
    println!("FID 3 follows {} users", follows.len());
    
    // 获取统计信息
    let stats = snaprag.get_statistics().await?;
    println!("Total profiles: {}", stats.total_fids);
    println!("Total casts: {}", stats.total_casts);
    println!("Total activities: {}", stats.total_activities);
    
    Ok(())
}
```

## 🔍 RAG功能

### 1. 使用内置RAG Service

```rust
use snaprag::{SnapRag, AppConfig, RagQuery, RetrievalMethod};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    
    // 创建RAG service
    let rag_service = snaprag.create_rag_service().await?;
    
    // 方式1: 简单查询
    let response = rag_service.query(
        "Find developers interested in crypto"
    ).await?;
    
    println!("Answer: {}", response.answer);
    println!("Sources: {} profiles", response.sources.len());
    
    // 方式2: 高级查询
    let query = RagQuery {
        question: "Who are the most active builders?".to_string(),
        retrieval_limit: 20,
        retrieval_method: RetrievalMethod::Hybrid,
        temperature: 0.7,
        max_tokens: 1000,
    };
    
    let response = rag_service.query_with_options(query).await?;
    println!("Answer: {}", response.answer);
    
    Ok(())
}
```

### 2. 语义搜索（不用LLM）

```rust
use snaprag::{SnapRag, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    
    // Profile语义搜索
    let profiles = snaprag.semantic_search_profiles(
        "developers building on Farcaster",
        10,              // limit
        Some(0.7),       // similarity threshold
    ).await?;
    
    for result in profiles {
        println!("Profile: @{:?} (score: {:.2})", 
            result.profile.username, 
            result.score
        );
    }
    
    // Cast语义搜索
    let casts = snaprag.semantic_search_casts(
        "discussions about frames",
        15,
        Some(0.7),
    ).await?;
    
    for cast in casts {
        println!("Cast: {} (similarity: {:.1}%, {} replies, {} reactions)",
            cast.text,
            cast.similarity * 100.0,
            cast.reply_count,
            cast.reaction_count
        );
    }
    
    Ok(())
}
```

### 3. Cast Thread检索

```rust
use snaprag::{SnapRag, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    
    // 获取完整对话线程
    let message_hash = hex::decode("your_cast_hash_here")?;
    let thread = snaprag.get_cast_thread(message_hash, 10).await?;
    
    println!("Parent chain: {} casts", thread.parents.len());
    if let Some(root) = &thread.root {
        println!("Root: {:?}", root.text);
    }
    println!("Children: {} replies", thread.children.len());
    
    Ok(())
}
```

## 🎨 Embeddings生成

### 1. Backfill Embeddings

```rust
use snaprag::{SnapRag, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    
    // Profile embeddings
    let profile_stats = snaprag.backfill_profile_embeddings(Some(1000)).await?;
    println!("Profile embeddings: {} updated, {} skipped, {} failed",
        profile_stats.updated,
        profile_stats.skipped,
        profile_stats.failed
    );
    
    // Cast embeddings (并行处理，5x性能)
    let cast_stats = snaprag.backfill_cast_embeddings(Some(1000)).await?;
    println!("Cast embeddings: {} success, {} skipped, {} failed",
        cast_stats.success,
        cast_stats.skipped,
        cast_stats.failed
    );
    
    Ok(())
}
```

### 2. 使用Embedding Service

```rust
use snaprag::{SnapRag, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    
    // 创建embedding service
    let embedding_service = snaprag.create_embedding_service()?;
    
    // 生成单个embedding
    let embedding = embedding_service.generate(
        "This is a test text for embedding"
    ).await?;
    
    println!("Generated embedding with {} dimensions", embedding.len());
    
    Ok(())
}
```

## 🤖 LLM集成

```rust
use snaprag::{SnapRag, AppConfig, ChatMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    
    // 创建LLM service
    let llm_service = snaprag.create_llm_service()?;
    
    // 方式1: 简单文本生成
    let response = llm_service.query(
        "Explain Farcaster in one sentence",
        0.7,        // temperature
        100,        // max_tokens
    ).await?;
    
    println!("LLM: {}", response);
    
    // 方式2: Chat格式
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are a Farcaster expert.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: "What are frames?".to_string(),
        },
    ];
    
    let response = llm_service.chat(messages, 0.7, 200).await?;
    println!("Chat response: {}", response);
    
    Ok(())
}
```

## 🔧 自定义Retrieval

### 使用低级API

```rust
use std::sync::Arc;
use snaprag::{
    AppConfig, Database, EmbeddingService,
    Retriever, CastRetriever, ContextAssembler,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    
    // 创建服务
    let db = Arc::new(Database::from_config(&config).await?);
    let embedding_service = Arc::new(EmbeddingService::new(&config)?);
    
    // 创建retriever
    let retriever = Retriever::new(
        Arc::clone(&db),
        Arc::clone(&embedding_service)
    );
    
    // 语义搜索
    let results = retriever.semantic_search(
        "blockchain developers",
        10,
        Some(0.75),
    ).await?;
    
    // 关键词搜索
    let keyword_results = retriever.keyword_search(
        "Ethereum",
        10
    ).await?;
    
    // 混合搜索（RRF融合）
    let hybrid_results = retriever.hybrid_search(
        "Ethereum developers",
        15
    ).await?;
    
    // 自动选择最佳方法
    let auto_results = retriever.auto_search(
        "Find people interested in AI",
        10
    ).await?;
    
    // 组装上下文
    let context_assembler = ContextAssembler::new(4096);
    let context = context_assembler.assemble(&results);
    
    println!("Context: {}", context);
    
    Ok(())
}
```

## 📊 直接数据库访问

### 使用Database API

```rust
use std::sync::Arc;
use snaprag::{AppConfig, Database, UserProfileQuery, CastQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let db = Arc::new(Database::from_config(&config).await?);
    
    // 复杂profile查询
    let query = UserProfileQuery {
        fid: None,
        username: Some("vitalik".to_string()),
        bio: Some("ethereum".to_string()),
        limit: Some(10),
        offset: None,
        sort_by: Some(snaprag::ProfileSortBy::LastUpdated),
        ..Default::default()
    };
    
    let profiles = db.list_user_profiles(query).await?;
    
    // 复杂cast查询
    let cast_query = CastQuery {
        fid: Some(3),
        text_search: Some("frames".to_string()),
        start_timestamp: Some(1640000000),
        has_embeds: Some(true),
        limit: Some(20),
        ..Default::default()
    };
    
    let casts = db.list_casts(cast_query).await?;
    
    // 获取cast统计
    let cast_hash = hex::decode("some_hash")?;
    let stats = db.get_cast_stats(&cast_hash).await?;
    println!("Replies: {}, Reactions: {}, Unique reactors: {}",
        stats.reply_count,
        stats.reaction_count,
        stats.unique_reactors
    );
    
    Ok(())
}
```

## 🎯 完整示例：构建自定义RAG

```rust
use std::sync::Arc;
use snaprag::{
    AppConfig, Database, EmbeddingService, LlmService,
    CastRetriever, CastContextAssembler,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    
    // 初始化服务
    let db = Arc::new(Database::from_config(&config).await?);
    let embedding_service = Arc::new(EmbeddingService::new(&config)?);
    let llm_service = Arc::new(LlmService::new(&config)?);
    
    // 步骤1: 检索相关casts
    let cast_retriever = CastRetriever::new(
        Arc::clone(&db),
        Arc::clone(&embedding_service)
    );
    
    let query = "What are people saying about Farcaster frames?";
    let casts = cast_retriever.semantic_search(query, 10, Some(0.7)).await?;
    
    println!("Found {} relevant casts", casts.len());
    
    // 步骤2: 组装上下文
    let context_assembler = CastContextAssembler::new(4096);
    let context = context_assembler
        .assemble_with_authors(&casts, Arc::clone(&db))
        .await?;
    
    println!("Context size: {} chars", context.len());
    
    // 步骤3: 构建prompt
    let prompt = format!(
        "Based on the following casts:\n\n{}\n\n\
         Question: {}\n\n\
         Answer based only on the information above:",
        context, query
    );
    
    // 步骤4: 查询LLM
    let answer = llm_service.query(&prompt, 0.7, 500).await?;
    
    println!("\n📝 Answer:\n{}", answer);
    
    // 显示来源
    println!("\n📚 Sources:");
    for (i, cast) in casts.iter().take(5).enumerate() {
        println!("  {}. {} (similarity: {:.1}%, {} replies)",
            i + 1,
            &cast.text[..cast.text.len().min(100)],
            cast.similarity * 100.0,
            cast.reply_count
        );
    }
    
    Ok(())
}
```

## 📚 导出的公共API

### 核心类型

```rust
pub use snaprag::{
    // 主要客户端
    SnapRag,
    AppConfig,
    
    // 数据库
    Database,
    
    // 错误处理
    Result,
    SnapRagError,
    
    // 数据模型
    UserProfile,
    Cast,
    Link,
    UserActivityTimeline,
    CastSearchResult,
    
    // 同步
    SyncService,
    
    // RAG
    RagService,
    RagQuery,
    RagResponse,
    Retriever,
    CastRetriever,
    ContextAssembler,
    CastContextAssembler,
    SearchResult,
    RetrievalMethod,
    
    // Embeddings
    EmbeddingService,
    backfill_profile_embeddings,
    backfill_cast_embeddings,
    ProfileBackfillStats,
    CastBackfillStats,
    
    // LLM
    LlmService,
    ChatMessage,
    StreamingResponse,
};
```

### SnapRag方法

```rust
impl SnapRag {
    // 初始化
    pub async fn new(config: &AppConfig) -> Result<Self>;
    pub async fn init_database(&self) -> Result<()>;
    pub fn database(&self) -> &Arc<Database>;
    
    // 同步
    pub async fn start_sync(&mut self) -> Result<()>;
    pub async fn start_sync_with_range(&mut self, from: u64, to: u64) -> Result<()>;
    pub async fn stop_sync(&self, force: bool) -> Result<()>;
    pub fn override_sync_config(&mut self, shards: Vec<u32>, batch: Option<u32>, interval: Option<u64>) -> Result<()>;
    
    // 查询
    pub async fn search_profiles(&self, query: &str) -> Result<Vec<UserProfile>>;
    pub async fn get_profile(&self, fid: i64) -> Result<Option<UserProfile>>;
    pub async fn list_casts(&self, limit: Option<i64>) -> Result<Vec<Cast>>;
    pub async fn list_follows(&self, fid: Option<i64>, limit: Option<i64>) -> Result<Vec<Link>>;
    pub async fn get_user_activity(&self, fid: i64, limit: i64, offset: i64, activity_type: Option<String>) -> Result<Vec<UserActivityTimeline>>;
    pub async fn get_statistics(&self) -> Result<StatisticsResult>;
    
    // 服务创建
    pub async fn create_rag_service(&self) -> Result<RagService>;
    pub fn create_embedding_service(&self) -> Result<Arc<EmbeddingService>>;
    pub fn create_llm_service(&self) -> Result<Arc<LlmService>>;
    
    // 语义搜索
    pub async fn semantic_search_profiles(&self, query: &str, limit: usize, threshold: Option<f32>) -> Result<Vec<SearchResult>>;
    pub async fn semantic_search_casts(&self, query: &str, limit: usize, threshold: Option<f32>) -> Result<Vec<CastSearchResult>>;
    
    // Thread
    pub async fn get_cast_thread(&self, message_hash: Vec<u8>, depth: usize) -> Result<CastThread>;
    
    // Embeddings backfill
    pub async fn backfill_profile_embeddings(&self, limit: Option<usize>) -> Result<ProfileBackfillStats>;
    pub async fn backfill_cast_embeddings(&self, limit: Option<usize>) -> Result<CastBackfillStats>;
}
```

## 🔨 构建&测试

### 作为库编译

```bash
# 仅编译库
cargo build --lib

# 编译库+文档
cargo doc --lib --open

# 运行库测试
cargo test --lib
```

### 在其他项目中使用

**项目结构**:
```
my-farcaster-app/
├── Cargo.toml
└── src/
    └── main.rs
```

**Cargo.toml**:
```toml
[package]
name = "my-farcaster-app"
version = "0.1.0"
edition = "2021"

[dependencies]
snaprag = { path = "../snaprag" }
tokio = { version = "1.0", features = ["full"] }
```

**src/main.rs**:
```rust
use snaprag::{SnapRag, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    
    // 使用SnapRAG功能...
    let stats = snaprag.get_statistics().await?;
    println!("Total users: {}", stats.total_fids);
    
    Ok(())
}
```

## 📖 API文档

### 生成文档

```bash
# 生成并打开文档
cargo doc --lib --open

# 仅生成文档
cargo doc --lib --no-deps
```

### 在线文档

所有公共API都有完整的Rustdoc注释，包括：
- 函数签名
- 参数说明
- 返回值说明
- 使用示例
- 相关链接

## 🎓 最佳实践

### 1. 错误处理

```rust
use snaprag::{SnapRag, AppConfig, SnapRagError};

#[tokio::main]
async fn main() {
    match run().await {
        Ok(()) => println!("Success!"),
        Err(SnapRagError::Database(e)) => eprintln!("Database error: {}", e),
        Err(SnapRagError::LlmError(e)) => eprintln!("LLM error: {}", e),
        Err(e) => eprintln!("Error: {}", e),
    }
}

async fn run() -> Result<(), SnapRagError> {
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    // ...
    Ok(())
}
```

### 2. 并发处理

```rust
use snaprag::{SnapRag, AppConfig};
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let snaprag = Arc::new(SnapRag::new(&config).await?);
    
    let mut tasks = JoinSet::new();
    
    // 并行查询多个FID
    for fid in vec![1, 2, 3, 4, 5] {
        let snaprag_clone = Arc::clone(&snaprag);
        tasks.spawn(async move {
            snaprag_clone.get_profile(fid).await
        });
    }
    
    // 收集结果
    while let Some(result) = tasks.join_next().await {
        match result? {
            Ok(Some(profile)) => println!("Profile: {:?}", profile.username),
            Ok(None) => println!("Profile not found"),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

### 3. 资源管理

```rust
use snaprag::{SnapRag, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    
    // SnapRag实例可以被多次clone（内部使用Arc）
    let snaprag = SnapRag::new(&config).await?;
    
    // Database连接池会自动管理
    // 不需要手动关闭连接
    
    // 在作用域结束时自动清理
    Ok(())
}
```

## 🚀 性能提示

1. **复用SnapRag实例**: 创建一次，多次使用
2. **使用Arc**: Database和Service都使用Arc，克隆成本低
3. **批量操作**: 使用backfill而非逐个生成embedding
4. **并发查询**: 使用tokio::spawn进行并发
5. **配置连接池**: 根据并发度调整max_connections

## 📝 完整示例项目

查看 `examples/` 目录（如果存在）或参考：
- `src/main.rs` - CLI实现
- `src/tests/rag_integration_test.rs` - RAG使用示例
- `src/cli/handlers.rs` - 各种功能的使用方式

---

**SnapRAG作为库使用时提供了完整、类型安全、高性能的API！** 🎉

