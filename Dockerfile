# Build stage
FROM rust:1.93 AS builder
WORKDIR /app

# Cache dependencies first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy real source and build
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app

COPY --from=builder /app/target/release/galactic-exchange /usr/local/bin/galactic-exchange

CMD ["galactic-exchange"]
