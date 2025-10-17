# SnapRAG Library API Reference

## 📚 公共API概览

SnapRAG提供了完整的公共API，可以作为Rust库集成到你的项目中。

## 🎯 核心类型导出

### 主要客户端

```rust
pub struct SnapRag {
    // High-level client for all SnapRAG operations
}

pub struct AppConfig {
    // Configuration loaded from config.toml
}
```

### 数据库

```rust
pub struct Database {
    // PostgreSQL database connection pool
}
```

### 错误处理

```rust
pub enum SnapRagError {
    Database(sqlx::Error),
    LlmError(String),
    Io(std::io::Error),
    Custom(String),
    // ...
}

pub type Result<T> = std::result::Result<T, SnapRagError>;
```

## 🔧 SnapRag 方法

### 初始化

```rust
impl SnapRag {
    /// 创建新实例
    pub async fn new(config: &AppConfig) -> Result<Self>;
    
    /// 初始化数据库schema
    pub async fn init_database(&self) -> Result<()>;
    
    /// 获取数据库实例（直接访问）
    pub fn database(&self) -> &Arc<Database>;
}
```

### 数据同步

```rust
impl SnapRag {
    /// 开始完整同步（historical + realtime）
    pub async fn start_sync(&mut self) -> Result<()>;
    
    /// 同步指定区块范围
    pub async fn start_sync_with_range(&mut self, from_block: u64, to_block: u64) -> Result<()>;
    
    /// 停止同步
    pub async fn stop_sync(&self, force: bool) -> Result<()>;
    
    /// 覆盖同步配置
    pub fn override_sync_config(
        &mut self,
        shard_ids: Vec<u32>,
        batch_size: Option<u32>,
        interval_ms: Option<u64>,
    ) -> Result<()>;
    
    /// 获取同步状态
    pub fn get_sync_status(&self) -> Result<Option<SyncLockFile>>;
}
```

### 数据查询

```rust
impl SnapRag {
    /// 搜索用户profiles（关键词）
    pub async fn search_profiles(&self, query: &str) -> Result<Vec<UserProfile>>;
    
    /// 获取单个profile
    pub async fn get_profile(&self, fid: i64) -> Result<Option<UserProfile>>;
    
    /// 列出casts
    pub async fn list_casts(&self, limit: Option<i64>) -> Result<Vec<Cast>>;
    
    /// 列出follows
    pub async fn list_follows(
        &self,
        fid: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<Link>>;
    
    /// 获取用户活动时间线
    pub async fn get_user_activity(
        &self,
        fid: i64,
        limit: i64,
        offset: i64,
        activity_type: Option<String>,
    ) -> Result<Vec<UserActivityTimeline>>;
    
    /// 获取统计信息
    pub async fn get_statistics(&self) -> Result<StatisticsResult>;
}
```

### 服务创建

```rust
impl SnapRag {
    /// 创建RAG服务
    pub async fn create_rag_service(&self) -> Result<RagService>;
    
    /// 创建Embedding服务
    pub fn create_embedding_service(&self) -> Result<Arc<EmbeddingService>>;
    
    /// 创建LLM服务
    pub fn create_llm_service(&self) -> Result<Arc<LlmService>>;
}
```

### 语义搜索

```rust
impl SnapRag {
    /// Profile语义搜索
    pub async fn semantic_search_profiles(
        &self,
        query: &str,
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<SearchResult>>;
    
    /// Cast语义搜索
    pub async fn semantic_search_casts(
        &self,
        query: &str,
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<CastSearchResult>>;
    
    /// 获取Cast对话线程
    pub async fn get_cast_thread(
        &self,
        message_hash: Vec<u8>,
        depth: usize,
    ) -> Result<CastThread>;
}
```

### Embeddings Backfill

```rust
impl SnapRag {
    /// Backfill profile embeddings
    pub async fn backfill_profile_embeddings(
        &self,
        limit: Option<usize>,
    ) -> Result<ProfileBackfillStats>;
    
    /// Backfill cast embeddings
    pub async fn backfill_cast_embeddings(
        &self,
        limit: Option<usize>,
    ) -> Result<CastBackfillStats>;
}
```

## 🎨 RAG模块

### RagService

```rust
pub struct RagService {
    // Complete RAG pipeline
}

impl RagService {
    /// 创建新的RAG服务
    pub async fn new(config: &AppConfig) -> Result<Self>;
    
    /// 简单查询
    pub async fn query(&self, query: &str) -> Result<RagResponse>;
    
    /// 高级查询（自定义选项）
    pub async fn query_with_options(&self, query: RagQuery) -> Result<RagResponse>;
}

pub struct RagQuery {
    pub question: String,
    pub retrieval_limit: usize,
    pub retrieval_method: RetrievalMethod,
    pub temperature: f32,
    pub max_tokens: usize,
}

pub struct RagResponse {
    pub answer: String,
    pub sources: Vec<SearchResult>,
    pub method_used: RetrievalMethod,
}

pub enum RetrievalMethod {
    Semantic,   // Vector similarity
    Keyword,    // Text matching
    Hybrid,     // RRF fusion
    Auto,       // Intelligent selection
}
```

### Retriever (Profile)

```rust
pub struct Retriever {
    // Profile retrieval and search
}

impl Retriever {
    pub fn new(database: Arc<Database>, embedding_service: Arc<EmbeddingService>) -> Self;
    
    /// 语义搜索
    pub async fn semantic_search(
        &self,
        query: &str,
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<SearchResult>>;
    
    /// 关键词搜索
    pub async fn keyword_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
    
    /// 混合搜索（RRF）
    pub async fn hybrid_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
    
    /// 自动选择最佳方法
    pub async fn auto_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
}
```

### CastRetriever

```rust
pub struct CastRetriever {
    // Cast retrieval and search
}

impl CastRetriever {
    pub fn new(database: Arc<Database>, embedding_service: Arc<EmbeddingService>) -> Self;
    
    /// 语义搜索casts
    pub async fn semantic_search(
        &self,
        query: &str,
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<CastSearchResult>>;
    
    /// 关键词搜索casts
    pub async fn keyword_search(&self, query: &str, limit: usize) -> Result<Vec<CastSearchResult>>;
    
    /// 混合搜索
    pub async fn hybrid_search(&self, query: &str, limit: usize) -> Result<Vec<CastSearchResult>>;
    
    /// 按FID搜索
    pub async fn search_by_fid(&self, fid: i64, limit: usize) -> Result<Vec<Cast>>;
    
    /// 获取Thread
    pub async fn get_thread(&self, message_hash: Vec<u8>, depth: usize) -> Result<CastThread>;
    
    /// 最近的casts
    pub async fn search_recent(&self, limit: usize, offset: usize) -> Result<Vec<Cast>>;
    
    /// 时间范围过滤
    pub async fn search_by_time_range(
        &self,
        start: i64,
        end: i64,
        limit: usize,
    ) -> Result<Vec<Cast>>;
}
```

### Context Assemblers

```rust
pub struct ContextAssembler {
    // Profile context assembly
}

impl ContextAssembler {
    pub fn new(max_context_length: usize) -> Self;
    pub fn assemble(&self, results: &[SearchResult]) -> String;
    pub fn create_summary(&self, results: &[SearchResult]) -> String;
}

pub struct CastContextAssembler {
    // Cast context assembly
}

impl CastContextAssembler {
    pub fn new(max_context_length: usize) -> Self;
    pub fn assemble(&self, results: &[CastSearchResult]) -> String;
    pub async fn assemble_with_authors(
        &self,
        results: &[CastSearchResult],
        database: &Database,
    ) -> Result<String>;
}
```

## 🧠 Embeddings模块

```rust
pub struct EmbeddingService {
    // Embedding generation service
}

impl EmbeddingService {
    pub fn new(config: &AppConfig) -> Result<Self>;
    
    /// 生成单个text的embedding
    pub async fn generate(&self, text: &str) -> Result<Vec<f32>>;
    
    /// 批量生成embeddings
    pub async fn generate_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
}

/// Backfill profile embeddings
pub async fn backfill_profile_embeddings(
    db: Arc<Database>,
    embedding_service: Arc<EmbeddingService>,
) -> Result<BackfillStats>;

/// Backfill cast embeddings（并行处理）
pub async fn backfill_cast_embeddings(
    db: Arc<Database>,
    embedding_service: Arc<EmbeddingService>,
    limit: Option<usize>,
) -> Result<CastBackfillStats>;

pub struct ProfileBackfillStats {
    pub total_profiles: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
}

pub struct CastBackfillStats {
    pub total_casts: usize,
    pub success: usize,
    pub skipped: usize,
    pub failed: usize,
}
```

## 🤖 LLM模块

```rust
pub struct LlmService {
    // LLM query service
}

impl LlmService {
    pub fn new(config: &AppConfig) -> Result<Self>;
    
    /// 简单文本查询
    pub async fn query(
        &self,
        prompt: &str,
        temperature: f32,
        max_tokens: usize,
    ) -> Result<String>;
    
    /// Chat格式查询
    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: usize,
    ) -> Result<String>;
    
    /// 流式响应
    pub async fn query_stream(
        &self,
        prompt: &str,
        temperature: f32,
        max_tokens: usize,
    ) -> Result<StreamingResponse>;
}

pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub struct StreamingResponse {
    // Streaming response wrapper
}
```

## 📊 数据模型

### 核心模型

```rust
pub struct UserProfile {
    pub id: Uuid,
    pub fid: i64,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub pfp_url: Option<String>,
    // ... 更多字段
}

pub struct Cast {
    pub id: Uuid,
    pub fid: i64,
    pub message_hash: Vec<u8>,
    pub text: Option<String>,
    pub timestamp: i64,
    pub parent_hash: Option<Vec<u8>>,
    pub embeds: Option<serde_json::Value>,
    pub mentions: Option<serde_json::Value>,
    // ...
}

pub struct CastSearchResult {
    pub message_hash: Vec<u8>,
    pub fid: i64,
    pub text: String,
    pub timestamp: i64,
    pub similarity: f32,
    pub reply_count: i64,        // ⭐ Engagement metrics
    pub reaction_count: i64,     // ⭐ Engagement metrics
    // ...
}

pub struct Link {
    pub id: Uuid,
    pub fid: i64,
    pub target_fid: i64,
    pub link_type: String,
    pub timestamp: i64,
    // ...
}

pub struct UserActivityTimeline {
    pub id: Uuid,
    pub fid: i64,
    pub activity_type: String,
    pub message_hash: Option<Vec<u8>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub shard_id: Option<i32>,
    pub block_height: Option<i64>,
    // ...
}

pub struct CastThread {
    pub root: Option<Cast>,
    pub parents: Vec<Cast>,
    pub children: Vec<Cast>,
}
```

### 查询结构

```rust
pub struct UserProfileQuery {
    pub fid: Option<i64>,
    pub username: Option<String>,
    pub bio: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<ProfileSortBy>,
    pub sort_order: Option<SortOrder>,
    // ...
}

pub struct CastQuery {
    pub fid: Option<i64>,
    pub text_search: Option<String>,
    pub parent_hash: Option<Vec<u8>>,
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<CastSortBy>,
    // ...
}
```

## 🎯 使用模式

### 模式1: 高级API（推荐）

使用`SnapRag`结构体的便捷方法：

```rust
use snaprag::{SnapRag, AppConfig};

let snaprag = SnapRag::new(&config).await?;

// 一行搞定
let results = snaprag.semantic_search_profiles("crypto", 10, None).await?;
```

### 模式2: 中级API

直接使用RAG组件：

```rust
use snaprag::{RagService, RetrievalMethod};

let rag = snaprag.create_rag_service().await?;
let response = rag.query("Find developers").await?;
```

### 模式3: 低级API

手动组装pipeline：

```rust
use std::sync::Arc;
use snaprag::{Database, EmbeddingService, Retriever};

let db = Arc::new(Database::from_config(&config).await?);
let embedding = Arc::new(EmbeddingService::new(&config)?);
let retriever = Retriever::new(db, embedding);
let results = retriever.semantic_search("query", 10, None).await?;
```

## 📝 常用代码片段

### 1. 初始化并查询

```rust
use snaprag::{SnapRag, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let snaprag = SnapRag::new(&config).await?;
    
    let profiles = snaprag.search_profiles("developer").await?;
    println!("Found {} profiles", profiles.len());
    
    Ok(())
}
```

### 2. RAG查询

```rust
let rag = snaprag.create_rag_service().await?;
let response = rag.query("Find AI developers").await?;

println!("Answer: {}", response.answer);
for source in &response.sources {
    println!("  - @{:?}", source.profile.username);
}
```

### 3. Semantic搜索

```rust
let results = snaprag.semantic_search_casts(
    "discussions about frames",
    10,
    Some(0.7), // similarity threshold
).await?;

for cast in results {
    println!("{} ({}% match, {} replies)",
        cast.text,
        (cast.similarity * 100.0) as i32,
        cast.reply_count
    );
}
```

### 4. Cast Thread

```rust
let hash = hex::decode("your_hash")?;
let thread = snaprag.get_cast_thread(hash, 5).await?;

println!("Parents: {}", thread.parents.len());
println!("Children: {}", thread.children.len());
```

### 5. Embeddings生成

```rust
// Profile embeddings
let stats = snaprag.backfill_profile_embeddings(Some(1000)).await?;
println!("{} updated, {} skipped", stats.updated, stats.skipped);

// Cast embeddings（5x并行加速）
let stats = snaprag.backfill_cast_embeddings(Some(1000)).await?;
println!("{} success, {} failed", stats.success, stats.failed);
```

### 6. 用户活动

```rust
let activities = snaprag.get_user_activity(
    3,                          // FID
    100,                        // limit
    0,                          // offset
    Some("cast_add".to_string()) // 只看casts
).await?;

for activity in activities {
    println!("{}: block {}", 
        activity.activity_type,
        activity.block_height.unwrap_or(0)
    );
}
```

## 🔍 可用的查询类型

### Profile Queries
- Semantic search (vector similarity)
- Keyword search (text matching)
- By FID
- By username
- With filters (location, twitter, github, etc.)

### Cast Queries
- Semantic search (with engagement metrics)
- Keyword search
- Hybrid search (RRF fusion)
- By FID
- By parent_hash (replies)
- Time range filtering
- Thread retrieval

### Activity Queries
- By FID
- By activity type
- Time range
- Pagination

## 📦 完整导出列表

```rust
// 主要类型
pub use snaprag::{
    SnapRag,
    AppConfig,
    Database,
    SyncService,
    Result,
    SnapRagError,
};

// 数据模型
pub use snaprag::{
    UserProfile,
    Cast,
    Link,
    UserActivityTimeline,
    CastSearchResult,
    StatisticsResult,
    CastThread,
};

// RAG
pub use snaprag::{
    RagService,
    RagQuery,
    RagResponse,
    Retriever,
    CastRetriever,
    ContextAssembler,
    CastContextAssembler,
    SearchResult,
    RetrievalMethod,
};

// Embeddings
pub use snaprag::{
    EmbeddingService,
    backfill_profile_embeddings,
    backfill_cast_embeddings,
    ProfileBackfillStats,
    CastBackfillStats,
};

// LLM
pub use snaprag::{
    LlmService,
    ChatMessage,
    StreamingResponse,
};

// 工具函数
pub use snaprag::{
    farcaster_to_unix_timestamp,
    unix_to_farcaster_timestamp,
    FARCASTER_EPOCH,
};
```

## 🎓 最佳实践

1. **复用SnapRag实例**: 创建一次，多处使用
2. **使用Arc共享**: Database/Service都使用Arc
3. **错误处理**: 总是处理Result
4. **并发安全**: 所有方法都是async且线程安全
5. **资源管理**: 连接池自动管理，无需手动清理

## 📚 更多资源

- [LIBRARY_USAGE.md](./LIBRARY_USAGE.md) - 详细使用指南
- [RAG_ARCHITECTURE.md](./RAG_ARCHITECTURE.md) - RAG架构说明
- [examples/](./examples/) - 可运行示例
- API文档: `cargo doc --lib --open`

---

**SnapRAG提供了完整、类型安全、高性能的Rust库API！** 🚀

