FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin backend

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/backend ./backend
EXPOSE 3000
CMD ["./backend"]
