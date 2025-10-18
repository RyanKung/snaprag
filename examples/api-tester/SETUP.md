# 🚀 API Tester 快速设置指南

## 一键启动脚本

如果遇到 `trunk` 命令问题，可以使用以下替代方法：

### 方法 1: 使用 Makefile

```bash
cd examples/api-tester
make dev
```

### 方法 2: 直接使用 trunk

```bash
cd examples/api-tester
trunk serve --port 8080 --address 127.0.0.1
```

然后访问: http://127.0.0.1:8080

### 方法 3: 手动构建并用本地 HTTP 服务器

```bash
cd examples/api-tester

# 构建
trunk build

# 使用 Python 启动简单 HTTP 服务器
cd dist
python3 -m http.server 8080
```

然后访问: http://127.0.0.1:8080

## 前置要求

1. **安装 Trunk**
   ```bash
   cargo install trunk --locked
   ```

2. **添加 WASM 目标**
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. **安装 MetaMask**
   - 浏览器扩展: https://metamask.io/
   - 安装后刷新页面

## 启动 API 服务器

在另一个终端窗口：

```bash
cd /Users/ryan/Dev/farcaster/snaprag
cargo run -- serve api --host 127.0.0.1 --port 3000
```

## 验证安装

```bash
# 检查 Trunk
trunk --version
# 应该显示: trunk 0.x.x

# 检查 WASM 目标
rustup target list | grep wasm32-unknown-unknown
# 应该显示: wasm32-unknown-unknown (installed)

# 测试编译
cd examples/api-tester
cargo build --target wasm32-unknown-unknown
# 应该成功编译
```

## 故障排除

### 问题: "trunk: command not found"

```bash
cargo install trunk --locked
export PATH="$HOME/.cargo/bin:$PATH"
```

### 问题: "target 'wasm32-unknown-unknown' not found"

```bash
rustup target add wasm32-unknown-unknown
```

### 问题: "MetaMask is not installed"

1. 安装 MetaMask 浏览器扩展
2. 刷新页面
3. 点击 "Connect MetaMask"

### 问题: "Failed to fetch"

- 确保 API 服务器正在运行
- 检查 API Base URL 是否为 `http://127.0.0.1:3000`
- 查看浏览器控制台的 CORS 错误

### 问题: Trunk 构建失败

如果 `trunk build` 失败，尝试：

```bash
# 清理并重建
cargo clean
trunk clean
trunk build
```

## 浏览器兼容性

✅ **推荐**: Chrome/Edge  
✅ Firefox  
✅ Safari  
⚠️ 需要 MetaMask 扩展

## 开发技巧

### 实时重载

```bash
trunk serve --open
# 修改代码会自动重新编译和刷新浏览器
```

### 查看编译警告

```bash
cargo clippy --target wasm32-unknown-unknown
```

### 优化构建

```bash
# 生产构建（更小的 WASM 文件）
trunk build --release
```

### 调试

1. 打开浏览器开发者工具 (F12)
2. 查看 Console 标签的日志
3. 查看 Network 标签的请求/响应

## 目录结构

```
examples/api-tester/
├── dist/              # 构建输出（trunk build）
├── js/                # JavaScript 文件
│   └── wallet.js      # MetaMask 集成
├── src/               # Rust 源码
│   ├── main.rs        # 主应用
│   ├── api.rs         # API 逻辑
│   └── wallet.rs      # Wallet 绑定
├── index.html         # HTML 模板
├── Cargo.toml         # Rust 依赖
├── Trunk.toml         # Trunk 配置
└── Makefile           # 快捷命令
```

## 下一步

1. ✅ 启动 API Tester
2. ✅ 连接 MetaMask
3. ✅ 测试免费端点
4. 🚧 实现 x402 支付流程
5. 🚧 添加请求历史记录
6. 🚧 导出为 cURL

## 参考资料

- [Yew 文档](https://yew.rs/)
- [Trunk 文档](https://trunkrs.dev/)
- [MetaMask 文档](https://docs.metamask.io/)
- [SnapRAG API 文档](../../README.md)

---

**遇到问题？** 查看浏览器控制台或 terminal 输出获取详细错误信息。

