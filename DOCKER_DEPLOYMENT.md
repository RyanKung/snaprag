# 🐳 SnapRAG Docker Deployment Guide

Simple Docker deployment for SnapRAG. Connects to your existing PostgreSQL and Redis.

## 📋 Prerequisites

- Docker 20.10+
- PostgreSQL 15+ with pgvector extension (running separately)
- Redis 7+ (optional, for caching)
- 2GB RAM minimum for SnapRAG container
- config.toml file configured for your environment

## 🚀 Quick Start

```bash
# 1. Create config.toml
make -f Makefile.docker setup-config

# 2. Edit config.toml with your database and Redis URLs
vim config.toml

# 3. Build image
make -f Makefile.docker docker-build

# 4. Test image
make -f Makefile.docker docker-test

# 5. Run container
make -f Makefile.docker docker-run
```

Access API at: http://localhost:3000

## 📦 What's Included

The Docker image contains:
- ✅ SnapRAG binary (compiled from source)
- ✅ AFINN lexicon and data files
- ✅ Database migrations
- ✅ config.example.toml (reference only)

**NOT included (by design):**
- ❌ config.toml - must be mounted from host
- ❌ PostgreSQL - use external database
- ❌ Redis - use external Redis instance

## 🔧 Configuration

### config.toml Location

**IMPORTANT:** `config.toml` is **NOT** included in the image. You must mount it:

```bash
docker run -v ./config.toml:/app/config.toml:ro snaprag
```

This provides:
- **Security**: No secrets baked into image
- **Flexibility**: Update config without rebuilding
- **Best Practice**: 12-Factor App compliance

### Database Configuration

Update `config.toml`:

```toml
[database]
# Use host.docker.internal to access services on host machine
url = "postgresql://snaprag:password@host.docker.internal:5432/snaprag"

# Or use external server
url = "postgresql://user:pass@db.example.com:5432/snaprag"
```

### Redis Configuration (Optional)

```toml
[cache]
enabled = true
redis_url = "redis://host.docker.internal:6379"
```

## 🛠️ Makefile Commands

All operations via Makefile.docker:

### Setup & Build

```bash
# Create config.toml from example
make -f Makefile.docker setup-config

# Build Docker image
make -f Makefile.docker docker-build

# Test the image
make -f Makefile.docker docker-test
```

### Run Containers

```bash
# Run API server
make -f Makefile.docker docker-run

# Run sync process
make -f Makefile.docker docker-run-sync

# Stop containers
make -f Makefile.docker docker-stop
```

### Manage

```bash
# View logs
make -f Makefile.docker docker-logs

# Check status
make -f Makefile.docker docker-status

# Open shell
make -f Makefile.docker docker-shell

# Clean up
make -f Makefile.docker docker-clean
```

### Registry Operations

```bash
# Push to registry
make -f Makefile.docker docker-push REGISTRY=ghcr.io/yourname

# Multi-arch build
make -f Makefile.docker docker-buildx REGISTRY=ghcr.io/yourname
```

## 🔍 Common Operations

### Run Commands in Container

```bash
# Check version
docker exec snaprag snaprag --version

# Show configuration
docker exec snaprag snaprag config

# List FIDs
docker exec snaprag snaprag list fid --limit 10

# Sync status
docker exec snaprag snaprag sync status
```

### Update Configuration

```bash
# 1. Edit config.toml on host
vim config.toml

# 2. Restart container
docker restart snaprag

# No rebuild needed!
```

### Access Logs

```bash
# Container logs
docker logs -f snaprag

# Application logs (if mounted)
tail -f logs/snaprag.log
```

## 🐛 Troubleshooting

### Container Won't Start

```bash
# Check logs
docker logs snaprag

# Verify config.toml exists
ls -la config.toml

# Test with shell
docker run -it --rm \
  -v $(pwd)/config.toml:/app/config.toml:ro \
  snaprag:latest /bin/bash
```

### Database Connection Issues

```bash
# Test database connectivity from container
docker run --rm \
  --add-host=host.docker.internal:host-gateway \
  snaprag:latest \
  /bin/bash -c "apt-get update && apt-get install -y postgresql-client && psql $DATABASE_URL -c 'SELECT 1'"

# Or use config.toml
docker run --rm \
  -v $(pwd)/config.toml:/app/config.toml:ro \
  snaprag:latest config
```

### Can't Access Host Services

Use `host.docker.internal` in config.toml:

```toml
[database]
url = "postgresql://user:pass@host.docker.internal:5432/snaprag"

[sync]
snapchain_http_endpoint = "http://host.docker.internal:3381"
snapchain_grpc_endpoint = "http://host.docker.internal:3383"
```

On Linux, add `--add-host=host.docker.internal:host-gateway` to docker run.

## 🚢 Production Deployment

### Build Production Image

```bash
# Build with specific tag
make -f Makefile.docker docker-build IMAGE_TAG=v0.1.0

# Tag as latest
docker tag snaprag:v0.1.0 snaprag:latest
```

### Run with Systemd

Create `/etc/systemd/system/snaprag.service`:

```ini
[Unit]
Description=SnapRAG Container
After=docker.service postgresql.service redis.service
Requires=docker.service

[Service]
Type=simple
ExecStartPre=-/usr/bin/docker stop snaprag
ExecStartPre=-/usr/bin/docker rm snaprag
ExecStart=/usr/bin/docker run --rm \
  --name snaprag \
  -p 3000:3000 \
  -v /opt/snaprag/config.toml:/app/config.toml:ro \
  -v /opt/snaprag/logs:/app/logs \
  --add-host=host.docker.internal:host-gateway \
  snaprag:latest api
ExecStop=/usr/bin/docker stop snaprag
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable snaprag
sudo systemctl start snaprag
sudo systemctl status snaprag
```

### Environment Variables

Pass config via environment variables:

```bash
docker run -d \
  --name snaprag \
  -p 3000:3000 \
  -e DATABASE_URL="postgresql://user:pass@host:5432/db" \
  -e RUST_LOG=info \
  snaprag:latest api
```

### Security Considerations

1. **Don't include secrets in image**
   - ✅ Mount config.toml at runtime
   - ✅ Use environment variables
   - ✅ Use Docker secrets in swarm mode

2. **Run as non-root**
   - ✅ Image already uses `snaprag` user (UID 1000)

3. **Limit resources**
   ```bash
   docker run -d \
     --name snaprag \
     --memory=2g \
     --cpus=2 \
     -v ./config.toml:/app/config.toml:ro \
     snaprag:latest api
   ```

4. **Use read-only config**
   - Always mount config with `:ro` flag

## 🌐 Multi-Architecture Build

Build for both amd64 and arm64:

```bash
# Setup buildx
docker buildx create --name multiarch --use

# Build and push
make -f Makefile.docker docker-buildx REGISTRY=ghcr.io/yourname
```

## 🔗 Integration Examples

### With Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: snaprag
spec:
  replicas: 2
  template:
    spec:
      containers:
      - name: snaprag
        image: snaprag:latest
        ports:
        - containerPort: 3000
        volumeMounts:
        - name: config
          mountPath: /app/config.toml
          subPath: config.toml
          readOnly: true
        - name: logs
          mountPath: /app/logs
      volumes:
      - name: config
        configMap:
          name: snaprag-config
      - name: logs
        emptyDir: {}
```

### With Docker Swarm

```bash
# Create config secret
docker secret create snaprag-config config.toml

# Deploy
docker service create \
  --name snaprag \
  --replicas 3 \
  --publish 3000:3000 \
  --secret source=snaprag-config,target=/app/config.toml \
  snaprag:latest api
```

### Behind Nginx

```nginx
upstream snaprag {
    server localhost:3000;
}

server {
    listen 80;
    server_name api.example.com;

    location / {
        proxy_pass http://snaprag;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## 📊 Resource Usage

Expected resource usage:

- **Memory**: 500MB - 2GB (depends on database size and cache)
- **CPU**: 1-2 cores (more during sync)
- **Disk**: ~100MB for image, varies for logs
- **Network**: Depends on sync load

## 📚 Additional Resources

- [Dockerfile Reference](https://docs.docker.com/engine/reference/builder/)
- [Docker Run Reference](https://docs.docker.com/engine/reference/run/)
- [SnapRAG Documentation](./README.md)

## ❓ FAQ

**Q: Why isn't PostgreSQL included?**  
A: SnapRAG requires persistent database storage. It's better to manage PostgreSQL separately for data safety and scaling.

**Q: Can I use Docker Compose?**  
A: Not provided by default. For simplicity, we focus on single-container deployment. You can create your own compose file if needed.

**Q: How do I update config.toml?**  
A: Edit the file on your host, then restart: `docker restart snaprag`

**Q: How do I access the host's PostgreSQL from container?**  
A: Use `host.docker.internal` in your config.toml database URL.

**Q: Can I run multiple instances?**  
A: Yes! Just use different names and ports:
```bash
docker run -d --name snaprag-1 -p 3001:3000 ...
docker run -d --name snaprag-2 -p 3002:3000 ...
```
