# Multi-stage build for optimized image size

# Stage 1: Build
FROM rust:1.90-bookworm as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code and migrations (migrations needed for SQLx compile-time checks)
COPY src ./src
COPY migrations ./migrations

# Build release binary
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/timer /app/timer

# Expose port
EXPOSE 8080

# Run the binary
CMD ["/app/timer"]
