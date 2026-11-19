# ============================================================================
# STAGE 1: BUILDER
# ============================================================================
FROM rust:1.82-bookworm AS builder
WORKDIR /app

COPY Cargo.toml ./
COPY crates ./crates

RUN cargo build --release -p server-rust -p agent-rust

# ============================================================================
# STAGE 2: RUNTIME (Non-Root User sesuai standar production-ready-coder)
# ============================================================================
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10001 appuser

WORKDIR /app
RUN mkdir -p /app/downloads /app/downloads-builtin
COPY --from=builder /app/target/release/cmk-server /usr/local/bin/cmk-server
COPY --from=builder /app/target/release/cmk-agent /app/downloads-builtin/bms-agent-linux-x86_64

USER appuser
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://127.0.0.1:8080/healthz || exit 1

ENTRYPOINT ["/usr/local/bin/cmk-server"]
