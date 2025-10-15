# SnapRAG Query Features Roadmap

## 📊 Current Features (Implemented)
- ✅ User profile RAG query (semantic search + LLM)
- ✅ Profile search (semantic/keyword/hybrid)
- ✅ User activity timeline query
- ✅ Statistics and dashboard
- ✅ Basic list/search commands

---

## 🎯 Recommended Features for Production RAG Service

### 1. **Cast内容查询** (Priority: HIGH)
Cast是Farcaster的核心内容，目前缺失。

#### 1.1 Cast语义搜索
```bash
snaprag cast search "AI agent discussion" --limit 50
snaprag cast search --semantic "machine learning" --time-range 7d
```
**功能：**
- 基于向量的cast内容语义搜索
- 支持时间范围过滤
- 支持话题/标签过滤
- 排序：相关性、时间、互动量

#### 1.2 Cast RAG问答
```bash
snaprag rag query-casts "What are people saying about AI agents?"
snaprag rag cast-trends "Summarize discussions about Farcaster frames"
```
**功能：**
- 基于cast内容的问答
- 多cast的信息聚合和摘要
- 话题趋势分析

#### 1.3 Cast线程追踪
```bash
snaprag cast thread <cast_hash> --depth 3
snaprag cast conversation <cast_hash> --format tree
```
**功能：**
- 显示完整对话线程
- 父子关系可视化
- 对话上下文提取

---

### 2. **社交关系分析** (Priority: HIGH)

#### 2.1 关系图谱查询
```bash
snaprag social graph <fid> --depth 2 --min-mutual 5
snaprag social common-followers <fid1> <fid2>
snaprag social network <fid> --visualize
```
**功能：**
- N度关系查询
- 共同关注者分析
- 社交网络可视化
- 关系强度评分

#### 2.2 社区发现
```bash
snaprag social communities --algorithm louvain --min-size 10
snaprag social cluster <fid> --show-members
```
**功能：**
- 社区/圈子检测
- 影响力中心识别
- 用户聚类分析

#### 2.3 影响力分析
```bash
snaprag social influence <fid> --metrics pagerank,betweenness
snaprag social influencers --topic "defi" --limit 50
```
**功能：**
- PageRank/影响力评分
- 特定话题的KOL识别
- 影响力传播路径

---

### 3. **推荐系统** (Priority: MEDIUM)

#### 3.1 用户推荐
```bash
snaprag recommend users <fid> --reason --limit 20
snaprag recommend similar-profiles <fid> --by interests
```
**功能：**
- 基于社交关系的推荐
- 基于兴趣/行为的相似用户
- 推荐理由解释

#### 3.2 内容推荐
```bash
snaprag recommend casts <fid> --personalized
snaprag recommend feed <fid> --diversify
```
**功能：**
- 个性化feed生成
- 内容多样性保证
- 时效性+相关性平衡

#### 3.3 话题推荐
```bash
snaprag recommend topics <fid>
snaprag recommend channels <fid>
```

---

### 4. **趋势与分析** (Priority: MEDIUM)

#### 4.1 实时趋势
```bash
snaprag trends hot --time-window 24h
snaprag trends topics --rising --limit 10
snaprag trends hashtags --period week
```
**功能：**
- 热门话题检测
- 上升趋势识别
- 话题生命周期分析

#### 4.2 用户活跃度分析
```bash
snaprag analytics user <fid> --time-series --metric engagement
snaprag analytics user <fid> --growth --compare-period
```
**功能：**
- 用户活跃度时间序列
- 增长曲线分析
- 互动质量评估

#### 4.3 内容分析
```bash
snaprag analytics content <fid> --topic-distribution
snaprag analytics sentiment <fid> --time-range 30d
```
**功能：**
- 内容话题分布
- 情感分析趋势
- 互动模式识别

---

### 5. **高级过滤与聚合** (Priority: MEDIUM)

#### 5.1 复合查询
```bash
snaprag query complex \
  --users "bio:contains(developer) AND location:SF" \
  --activity "cast_count > 100 AND reaction_count > 500" \
  --time-range 90d
```
**功能：**
- SQL-like复杂条件
- 多维度联合过滤
- 子查询支持

#### 5.2 聚合分析
```bash
snaprag aggregate users \
  --group-by location \
  --metrics "count,avg(followers),sum(casts)" \
  --having "count > 10"
```
**功能：**
- GROUP BY聚合
- 统计指标计算
- HAVING过滤

#### 5.3 批量查询
```bash
snaprag batch query --file queries.json --output results.jsonl
snaprag batch export --fids fid_list.txt --include-activities
```

---

### 6. **时间序列查询** (Priority: LOW)

```bash
snaprag timeseries user <fid> --metric followers --window 7d
snaprag timeseries compare <fid1> <fid2> --metric engagement
```
**功能：**
- 指标时间序列查询
- 多用户对比分析
- 异常检测

---

### 7. **关系型查询增强** (Priority: MEDIUM)

#### 7.1 多跳查询
```bash
snaprag query path <from_fid> <to_fid> --max-hops 5
snaprag query reach <fid> --target-size 1000 --max-hops 3
```
**功能：**
- 最短路径查询
- 可达性分析
- 扩散范围计算

#### 7.2 子图查询
```bash
snaprag query subgraph --fids <fid_list> --include-edges
snaprag query ego-network <fid> --radius 2 --min-weight 0.5
```

---

## 🏗️ 实现优先级建议

### Phase 1: Core Content (MVP+)
1. ✅ Cast语义搜索与向量化
2. ✅ Cast RAG问答
3. ✅ 基础关系图谱查询

### Phase 2: Social Intelligence
1. 🔲 社区发现
2. 🔲 影响力分析
3. 🔲 用户推荐系统

### Phase 3: Analytics & Insights
1. 🔲 趋势分析
2. 🔲 时间序列查询
3. 🔲 内容分析

### Phase 4: Advanced Features
1. 🔲 复杂查询引擎
2. 🔲 实时流处理
3. 🔲 多模态搜索（图片、视频）

---

## 🔧 技术实现建议

### 数据层增强
```sql
-- 1. Cast向量表
CREATE TABLE cast_embeddings (
  cast_hash bytea PRIMARY KEY,
  embedding vector(768),
  content_text text,
  created_at timestamp
);

-- 2. 关系权重表
CREATE TABLE social_edges (
  from_fid bigint,
  to_fid bigint,
  weight float,
  last_interaction timestamp,
  PRIMARY KEY (from_fid, to_fid)
);

-- 3. 话题表
CREATE TABLE topics (
  topic_id serial PRIMARY KEY,
  name text,
  embedding vector(768),
  cast_count int,
  last_active timestamp
);

-- 4. 时间序列表
CREATE TABLE user_metrics_daily (
  fid bigint,
  date date,
  followers_count int,
  cast_count int,
  engagement_score float,
  PRIMARY KEY (fid, date)
);
```

### API设计
```rust
// Cast搜索
pub async fn search_casts(
    &self,
    query: &str,
    filters: CastFilters,
    limit: usize
) -> Result<Vec<CastSearchResult>>;

// 关系查询
pub async fn get_social_graph(
    &self,
    fid: i64,
    depth: usize,
    filters: GraphFilters
) -> Result<SocialGraph>;

// 推荐
pub async fn recommend_users(
    &self,
    fid: i64,
    strategy: RecommendStrategy,
    limit: usize
) -> Result<Vec<Recommendation>>;
```

---

## 📈 性能考虑

1. **向量索引优化**
   - IVFFlat/HNSW for cast embeddings
   - 分区策略（按时间）

2. **缓存策略**
   - Redis for hot queries
   - Materialized views for aggregations

3. **查询优化**
   - Query planning for complex filters
   - Parallel execution for batch queries

---

## 🎨 用户体验

### Web UI (未来)
- 可视化社交网络图
- 交互式趋势dashboard
- 实时搜索建议

### API接口
- RESTful API
- GraphQL支持
- WebSocket for real-time

---

## 📝 总结

**立即实现（本周）：**
1. Cast内容向量化和语义搜索
2. Cast RAG问答
3. 基础社交关系查询

**短期实现（本月）：**
1. 用户推荐系统
2. 趋势分析基础功能
3. 复杂过滤查询

**长期规划（季度）：**
1. 实时流处理
2. 高级分析功能
3. 多模态支持

