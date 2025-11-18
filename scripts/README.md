# SnapRAG 数据库设置脚本

本目录包含用于设置 SnapRAG PostgreSQL 数据库的脚本。

## 📁 文件说明

### `setup_database.sql`
本地 PostgreSQL 数据库设置脚本。

**用途**: 本地开发环境或自托管服务器

**使用方法**:
```bash
sudo -u postgres psql -f setup_database.sql
```

**功能**:
- 创建 `snaprag` 用户
- 创建 `snaprag` 数据库
- 启用必需的扩展 (vector, pg_trgm, uuid-ossp)
- 配置用户权限

---

### `setup_database_aws.sql`
AWS RDS PostgreSQL 数据库设置脚本。

**用途**: AWS RDS 或其他云托管 PostgreSQL

**使用方法**:
```bash
psql -h your-rds-endpoint.region.rds.amazonaws.com -U postgres -f setup_database_aws.sql
```

**功能**:
- 创建 `snaprag` 用户和数据库
- 启用 RDS 兼容的扩展
- 配置适合云环境的权限
- 包含 RDS 特定的配置和故障排除

---

### `setup_guide.sh`
交互式设置指南脚本。

**用途**: 提供分步设置说明和命令

**使用方法**:
```bash
./setup_guide.sh
```

**功能**:
- 检测操作系统
- 提供平台特定的安装命令
- 显示设置步骤
- 包含验证和故障排除命令

---

## 🚀 快速开始

### 本地设置

1. 安装 PostgreSQL 和 pgvector
2. 运行设置脚本：
   ```bash
   sudo -u postgres psql -f setup_database.sql
   ```
3. 更新 `../config.toml` 中的数据库连接信息
4. 初始化数据库：
   ```bash
   snaprag init --force
   ```

### AWS RDS 设置

1. 创建 RDS PostgreSQL 15+ 实例
2. 配置安全组允许端口 5432
3. 运行设置脚本：
   ```bash
   psql -h your-rds.region.rds.amazonaws.com -U postgres -f setup_database_aws.sql
   ```
4. 更新 `../config.toml` 中的 RDS 端点
5. 初始化数据库：
   ```bash
   snaprag init --force
   ```

## ⚠️ 重要提示

1. **修改密码**: 所有脚本中的默认密码必须修改
2. **权限要求**: 需要 PostgreSQL 超级用户或管理员权限
3. **pgvector 扩展**: 必须安装才能运行 SnapRAG
4. **备份数据**: 在生产环境运行前务必备份

## 📖 详细文档

查看完整的数据库设置指南：[../DATABASE_SETUP.md](../DATABASE_SETUP.md)

## 🔧 故障排除

### 扩展未找到
```bash
git clone https://github.com/pgvector/pgvector.git
cd pgvector
make && sudo make install
```

### 连接被拒绝
```bash
# 检查 PostgreSQL 状态
systemctl status postgresql

# 启动服务
sudo systemctl start postgresql
```

### 权限被拒绝
```sql
-- 以超级用户身份运行
GRANT ALL ON ALL TABLES IN SCHEMA public TO snaprag;
```

---

有问题？查看 [DATABASE_SETUP.md](../DATABASE_SETUP.md) 获取完整的故障排除指南。


