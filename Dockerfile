# Build stage
FROM rust:1.93 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM  debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/vmbid ./
EXPOSE 8080
CMD ["./vmbid"]
