.PHONY: backend frontend migrate docker-up docker-down docker-build setup

BACKEND_DIR := backend
FRONTEND_DIR := frontend

## Run the backend server locally (reads backend/.env, auto-runs migrations on startup)
backend:
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
