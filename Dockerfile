# ============================================================================
# SnapRAG Docker Image - Multi-stage build for optimal size
# ============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Builder - Build the Rust binary
# -----------------------------------------------------------------------------
FROM rust:1.83-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml ./

# Copy source code
COPY src ./src
COPY proto ./proto
COPY build.rs ./
COPY data ./data
COPY migrations ./migrations

# Build for release with SQLx offline mode (no database required during build)
ENV SQLX_OFFLINE=true
RUN cargo build --release

# -----------------------------------------------------------------------------
# Stage 2: Runtime - Create minimal runtime image
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 snaprag

# Create necessary directories
RUN mkdir -p /app/logs /app/data && chown -R snaprag:snaprag /app

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/snaprag /usr/local/bin/snaprag

# Copy data files
COPY --from=builder /app/data ./data
COPY --from=builder /app/migrations ./migrations

# Copy example config (will be overridden by volume mount in production)
COPY config.example.toml /app/config.example.toml

# Switch to non-root user
USER snaprag

# Expose API port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD snaprag --version || exit 1

# Default command
ENTRYPOINT ["snaprag"]
CMD ["--help"]

