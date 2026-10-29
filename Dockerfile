# ==============================================================================
# RELIC — Contenedor de Producción Endurecido (Distroless Nonroot)
# Multi-Stage Build con sistema de archivos inmutable y cero intermediación shell
# ==============================================================================

# ------------------------------------------------------------------------------
# Etapa 1: Compilación optimizada en entorno reproducible
# ------------------------------------------------------------------------------
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    binutils \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 1. Copia de manifiestos del workspace para resolución determinista de capas
COPY Cargo.toml Cargo.lock ./
COPY crates/relic_core/Cargo.toml crates/relic_core/
COPY crates/relic_lexicon/Cargo.toml crates/relic_lexicon/
COPY crates/relic_blackwall/Cargo.toml crates/relic_blackwall/
COPY crates/relic_tokenizer/Cargo.toml crates/relic_tokenizer/
COPY crates/relic_cli/Cargo.toml crates/relic_cli/
COPY crates/relic_server/Cargo.toml crates/relic_server/
COPY crates/relic_mcp/Cargo.toml crates/relic_mcp/
COPY benchmarks/Cargo.toml benchmarks/

# 2. Copia del código fuente de producción
COPY crates/ crates/
COPY benchmarks/ benchmarks/

# 3. Compilación estricta del binario relic_server en modo release
RUN cargo build --release --package relic_server --bin relic_server

# 4. Eliminación de símbolos de depuración para minimizar tamaño y superficie
RUN strip /app/target/release/relic_server

# ------------------------------------------------------------------------------
# Etapa 2: Runtime Distroless ultra-ligero y sin privilegios
# ------------------------------------------------------------------------------
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /relic

LABEL org.opencontainers.image.title="RELIC Ingestion and Inference Engine"
LABEL org.opencontainers.image.description="Deterministic Neuro-Symbolic Engine with SHA-256 Engram"
LABEL org.opencontainers.image.authors="dev-nikolas / RELIC Team"
LABEL org.opencontainers.image.licenses="AGPL-3.0-only"

# Copiar exclusivamente el binario sellado
COPY --from=builder /app/target/release/relic_server /usr/local/bin/relic_server

# Puerto de escucha del servicio HTTP de RELIC
EXPOSE 5000

# Usuario sin privilegios por defecto (UID 65532 / GID 65532)
USER nonroot:nonroot

ENV PORT=5000
ENV RUST_BACKTRACE=0

# Punto de entrada directo (sin shell, inmune a inyección de scripts)
ENTRYPOINT ["/usr/local/bin/relic_server"]

