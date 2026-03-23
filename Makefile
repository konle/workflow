.PHONY: dev dev-down run-api run-engine build up down logs clean check

# ── Local Development ────────────────────────────────────────────
# Start MongoDB + Redis containers for local dev
dev:
	docker compose -f deploy/docker-compose.dev.yml up -d

dev-down:
	docker compose -f deploy/docker-compose.dev.yml down

# Run Rust services natively (requires `make dev` first)
run-api:
	MONGO_URL=mongodb://127.0.0.1:27017 REDIS_URL=redis://127.0.0.1:6379 \
	cargo run --bin apiserver

run-engine:
	MONGO_URL=mongodb://127.0.0.1:27017 REDIS_URL=redis://127.0.0.1:6379 \
	cargo run --bin engine

# ── Full Deployment ──────────────────────────────────────────────
build:
	docker compose -f deploy/docker-compose.yml build

up:
	docker compose -f deploy/docker-compose.yml up -d

down:
	docker compose -f deploy/docker-compose.yml down

logs:
	docker compose -f deploy/docker-compose.yml logs -f

# ── Utilities ────────────────────────────────────────────────────
check:
	cargo check

clean:
	cargo clean
	docker compose -f deploy/docker-compose.dev.yml down -v
	docker compose -f deploy/docker-compose.yml down -v
