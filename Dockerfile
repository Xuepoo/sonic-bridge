# ==========================================
# Multi-stage Dockerfile for SonicBridge
# ==========================================

# --- Stage 1: Build the statically-linked binary ---
FROM rust:alpine AS builder

# Install system compilation dependencies
RUN apk add --no-cache musl-dev gcc make pkgconfig

WORKDIR /usr/src/sonic-bridge
COPY . .

# Statically compile the Rust binary in musl-native alpine
RUN cargo build --release

# --- Stage 2: Minimal runtime image ---
FROM alpine:3.24

RUN apk add --no-cache ca-certificates

# Copy the statically compiled binary
COPY --from=builder /usr/src/sonic-bridge/target/release/sonic-bridge /usr/local/bin/sonic-bridge

# Define XDG directories in container for safety
ENV XDG_CONFIG_HOME=/etc/sonic-bridge
ENV XDG_CACHE_HOME=/var/cache/sonic-bridge

# Expose binary path as entrypoint
ENTRYPOINT ["/usr/local/bin/sonic-bridge"]
