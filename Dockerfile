# ── Stage 1: Build ─────────────────────────────────────────
FROM rust:1.83-slim AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
COPY static/ static/

RUN cargo build --release

# ── Stage 2: Runtime ───────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    useradd -m -u 1000 appuser

COPY --from=builder /app/target/release/polaris /usr/local/bin/polaris
RUN chmod +x /usr/local/bin/polaris

USER appuser
EXPOSE 7860

CMD ["polaris", "server", "--host", "0.0.0.0", "--port", "7860"]
