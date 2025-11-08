# 🐳 SnapRAG Docker Deployment Guide

This guide explains how to deploy SnapRAG using Docker and Docker Compose.

## 📋 Prerequisites

- Docker 20.10+
- Docker Compose 2.0+
- 4GB RAM minimum
- 20GB disk space (for database and embeddings)

## 🚀 Quick Start

### Option 1: Docker Compose (Recommended)

The easiest way to run SnapRAG with all dependencies:

```bash
# 1. Create configuration file
cp config.example.toml config.toml

# 2. Edit config.toml with your settings
# Update database URL to: postgresql://snaprag:snaprag_password@postgres:5432/snaprag
# Update Redis URL to: redis://redis:6379

# 3. Start all services
docker-compose up -d

# 4. Check status
docker-compose ps

# 5. View logs
docker-compose logs -f snaprag
```

Services will be available at:
- **SnapRAG API**: http://localhost:3000
- **PostgreSQL**: localhost:5432
- **Redis**: localhost:6379

### Option 2: Standalone Docker Container

If you have existing PostgreSQL and Redis:

```bash
# Build image
docker build -t snaprag:latest .

# Run container
docker run -d \
  --name snaprag \
  -p 3000:3000 \
  -v $(pwd)/config.toml:/app/config.toml:ro \
  -v $(pwd)/logs:/app/logs \
  snaprag:latest api
```

## 📦 What's Included

### Docker Compose Services

1. **PostgreSQL** (ankane/pgvector)
   - Database with pgvector extension
   - Port: 5432
   - Data persisted in Docker volume

2. **Redis** (redis:7-alpine)
   - Caching layer
   - Port: 6379
   - Data persisted in Docker volume

3. **SnapRAG** (custom build)
   - Main application
   - Port: 3000
   - Logs in `./logs/`

## 🔧 Configuration

### Database Configuration

Update `config.toml`:

```toml
[database]
url = "postgresql://snaprag:snaprag_password@postgres:5432/snaprag"
max_connections = 100
min_connections = 10
```

### Redis Configuration

```toml
[cache]
enabled = true
redis_url = "redis://redis:6379"
```

### Snapchain Configuration

```toml
[sync]
snapchain_http_endpoint = "http://your-snapchain-node:3381"
snapchain_grpc_endpoint = "http://your-snapchain-node:3383"
```

## 🛠️ Using Make Commands

We provide a `Makefile.docker` with convenient commands:

```bash
# Build Docker image
make -f Makefile.docker docker-build

# Start all services
make -f Makefile.docker docker-compose-up

# View logs
make -f Makefile.docker docker-compose-logs

# Stop all services
make -f Makefile.docker docker-compose-down

# Rebuild everything
make -f Makefile.docker docker-compose-rebuild

# Development mode (with source mounting)
make -f Makefile.docker docker-compose-dev

# See all commands
make -f Makefile.docker help
```

## 📊 Database Initialization

After starting services for the first time:

```bash
# Initialize database schema
docker-compose exec snaprag snaprag reset --force
docker-compose exec snaprag snaprag migrate

# Or use the make command
make -f Makefile.docker docker-init-db
```

## 🔍 Common Operations

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f snaprag
docker-compose logs -f postgres
```

### Execute Commands in Container

```bash
# Run snaprag commands
docker-compose exec snaprag snaprag --help
docker-compose exec snaprag snaprag sync status
docker-compose exec snaprag snaprag list fid --limit 10

# Open shell
docker-compose exec snaprag /bin/bash
```

### Restart Services

```bash
# Restart all
docker-compose restart

# Restart specific service
docker-compose restart snaprag
```

### Update Configuration

```bash
# 1. Edit config.toml
vim config.toml

# 2. Restart snaprag service
docker-compose restart snaprag
```

## 🔄 Data Persistence

All data is stored in Docker volumes:

- `postgres_data`: PostgreSQL database
- `redis_data`: Redis cache
- `./logs`: Application logs (bind mount)

### Backup Data

```bash
# Backup PostgreSQL
docker-compose exec -T postgres pg_dump -U snaprag snaprag > backup.sql

# Backup Redis
docker-compose exec redis redis-cli SAVE
docker cp snaprag-redis:/data/dump.rdb redis-backup.rdb
```

### Restore Data

```bash
# Restore PostgreSQL
docker-compose exec -T postgres psql -U snaprag snaprag < backup.sql

# Restore Redis
docker cp redis-backup.rdb snaprag-redis:/data/dump.rdb
docker-compose restart redis
```

## 🐛 Troubleshooting

### Container Won't Start

```bash
# Check logs
docker-compose logs snaprag

# Check health status
docker-compose ps

# Rebuild from scratch
docker-compose down -v
docker-compose build --no-cache
docker-compose up -d
```

### Database Connection Issues

```bash
# Check PostgreSQL is ready
docker-compose exec postgres pg_isready -U snaprag

# Check database exists
docker-compose exec postgres psql -U snaprag -l

# Recreate database
docker-compose down
docker volume rm snaprag_postgres_data
docker-compose up -d
```

### Performance Issues

```bash
# Check resource usage
docker stats

# Increase PostgreSQL resources in docker-compose.yml:
services:
  postgres:
    deploy:
      resources:
        limits:
          memory: 4G
        reservations:
          memory: 2G
```

## 🚢 Production Deployment

### Environment Variables

Set in `docker-compose.yml`:

```yaml
services:
  snaprag:
    environment:
      - RUST_LOG=info
      - RUST_BACKTRACE=1
      - DATABASE_URL=postgresql://user:pass@postgres:5432/snaprag
```

### Security Considerations

1. **Change Default Passwords**:
   ```yaml
   environment:
     POSTGRES_PASSWORD: your-secure-password
   ```

2. **Use Secrets**:
   ```yaml
   secrets:
     - db_password
   ```

3. **Limit Port Exposure**:
   ```yaml
   # Remove ports if not needed externally
   ports:
     - "127.0.0.1:5432:5432"  # Only local access
   ```

4. **Enable TLS**:
   - Configure PostgreSQL with SSL
   - Use Redis with TLS
   - Put behind reverse proxy (nginx/traefik)

### Resource Limits

```yaml
services:
  snaprag:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G
```

### Health Checks

All services have health checks configured. Check status:

```bash
docker-compose ps
```

## 🌐 Multi-Architecture Build

Build for multiple platforms:

```bash
# Setup buildx
docker buildx create --name multiarch --use

# Build and push
make -f Makefile.docker docker-buildx REGISTRY=ghcr.io/yourusername
```

## 📝 Environment-Specific Configs

### Development

```bash
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up
```

### Production

Create `docker-compose.prod.yml` with production settings:

```yaml
version: '3.8'
services:
  snaprag:
    restart: always
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

Run:
```bash
docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

## 🔗 Integration with Other Services

### Behind Nginx Reverse Proxy

```nginx
server {
    listen 80;
    server_name snaprag.example.com;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### Kubernetes Deployment

Convert docker-compose to Kubernetes:

```bash
# Install kompose
curl -L https://github.com/kubernetes/kompose/releases/download/v1.26.0/kompose-linux-amd64 -o kompose

# Convert
./kompose convert -f docker-compose.yml
```

## 📚 Additional Resources

- [Docker Documentation](https://docs.docker.com/)
- [Docker Compose Reference](https://docs.docker.com/compose/compose-file/)
- [SnapRAG Documentation](./README.md)

## ❓ Support

For issues or questions:
- GitHub Issues: https://github.com/your-org/snaprag/issues
- Docker Hub: https://hub.docker.com/r/your-org/snaprag

