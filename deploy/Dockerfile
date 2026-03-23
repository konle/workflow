FROM rust:1.85-bookworm AS builder

WORKDIR /app
COPY . .

RUN cargo build --release --bin apiserver --bin engine

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/apiserver /usr/local/bin/apiserver
COPY --from=builder /app/target/release/engine /usr/local/bin/engine

ENV MONGO_URL=mongodb://mongodb:27017
ENV REDIS_URL=redis://redis:6379

ENTRYPOINT ["apiserver"]
