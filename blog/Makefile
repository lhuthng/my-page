.PHONY: backend frontend migrate docker-up docker-down docker-build setup minio-up minio-down backend-local-storage

BACKEND_DIR := backend
FRONTEND_DIR := frontend

MINIO_CONTAINER := blog-minio-dev
MINIO_DATA_DIR := $(BACKEND_DIR)/.minio-data
MINIO_ROOT_USER := minioadmin
MINIO_ROOT_PASSWORD := minioadmin
MINIO_BUCKET := dev-bucket

## Run the backend server locally (reads backend/.env, auto-runs migrations on startup)
backend:
	cd $(BACKEND_DIR) && cargo run --bin backend

## Start a local MinIO container (S3-compatible) for R2 storage in dev. Idempotent:
## reuses/starts the existing container if present, and (re)creates the dev bucket.
## Set R2_ACCOUNT_ID/R2_ACCESS_KEY_ID/R2_SECRET_ACCESS_KEY/R2_BUCKET/R2_ENDPOINT in
## backend/.env to match (see backend/example.env for the local-storage block).
minio-up:
	@mkdir -p $(MINIO_DATA_DIR)
	@docker start $(MINIO_CONTAINER) >/dev/null 2>&1 || docker run -d \
		--name $(MINIO_CONTAINER) \
		-p 9000:9000 -p 9001:9001 \
		-e MINIO_ROOT_USER=$(MINIO_ROOT_USER) \
		-e MINIO_ROOT_PASSWORD=$(MINIO_ROOT_PASSWORD) \
		-v "$(abspath $(MINIO_DATA_DIR)):/data" \
		minio/minio server /data --console-address ":9001" >/dev/null
	@docker run --rm --network container:$(MINIO_CONTAINER) --entrypoint sh minio/mc -c \
		"mc alias set local http://localhost:9000 $(MINIO_ROOT_USER) $(MINIO_ROOT_PASSWORD) >/dev/null && mc mb -p local/$(MINIO_BUCKET) >/dev/null" \
		>/dev/null 2>&1 || true
	@echo "MinIO running: S3 endpoint http://localhost:9000, console http://localhost:9001, bucket '$(MINIO_BUCKET)'"

## Stop the local MinIO container (data persists in backend/.minio-data)
minio-down:
	@docker stop $(MINIO_CONTAINER) >/dev/null 2>&1 || true
	@echo "MinIO stopped"

## Run the backend against local MinIO storage instead of real R2; stops MinIO on exit
backend-local-storage: minio-up
	@trap '$(MAKE) minio-down' EXIT INT TERM; \
	cd $(BACKEND_DIR) && cargo run --bin backend

## Run the frontend dev server locally
frontend:
	cd $(FRONTEND_DIR) && bun run dev

## Run pending migrations (sqlx-cli requires sqlite:// format; override DB_URL for non-default paths)
DB_URL ?= sqlite://data/blog.db


# Code quality
fmt:
	cd backend && cargo fmt
	cd frontend && bun run format

lint:
	cd backend && cargo clippy -- -D warnings
	cd frontend && bun run lint

migrate:
	cd $(BACKEND_DIR) && DATABASE_URL=$(DB_URL) sqlx migrate run

## Start all services via Docker Compose
docker-up:
	docker compose up -d

## Stop all Docker Compose services
docker-down:
	docker compose down

## Rebuild Docker images locally
docker-build:
	docker compose build

## One-time setup: create data dir and copy example .env files if missing
setup:
	@mkdir -p $(BACKEND_DIR)/data
	@[ -f $(BACKEND_DIR)/.env ] || (cp $(BACKEND_DIR)/example.env $(BACKEND_DIR)/.env && echo "Created backend/.env from example.env")
	@[ -f $(FRONTEND_DIR)/.env ] || (cp $(FRONTEND_DIR)/example.env $(FRONTEND_DIR)/.env && echo "Created frontend/.env from example.env")
	@echo "Setup complete. Edit backend/.env and frontend/.env before running."
