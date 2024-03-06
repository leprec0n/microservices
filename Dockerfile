# Contains multistep process
# Build stage
FROM rust:1.76.0-slim-bookworm as builder
WORKDIR /app
COPY Cargo.toml .
Copy . .
RUN cargo build --release --bin default

# Prod stage
FROM debian:bookworm-20240211-slim
WORKDIR /app
EXPOSE 8080
COPY --from=builder /app/target/release/default /app/default
CMD ["./default"]