# GitHub Actions Workflows

## Overview

This directory contains CI/CD workflows for the SnapRAG project.

## Workflows

### 1. `ci.yml` - Continuous Integration

**Triggers**: Push to master/main, Pull Requests

**Jobs**:
- **Format Check**: Ensures code is formatted with `cargo fmt`
- **Clippy Lints**: Runs clippy with strict warnings
- **Unit Tests**: Runs on Ubuntu and macOS (matrix)
- **Build Minimal**: Verifies build without optional features
- **Security Audit**: Checks for known vulnerabilities
- **Documentation**: Builds and validates docs

**Dependencies**:
- protobuf-compiler (for gRPC)
- libssl-dev (for HTTPS/TLS)
- pkg-config (for C library linking)

### 2. `pr.yml` - Pull Request Checks

**Triggers**: PR opened/updated

**Jobs**:
- **Quick Validation**: Fast format and lint checks
- **Compilation Check**: Verify code compiles
- **Unit Tests**: Run fast unit tests only
- **API Compatibility**: Check public API documentation

**特点**:
- 快速反馈（~2-5分钟）
- 只运行纯单元测试
- 跳过集成测试（需要数据库）

### 3. `release.yml` - Release Builds

**Triggers**: Version tags (v*.*.*)

**Jobs**:
- **Build Release**: Multi-platform binary builds
  - Linux x86_64
  - macOS ARM64 (Apple Silicon)
  - macOS x86_64 (Intel)
- **Create Release**: Automated GitHub release with binaries

## Test Configuration

### Unit Tests (Fast)

只运行纯单元测试，跳过需要外部依赖的测试：

```bash
cargo test --lib --all-features -- \
  --skip integration \
  --skip database_tests \
  --skip real_data \
  --skip deterministic_blocks \
  --skip grpc_shard
```

### Doc Tests

测试文档中的代码示例：

```bash
cargo test --doc --all-features
```

### Full Tests (Local Only)

完整测试需要数据库和Snapchain连接：

```bash
# 需要初始化数据库
snaprag init --force

# 运行所有测试
cargo test --lib
```

## Caching Strategy

工作流使用多级缓存加速构建：

1. **Cargo Registry**: 下载的crate缓存
2. **Cargo Git**: Git依赖缓存
3. **Build Artifacts**: 编译产物缓存

缓存键基于：
- OS平台
- Rust版本
- Cargo.lock哈希值

## System Dependencies

### Ubuntu
```bash
sudo apt-get install -y libssl-dev pkg-config protobuf-compiler
```

### macOS
```bash
brew install protobuf
```

## Features Configuration

### Default Features
- 基础功能（sync, embeddings, RAG）
- 不包括GPU加速

### Optional Features
- `local-gpu`: 本地GPU加速（需要CUDA或Metal）
- `payment`: X402支付集成

### All Features
```bash
cargo build --all-features
```

### No Default Features (最小构建)
```bash
cargo build --no-default-features
```

## CI/CD Best Practices

### ✅ 已实现

1. **快速反馈**: PR检查在5分钟内完成
2. **多平台测试**: Ubuntu + macOS
3. **缓存优化**: 显著减少构建时间
4. **安全检查**: cargo-audit自动运行
5. **文档验证**: 确保doc build成功
6. **格式一致性**: 强制cargo fmt

### 🔄 可选改进

1. **代码覆盖率**: 使用tarpaulin
2. **性能基准**: 使用criterion
3. **Docker构建**: 容器化测试环境
4. **集成测试**: 使用GitHub Services（PostgreSQL）

## Troubleshooting

### 测试失败

1. **格式检查失败**
   ```bash
   cargo fmt --all
   ```

2. **Clippy警告**
   ```bash
   cargo clippy --all-targets --all-features --fix
   ```

3. **测试失败**
   ```bash
   cargo test --lib -- --nocapture
   ```

### 依赖问题

如果遇到依赖编译错误：

1. 清理缓存
   ```bash
   cargo clean
   ```

2. 更新依赖
   ```bash
   cargo update
   ```

3. 检查Rust版本
   ```bash
   rustc --version  # 应该是stable
   ```

## 本地运行CI检查

在提交前本地运行相同的检查：

```bash
# 格式检查
cargo fmt --all -- --check

# Lint检查
cargo clippy --all-targets --all-features -- -D warnings

# 单元测试
cargo test --lib --all-features -- \
  --skip integration --skip database --skip real_data

# Doc测试
cargo test --doc

# 构建
cargo build --all-features
```

## 维护

### 更新工作流

修改工作流时请测试：

```bash
# 使用act本地测试（需要安装act）
act -l  # 列出所有jobs
act push  # 模拟push事件
```

### 依赖更新

定期更新GitHub Actions版本：
- actions/checkout@v4
- actions/cache@v4
- actions/upload-artifact@v4
- dtolnay/rust-toolchain@stable

---

**维护者**: SnapRAG Team  
**最后更新**: 2025-11-03

